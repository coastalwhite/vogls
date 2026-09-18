//! Lowering of `Drive` and `DriveSlice`: write a value into a signal, then wake what watched it.
//!
//! Both instructions go through one emitter. A `Drive` knows the bit it starts at when compiling; a
//! `DriveSlice` only learns it when running, and it may point anywhere at all -- a negative index
//! wraps around to a huge one -- so its write is trimmed to the signal's bounds by *masking*, never
//! by branching. The value keeps its position; the bits that would land outside the bounds are
//! cleared from both the value and the clear mask; and word addresses are clamped into the signal,
//! so a far-out index reads an in-range word and writes it back unchanged. That keeps the hot path
//! one straight line, with nothing live across a call and nothing to spill, and it leaves the
//! changed-bits mask in the value's own coordinates, which is what everything downstream wants.
//!
//! The order of what happens after the write is the same for both: edge wakes, the plugin poke and
//! last-update time, then the level-sensitive wakes.

use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{InstBuilder, Value};
use cranelift_frontend::FunctionBuilder;
use vogls_codegen::{HeapAlignment, SixBitSize};
use vogls_ir::{
    LogicMode, SignalKey, SignalSlice, VSIZE_64, VariableKey, VectorSize, WatchCondition, WatchEdge,
};

use super::tr::{
    TrBuilder, clif_conditional_wake_listeners, clif_fv_negedge, clif_wide_tv_reduce_or,
    clif_wide_tv_slice_reduce_or,
};
use super::{I64, PLUGIN_STATE_SIZE, mask_of, mem, wide_load};
use crate::runtime::layout;

/// Where a drive starts, in bits from the start of the signal.
#[derive(Clone, Copy)]
pub(super) enum DriveOffset {
    /// Known when compiling, and in range.
    Imm(u32),
    /// Only known when running. `bit` may be anything, including past the signal or wrapped around
    /// from a negative index. `known` is false when the index held `x`/`z` bits, in which case
    /// nothing is written at all.
    Dyn {
        bit: Value,
        known: Option<Value>,
        /// Whether `bit` plus a width can wrap around: only a 64-bit index can get that far.
        may_wrap: bool,
    },
}

/// The bits of a signal a dynamic write may touch, inclusive, in signal-relative bits.
///
/// Today every site bounds a write by the whole signal, but the bytecode's clamp range is a
/// separate thing from the signal (for an array element it starts at the element), so the emitter
/// takes it as data rather than deriving it.
#[derive(Clone, Copy)]
struct Bounds {
    lower: u64,
    upper: u64,
}

/// A shift amount in `0..64`, folded to an immediate whenever it is known.
#[derive(Clone, Copy)]
enum Amt {
    Imm(u32),
    Dyn(Value),
}

impl Amt {
    fn shl(self, b: &mut FunctionBuilder, v: Value) -> Value {
        match self {
            Amt::Imm(0) => v,
            Amt::Imm(n) => b.ins().ishl_imm_u(v, n as i64),
            Amt::Dyn(a) => b.ins().ishl(v, a),
        }
    }

    fn ushr(self, b: &mut FunctionBuilder, v: Value) -> Value {
        match self {
            Amt::Imm(0) => v,
            Amt::Imm(n) => b.ins().ushr_imm_u(v, n as i64),
            Amt::Dyn(a) => b.ins().ushr(v, a),
        }
    }

    /// `v << (64 - amt)`: the part of `v` that spills into the next word.
    ///
    /// Zero for an amount of zero, which a plain shift gets wrong because Cranelift masks the
    /// amount to six bits; the dynamic form shifts by `63 - amt` and then once more.
    fn shl_inv(self, b: &mut FunctionBuilder, v: Value) -> Value {
        match self {
            Amt::Imm(0) => b.ins().iconst(I64, 0),
            Amt::Imm(n) => b.ins().ishl_imm_u(v, (64 - n) as i64),
            Amt::Dyn(a) => {
                let sixty_three = b.ins().iconst(I64, 63);
                let inv = b.ins().isub(sixty_three, a);
                let s = b.ins().ishl(v, inv);
                b.ins().ishl_imm_u(s, 1)
            }
        }
    }

    /// `v >> (64 - amt)`, with the same care as [`Amt::shl_inv`].
    fn ushr_inv(self, b: &mut FunctionBuilder, v: Value) -> Value {
        match self {
            Amt::Imm(0) => b.ins().iconst(I64, 0),
            Amt::Imm(n) => b.ins().ushr_imm_u(v, (64 - n) as i64),
            Amt::Dyn(a) => {
                let sixty_three = b.ins().iconst(I64, 63);
                let inv = b.ins().isub(sixty_three, a);
                let s = b.ins().ushr(v, inv);
                b.ins().ushr_imm_u(s, 1)
            }
        }
    }

    /// This amount plus a constant, for a plane that sits a fixed distance further along.
    fn plus(self, b: &mut FunctionBuilder, n: u32) -> Amt {
        match self {
            Amt::Imm(m) => Amt::Imm(m + n),
            Amt::Dyn(a) if n == 0 => Amt::Dyn(a),
            Amt::Dyn(a) => Amt::Dyn(b.ins().iadd_imm_u(a, n as i64)),
        }
    }
}

/// A bit mask, folded to an immediate whenever it is known.
#[derive(Clone, Copy)]
enum Mask {
    Imm(u64),
    Dyn(Value),
}

impl Mask {
    fn value(self, b: &mut FunctionBuilder) -> Value {
        match self {
            Mask::Imm(m) => b.ins().iconst(I64, m as i64),
            Mask::Dyn(v) => v,
        }
    }

    fn and(self, b: &mut FunctionBuilder, v: Value) -> Value {
        match self {
            Mask::Imm(u64::MAX) => v,
            Mask::Imm(m) => b.ins().band_imm_u(v, m as i64),
            Mask::Dyn(m) => b.ins().band(v, m),
        }
    }

    /// `v & !self`.
    fn clear(self, b: &mut FunctionBuilder, v: Value) -> Value {
        match self {
            Mask::Imm(0) => v,
            Mask::Imm(u64::MAX) => b.ins().iconst(I64, 0),
            Mask::Imm(m) => b.ins().band_imm_u(v, !m as i64),
            Mask::Dyn(m) => b.ins().band_not(v, m),
        }
    }

    fn shl(self, b: &mut FunctionBuilder, amt: Amt) -> Mask {
        match (self, amt) {
            (Mask::Imm(m), Amt::Imm(n)) => Mask::Imm(m << n),
            (mask, amt) => {
                let v = mask.value(b);
                Mask::Dyn(amt.shl(b, v))
            }
        }
    }

    fn ushr_inv(self, b: &mut FunctionBuilder, amt: Amt) -> Mask {
        match (self, amt) {
            (Mask::Imm(_), Amt::Imm(0)) => Mask::Imm(0),
            (Mask::Imm(m), Amt::Imm(n)) => Mask::Imm(m >> (64 - n)),
            (mask, amt) => {
                let v = mask.value(b);
                Mask::Dyn(amt.ushr_inv(b, v))
            }
        }
    }
}

/// One machine word of the heap: a pointer and a fixed byte offset from it.
#[derive(Clone, Copy)]
struct WordRef {
    ptr: Value,
    off: i32,
}

impl WordRef {
    fn load(self, b: &mut FunctionBuilder) -> Value {
        b.ins().load(I64, mem(), self.ptr, self.off)
    }

    fn store(self, b: &mut FunctionBuilder, v: Value) {
        b.ins().store(mem(), v, self.ptr, self.off);
    }

    fn add(self, bytes: i32) -> WordRef {
        WordRef {
            ptr: self.ptr,
            off: self.off + bytes,
        }
    }
}

/// Rewrite the field at bit `boff` of `w0` -- continuing into `w1` where the field crosses into
/// the next word -- with `value`. `value` holds no bits outside `trim`, and only the bits of the
/// field under `trim` move; everything else in both words is kept.
///
/// Returns the new words and the bits of `value` that differ from what the field held, in the
/// value's own bit positions.
fn insert_field(
    b: &mut FunctionBuilder,
    w0: Value,
    w1: Option<Value>,
    value: Value,
    trim: Mask,
    boff: Amt,
) -> (Value, Option<Value>, Value) {
    // A full-width write of a whole word needs no read-modify-write at all.
    if let (Mask::Imm(u64::MAX), Amt::Imm(0)) = (trim, boff) {
        let changed = b.ins().bxor(w0, value);
        return (value, w1, changed);
    }

    let mut old = boff.ushr(b, w0);
    if let Some(w1) = w1 {
        let hi = boff.shl_inv(b, w1);
        old = b.ins().bor(old, hi);
    }
    let old = trim.and(b, old);
    let changed = b.ins().bxor(old, value);

    let kept = trim.shl(b, boff).clear(b, w0);
    let put = boff.shl(b, value);
    let n0 = b.ins().bor(kept, put);

    let n1 = w1.map(|w1| {
        let kept = trim.ushr_inv(b, boff).clear(b, w1);
        let put = boff.ushr_inv(b, value);
        b.ins().bor(kept, put)
    });

    (n0, n1, changed)
}

/// `a - min(a, bound)`: how far `a` lies above `bound`, or zero.
fn above(b: &mut FunctionBuilder, a: Value, bound: u64) -> Value {
    let bound = b.ins().iconst(I64, bound as i64);
    let m = b.ins().umin(a, bound);
    b.ins().isub(a, m)
}

/// `bound - min(bound, a)`: how far `a` lies below `bound`, or zero.
fn below(b: &mut FunctionBuilder, bound: u64, a: Value) -> Value {
    let bound = b.ins().iconst(I64, bound as i64);
    let m = b.ins().umin(bound, a);
    b.ins().isub(bound, m)
}

/// The bits of a `width`-bit value written at `off` that fall inside `bounds`, as a mask in the
/// value's bit positions. Any offset at all is fine here, including one wrapped around from a
/// negative index: such a write lands entirely outside and the mask comes out zero.
fn window_trim(
    b: &mut FunctionBuilder,
    off: Value,
    width: u32,
    bounds: Bounds,
    may_wrap: bool,
) -> Value {
    let width_mask = mask_of(width) as u64;

    // Bits above the upper bound: `width_mask >> cut`, where a cut of 64 or more clears it all.
    // Cranelift masks the shift amount, so that last case has to be spelled out -- but only where
    // the mask has a bit 63 that a shift of 63 would leave behind.
    let mut end = if width == 1 {
        off
    } else {
        b.ins().iadd_imm_u(off, (width - 1) as i64)
    };
    if may_wrap {
        end = b.ins().umax(end, off);
    }
    let cut = above(b, end, bounds.upper);
    let sixty_four = b.ins().iconst(I64, 64);
    let wm = b.ins().iconst(I64, width_mask as i64);
    let mut trim;
    if width == 64 {
        let cut = b.ins().umin(cut, sixty_four);
        trim = b.ins().ushr(wm, cut);
        let all = b.ins().icmp(IntCC::Equal, cut, sixty_four);
        let zero = b.ins().iconst(I64, 0);
        trim = b.ins().select(all, zero, trim);
    } else {
        // A shift of 63 already clears a mask without bit 63, so clamping there is enough.
        let sixty_three = b.ins().iconst(I64, 63);
        let cut = b.ins().umin(cut, sixty_three);
        trim = b.ins().ushr(wm, cut);
    }

    // Bits below the lower bound, the same way around.
    if bounds.lower > 0 {
        let cut = below(b, bounds.lower, off);
        let cut = b.ins().umin(cut, sixty_four);
        let all_ones = b.ins().iconst(I64, -1);
        let m = b.ins().ishl(all_ones, cut);
        let all = b.ins().icmp(IntCC::Equal, cut, sixty_four);
        let zero = b.ins().iconst(I64, 0);
        let m = b.ins().select(all, zero, m);
        trim = b.ins().band(trim, m);
    }

    trim
}

/// One bit of the heap at a static bit address, as 0 or 1.
fn load_bit(b: &mut FunctionBuilder, base_ptr: Value, bit: u64) -> Value {
    let word = b.ins().load(I64, mem(), base_ptr, ((bit / 64) * 8) as i32);
    bit_of_word(b, word, bit % 64)
}

fn bit_of_word(b: &mut FunctionBuilder, word: Value, bit: u64) -> Value {
    let mut v = word;
    if bit != 0 {
        v = b.ins().ushr_imm_u(v, bit as i64);
    }
    if bit < 63 {
        v = b.ins().band_imm_u(v, 1);
    }
    v
}

/// Two bits of the heap at static bit addresses, sharing the load when they are in one word.
fn load_bit_pair(b: &mut FunctionBuilder, base_ptr: Value, fst: u64, snd: u64) -> (Value, Value) {
    if fst / 64 == snd / 64 {
        let word = b.ins().load(I64, mem(), base_ptr, ((fst / 64) * 8) as i32);
        (
            bit_of_word(b, word, fst % 64),
            bit_of_word(b, word, snd % 64),
        )
    } else {
        (load_bit(b, base_ptr, fst), load_bit(b, base_ptr, snd))
    }
}

/// Where a drive lands, described once so every step can ask it the same questions.
#[derive(Clone, Copy)]
struct Site {
    signal_size: VectorSize,
    /// The signal's first bit in the heap.
    base_bit: usize,
    /// Distance from a bit's `spc` plane to its `val` plane, in bits. Meaningless for two-valued.
    plane_bits: u64,
    src_size: VectorSize,
    mode: LogicMode,
    offset: DriveOffset,
    bounds: Bounds,
    /// Whether `offset + src_size` might wrap around a 64-bit offset.
    may_wrap: bool,
}

impl Site {
    /// The static window this drive writes, when it has one.
    fn window(self) -> Option<SignalSlice> {
        match self.offset {
            DriveOffset::Imm(off) => SignalSlice::from_width(off, self.src_size),
            DriveOffset::Dyn { .. } => None,
        }
    }

    fn is_wide_signal(self) -> bool {
        SixBitSize::from_vector_size(self.signal_size).is_none()
    }

    fn signal_words(self) -> u32 {
        self.signal_size.get().div_ceil(64)
    }

    fn upper_word(self) -> u64 {
        self.bounds.upper / 64
    }
}

/// The old level of a watched bit, taken before the write.
struct EdgeSite {
    index: usize,
    condition: WatchCondition,
    old_val: Value,
    old_spc: Option<Value>,
}

impl<'a, 'b> TrBuilder<'a, 'b> {
    /// `dst = drive signal[offset..offset + |src|] <- src`, waking every watch the write reached.
    pub(super) fn emit_drive(
        &mut self,
        dst: VariableKey,
        signal: SignalKey,
        src: VariableKey,
        offset: DriveOffset,
    ) {
        let gl = self.compiler.gl;
        let src_size = gl.vars.size(src);
        let signal_size = gl.signals[signal].size;
        let (heap_ref, rt, _mode) = self.compiler.info.heap_ref(signal);
        let mode = src.mode();

        let may_wrap = match offset {
            DriveOffset::Imm(_) => false,
            DriveOffset::Dyn { may_wrap, .. } => may_wrap,
        };
        let site = Site {
            signal_size,
            base_bit: heap_ref.offset.bit_offset,
            plane_bits: HeapAlignment::spc_offset_to_val_offset(signal_size, 0),
            src_size,
            mode,
            offset,
            bounds: Bounds {
                lower: 0,
                upper: signal_size.get() as u64 - 1,
            },
            may_wrap,
        };

        let conditions = self
            .compiler
            .watch_map
            .map()
            .get(&signal)
            .cloned()
            .unwrap_or_default();

        // Edge watches need the level each watched bit held before the write. With a static
        // window the ones it cannot reach are skipped outright; with a dynamic one every watched
        // bit is read, and whether the write reached it falls out of comparing levels afterwards.
        let mut edges = Vec::new();
        let mut prev_condition = None;
        for index in conditions.clone() {
            let (condition, _) = self.compiler.watch_map.watchers()[index];
            if Some(condition) == prev_condition || !condition.edge.is_directed() {
                prev_condition = Some(condition);
                continue;
            }
            prev_condition = Some(condition);
            if let Some(window) = site.window()
                && (condition.offset < window.lsb() || condition.offset > window.msb())
            {
                continue;
            }
            let (old_val, old_spc) = self.watched_levels(site, condition.offset);
            edges.push(EdgeSite {
                index,
                condition,
                old_val,
                old_spc,
            });
        }

        // The write itself, and which bits of the value it moved.
        let mask = self.write(site, dst, src);
        if src_size <= VSIZE_64 {
            let b = &mut self.b;
            b.def_var(self.vmap[&dst], mask);
        }

        for edge in edges {
            let (new_val, new_spc) = match site.offset {
                DriveOffset::Imm(off) => self.src_levels(site, src, edge.condition.offset - off),
                DriveOffset::Dyn { .. } => self.watched_levels(site, edge.condition.offset),
            };
            let b = &mut self.b;
            let cond = match (edge.old_spc, new_spc) {
                (None, None) => {
                    if edge.condition.edge == WatchEdge::Posedge {
                        b.ins().band_not(new_val, edge.old_val)
                    } else {
                        b.ins().band_not(edge.old_val, new_val)
                    }
                }
                (Some(old_spc), Some(new_spc)) => {
                    if edge.condition.edge == WatchEdge::Posedge {
                        clif_fv_negedge(b, new_val, new_spc, edge.old_val, old_spc)
                    } else {
                        clif_fv_negedge(b, edge.old_val, old_spc, new_val, new_spc)
                    }
                }
                _ => unreachable!("old and new levels come from the same signal"),
            };
            let mut index = edge.index;
            clif_conditional_wake_listeners(self.compiler, &self.params, b, cond, &mut index);
        }

        // The poke and the last-update time, only if something moved.
        let num_plugins = self.compiler.num_plugins;
        let lupdt = self.compiler.info.lupdt_indexes.get(&rt).copied();
        if num_plugins > 0 || lupdt.is_some() {
            let b = &mut self.b;
            let cond = if src_size <= VSIZE_64 {
                mask
            } else {
                clif_wide_tv_reduce_or(b, self.compiler.ptr, mask, src_size)
            };

            let poke_bb = b.create_block();
            let post_bb = b.create_block();
            b.ins().brif(cond, poke_bb, &[], post_bb, &[]);
            b.switch_to_block(poke_bb);

            if num_plugins > 0 {
                let ptr = self.compiler.ptr;
                let cldctx = self.params.cldctx;
                let plugins = b.ins().load(ptr, mem(), cldctx, layout::CTX_PLUGINS as i32);
                let poke = b
                    .ins()
                    .load(ptr, mem(), cldctx, layout::CTX_PLUGIN_POKE as i32);
                let sig_ref = b.import_signature(self.compiler.sigs.plugin_poke.clone());
                let id = b.ins().iconst(I64, rt.as_u64() as i64);
                for i in 0..num_plugins {
                    let pl = b.ins().iadd_imm_u(plugins, (i * PLUGIN_STATE_SIZE) as i64);
                    b.ins().call_indirect(sig_ref, poke, &[pl, id]);
                }
            }

            if let Some(li) = lupdt {
                b.ins().store(
                    mem(),
                    self.params.time,
                    self.params.last_active_time,
                    (li * 8) as i32,
                );
            }

            b.ins().jump(post_bb, &[]);
            b.switch_to_block(post_bb);
        }

        // Level-sensitive watches wake on any moved bit inside their slice.
        let mut index = conditions.start;
        while let Some((condition, _)) = self.compiler.watch_map.watchers().get(index)
            && condition.signal == signal
        {
            let WatchEdge::Any { .. } = condition.edge else {
                index += 1;
                continue;
            };
            let Some(cond) = self.moved_within(site, mask, condition.slice()) else {
                index += 1;
                continue;
            };
            clif_conditional_wake_listeners(
                self.compiler,
                &self.params,
                &mut self.b,
                cond,
                &mut index,
            );
        }
    }

    /// The level bit `bit` of the signal holds right now, as `(val, spc)`.
    fn watched_levels(&mut self, site: Site, bit: u32) -> (Value, Option<Value>) {
        let b = &mut self.b;
        let heap_ptr = self.params.heap_ptr;
        let spc_bit = site.base_bit as u64 + bit as u64;
        match site.mode {
            LogicMode::TwoValue => (load_bit(b, heap_ptr, spc_bit), None),
            LogicMode::FourValue => {
                let val_bit = spc_bit + site.plane_bits;
                let (spc, val) = load_bit_pair(b, heap_ptr, spc_bit, val_bit);
                (val, Some(spc))
            }
        }
    }

    /// The level bit `bit` of `src` carries, as `(val, spc)`.
    fn src_levels(&mut self, site: Site, src: VariableKey, bit: u32) -> (Value, Option<Value>) {
        let b = &mut self.b;
        let ptr = self.compiler.ptr;
        let cldctx = self.params.cldctx;
        match SixBitSize::from_vector_size(site.src_size) {
            None => {
                let src_ptr = self.wide_map[&src].addr(b, ptr, cldctx, 0);
                let val_bit = bit as u64;
                match site.mode {
                    LogicMode::TwoValue => (load_bit(b, src_ptr, val_bit), None),
                    LogicMode::FourValue => {
                        let spc_bit = val_bit;
                        let val_bit =
                            HeapAlignment::spc_offset_to_val_offset(site.src_size, spc_bit);
                        let (spc, val) = load_bit_pair(b, src_ptr, spc_bit, val_bit);
                        (val, Some(spc))
                    }
                }
            }
            Some(size) => {
                let extract = |b: &mut FunctionBuilder, v: Value| {
                    let mut v = v;
                    if bit > 0 {
                        v = b.ins().ushr_imm_u(v, bit as i64);
                    }
                    if bit < size as u32 - 1 {
                        v = b.ins().band_imm_u(v, 1);
                    }
                    v
                };
                let val = b.use_var(self.vmap[&src]);
                let val = extract(b, val);
                let spc = match site.mode {
                    LogicMode::TwoValue => None,
                    LogicMode::FourValue => {
                        let spc = b.use_var(self.spc_map[&src]);
                        Some(extract(b, spc))
                    }
                };
                (val, spc)
            }
        }
    }

    /// Whether any bit of `slice` moved, from the drive's changed-bits `mask`, or `None` when the
    /// write cannot have reached the slice at all.
    fn moved_within(&mut self, site: Site, mask: Value, slice: SignalSlice) -> Option<Value> {
        let b = &mut self.b;
        let ptr = self.compiler.ptr;
        let narrow = site.src_size <= VSIZE_64;
        match site.offset {
            DriveOffset::Imm(_) => {
                let window = site.window().unwrap();
                let overlap = slice.overlapping_slice(window)?;
                if overlap == window {
                    return Some(if narrow {
                        mask
                    } else {
                        clif_wide_tv_reduce_or(b, ptr, mask, site.src_size)
                    });
                }
                let relative_offset = overlap.lsb() - window.lsb();
                Some(if narrow {
                    let mut cond = mask;
                    if relative_offset > 0 {
                        cond = b.ins().ushr_imm_u(cond, relative_offset as i64);
                    }
                    if overlap.msb() < window.msb() {
                        cond = b.ins().band_imm_u(cond, mask_of(overlap.width().get()));
                    }
                    cond
                } else {
                    clif_wide_tv_slice_reduce_or(b, ptr, mask, relative_offset, overlap.width())
                })
            }
            DriveOffset::Dyn { bit, .. } => {
                // The mask already holds nothing outside the bounds, so a slice covering them
                // needs no further trimming.
                let covers = slice.lsb() as u64 <= site.bounds.lower
                    && slice.msb() as u64 >= site.bounds.upper;
                let bounds = Bounds {
                    lower: slice.lsb() as u64,
                    upper: slice.msb() as u64,
                };
                Some(if narrow {
                    if covers {
                        mask
                    } else {
                        let trim = window_trim(b, bit, site.src_size.get(), bounds, site.may_wrap);
                        b.ins().band(mask, trim)
                    }
                } else if covers {
                    clif_wide_tv_reduce_or(b, ptr, mask, site.src_size)
                } else {
                    wide_trimmed_reduce_or(b, mask, site.src_size, bit, bounds, site.may_wrap)
                })
            }
        }
    }

    /// Write `src` into the signal and return the changed-bits mask: a value for a narrow `src`,
    /// the address of `dst`'s words for a wide one.
    fn write(&mut self, site: Site, dst: VariableKey, src: VariableKey) -> Value {
        match SixBitSize::from_vector_size(site.src_size) {
            Some(size) => self.write_narrow(site, src, size),
            None => self.write_wide(site, dst, src),
        }
    }

    /// The bit offset within its first word, and that word (and the one after it, for a field that
    /// may cross into it), for a write starting at the drive's offset plus `extra` bits.
    ///
    /// A static offset is in range by construction. A dynamic one is clamped into the signal's
    /// words; a write that clamps has nothing under its trim, so the word goes back unchanged.
    fn locate(&mut self, site: Site, extra: u32, crosses: bool) -> (Amt, WordRef, Option<WordRef>) {
        let b = &mut self.b;
        let heap_ptr = self.params.heap_ptr;
        match site.offset {
            DriveOffset::Imm(off) => {
                let bit = site.base_bit + off as usize + extra as usize;
                let w0 = WordRef {
                    ptr: heap_ptr,
                    off: ((bit / 64) * 8) as i32,
                };
                let w1 = crosses.then(|| w0.add(8));
                (Amt::Imm((bit % 64) as u32), w0, w1)
            }
            DriveOffset::Dyn { bit, .. } => {
                let off = if extra == 0 {
                    bit
                } else {
                    b.ins().iadd_imm_u(bit, extra as i64)
                };
                if !site.is_wide_signal() {
                    // The whole signal sits inside one word, so any in-range bit does too, and an
                    // out-of-range one has been trimmed to nothing.
                    debug_assert!(!crosses);
                    let base_boff = (site.base_bit % 64) as u32;
                    let boff = Amt::Dyn(off).plus(b, base_boff);
                    let w0 = WordRef {
                        ptr: heap_ptr,
                        off: ((site.base_bit / 64) * 8) as i32,
                    };
                    return (boff, w0, None);
                }

                debug_assert_eq!(site.base_bit % 64, 0);
                let base_word = (site.base_bit / 64) as i64;
                let heap_base = if base_word == 0 {
                    heap_ptr
                } else {
                    b.ins().iadd_imm_u(heap_ptr, base_word * 8)
                };
                let upper_word = b.ins().iconst(I64, site.upper_word() as i64);
                let w = b.ins().ushr_imm_u(off, 6);
                let w = b.ins().umin(w, upper_word);
                let w0 = WordRef {
                    ptr: word_ptr(b, heap_base, w),
                    off: 0,
                };
                let w1 = crosses.then(|| {
                    let w1 = b.ins().iadd_imm_u(w, 1);
                    let w1 = b.ins().umin(w1, upper_word);
                    WordRef {
                        ptr: word_ptr(b, heap_base, w1),
                        off: 0,
                    }
                });
                (Amt::Dyn(b.ins().band_imm_u(off, 63)), w0, w1)
            }
        }
    }

    /// The bits of a `width`-bit value at the drive's offset plus `extra` that may be written.
    fn trim(&mut self, site: Site, extra: u32, width: u32) -> Mask {
        let b = &mut self.b;
        match site.offset {
            DriveOffset::Imm(_) => Mask::Imm(mask_of(width) as u64),
            DriveOffset::Dyn { bit, known, .. } => {
                let off = if extra == 0 {
                    bit
                } else {
                    b.ins().iadd_imm_u(bit, extra as i64)
                };
                let mut trim = window_trim(b, off, width, site.bounds, site.may_wrap);
                if let Some(known) = known {
                    let zero = b.ins().iconst(I64, 0);
                    trim = b.ins().select(known, trim, zero);
                }
                Mask::Dyn(trim)
            }
        }
    }

    fn write_narrow(&mut self, site: Site, src: VariableKey, size: SixBitSize) -> Value {
        let width = size as u32;
        let trim = self.trim(site, 0, width);
        let val = self.b.use_var(self.vmap[&src]);
        let val = trim.and(&mut self.b, val);

        match site.mode {
            LogicMode::TwoValue => {
                let crosses = self.crosses(site, 0, width);
                let (boff, w0, w1) = self.locate(site, 0, crosses);
                let b = &mut self.b;
                let old0 = w0.load(b);
                let old1 = w1.map(|w| w.load(b));
                let (n0, n1, changed) = insert_field(b, old0, old1, val, trim, boff);
                // The second word first: where a clamp made the two alias, the first holds the
                // change and has to land last.
                if let (Some(w1), Some(n1)) = (w1, n1) {
                    w1.store(b, n1);
                }
                w0.store(b, n0);
                changed
            }
            LogicMode::FourValue => {
                let spc = self.b.use_var(self.spc_map[&src]);
                let spc = trim.and(&mut self.b, spc);
                let crosses = self.crosses(site, 0, width);
                let (boff, w0, w1) = self.locate(site, 0, crosses);
                let b = &mut self.b;

                if site.signal_size.get() <= 32 {
                    // Both planes are packed into the one word, the val plane a signal's width
                    // further along.
                    let old = w0.load(b);
                    let (n, _, spc_changed) = insert_field(b, old, None, spc, trim, boff);
                    let val_boff = boff.plus(b, site.plane_bits as u32);
                    let (n, _, val_changed) = insert_field(b, n, None, val, trim, val_boff);
                    w0.store(b, n);
                    return b.ins().bor(spc_changed, val_changed);
                }

                let plane_bytes = (site.plane_bits / 8) as i32;
                let v0 = w0.add(plane_bytes);
                let v1 = w1.map(|w| w.add(plane_bytes));

                let old_s0 = w0.load(b);
                let old_s1 = w1.map(|w| w.load(b));
                let (ns0, ns1, spc_changed) = insert_field(b, old_s0, old_s1, spc, trim, boff);
                if let (Some(w1), Some(ns1)) = (w1, ns1) {
                    w1.store(b, ns1);
                }
                w0.store(b, ns0);

                let old_v0 = v0.load(b);
                let old_v1 = v1.map(|w| w.load(b));
                let (nv0, nv1, val_changed) = insert_field(b, old_v0, old_v1, val, trim, boff);
                if let (Some(v1), Some(nv1)) = (v1, nv1) {
                    v1.store(b, nv1);
                }
                v0.store(b, nv0);

                b.ins().bor(spc_changed, val_changed)
            }
        }
    }

    /// Whether a `width`-bit field at the drive's offset plus `extra` can cross a word boundary.
    fn crosses(&self, site: Site, extra: u32, width: u32) -> bool {
        match site.offset {
            DriveOffset::Imm(off) => {
                (site.base_bit + off as usize + extra as usize) % 64 + width as usize > 64
            }
            // Only a wide signal has a next word to cross into; a single bit never crosses.
            DriveOffset::Dyn { .. } => site.is_wide_signal() && width > 1,
        }
    }

    fn write_wide(&mut self, site: Site, dst: VariableKey, src: VariableKey) -> Value {
        let ptr = self.compiler.ptr;
        let cldctx = self.params.cldctx;
        let src_size = site.src_size.get();
        let src_nwords = src_size.div_ceil(64);
        let src_full_nwords = src_size / 64;
        let signal_nwords = site.signal_words();

        // A word-aligned static write of whole words, or one that runs to the signal's end, is a
        // plain copy.
        if let DriveOffset::Imm(off) = site.offset
            && off % 64 == 0
            && (off + src_size == site.signal_size.get() || src_size.is_multiple_of(64))
        {
            let b = &mut self.b;
            let heap_ptr = self.params.heap_ptr;
            let heap_offset = site.base_bit + off as usize;
            let post_bb = b.create_block();
            let loop_bb = b.create_block();

            // (heap_ptr, val_ptr, mask_ptr)
            b.append_block_param(loop_bb, ptr);
            b.append_block_param(loop_bb, ptr);
            b.append_block_param(loop_bb, ptr);

            let start_heap_ptr = b.ins().iadd_imm_u(heap_ptr, (heap_offset / 8) as i64);
            let start_val_ptr = self.wide_map[&src].addr(b, ptr, cldctx, 0);
            let start_mask_ptr = self.wide_map[&dst].addr(b, ptr, cldctx, 0);
            let end_heap_ptr = b.ins().iadd_imm_u(start_heap_ptr, (src_nwords * 8) as i64);

            b.ins().jump(
                loop_bb,
                &[
                    start_heap_ptr.into(),
                    start_val_ptr.into(),
                    start_mask_ptr.into(),
                ],
            );

            b.switch_to_block(loop_bb);
            let cur_heap_ptr = b.block_params(loop_bb)[0];
            let cur_val_ptr = b.block_params(loop_bb)[1];
            let cur_mask_ptr = b.block_params(loop_bb)[2];

            let mask = match site.mode {
                LogicMode::TwoValue => {
                    let old_word = b.ins().load(I64, mem(), cur_heap_ptr, 0);
                    let new_word = b.ins().load(I64, mem(), cur_val_ptr, 0);
                    b.ins().store(mem(), new_word, cur_heap_ptr, 0);
                    b.ins().bxor(old_word, new_word)
                }
                LogicMode::FourValue => {
                    let old_spc = b.ins().load(I64, mem(), cur_heap_ptr, 0);
                    let old_val =
                        b.ins()
                            .load(I64, mem(), cur_heap_ptr, (signal_nwords * 8) as i32);
                    let new_spc = b.ins().load(I64, mem(), cur_val_ptr, 0);
                    let new_val = b
                        .ins()
                        .load(I64, mem(), cur_val_ptr, (src_nwords * 8) as i32);
                    b.ins().store(mem(), new_spc, cur_heap_ptr, 0);
                    b.ins()
                        .store(mem(), new_val, cur_heap_ptr, (signal_nwords * 8) as i32);
                    let mask_spc = b.ins().bxor(old_spc, new_spc);
                    let mask_val = b.ins().bxor(old_val, new_val);
                    b.ins().bor(mask_spc, mask_val)
                }
            };
            b.ins().store(mem(), mask, cur_mask_ptr, 0);

            let next_heap_ptr = b.ins().iadd_imm_u(cur_heap_ptr, 8);
            let next_val_ptr = b.ins().iadd_imm_u(cur_val_ptr, 8);
            let next_mask_ptr = b.ins().iadd_imm_u(cur_mask_ptr, 8);
            let is_lt = b
                .ins()
                .icmp(IntCC::UnsignedLessThan, next_heap_ptr, end_heap_ptr);
            b.ins().brif(
                is_lt,
                loop_bb,
                &[
                    next_heap_ptr.into(),
                    next_val_ptr.into(),
                    next_mask_ptr.into(),
                ],
                post_bb,
                &[],
            );
            b.switch_to_block(post_bb);
            return start_mask_ptr;
        }

        // Otherwise every source word is inserted as a field: the full ones in a loop, the
        // remainder after it. The loop carries the word's position -- as a heap pointer when the
        // drive's offset is static, since then only the word moves, or as a bit offset from the
        // drive's start when it is dynamic, which is folded into the offset itself.
        let start_val_ptr = self.wide_map[&src].addr(&mut self.b, ptr, cldctx, 0);
        let start_mask_ptr = self.wide_map[&dst].addr(&mut self.b, ptr, cldctx, 0);

        if src_full_nwords > 0 {
            let b = &mut self.b;
            let heap_ptr = self.params.heap_ptr;
            let post_bb = b.create_block();
            let loop_bb = b.create_block();

            let (cursor_ty, start_cursor, cursor_step) = match site.offset {
                DriveOffset::Imm(off) => {
                    let heap_offset = site.base_bit + off as usize;
                    let start = b.ins().iadd_imm_u(heap_ptr, (heap_offset / 8) as i64);
                    (ptr, start, 8)
                }
                DriveOffset::Dyn { .. } => (I64, b.ins().iconst(I64, 0), 64),
            };

            // (val_ptr, mask_ptr, cursor)
            b.append_block_param(loop_bb, ptr);
            b.append_block_param(loop_bb, ptr);
            b.append_block_param(loop_bb, cursor_ty);

            let end_val_ptr = b
                .ins()
                .iadd_imm_u(start_val_ptr, (src_full_nwords * 8) as i64);
            b.ins().jump(
                loop_bb,
                &[
                    start_val_ptr.into(),
                    start_mask_ptr.into(),
                    start_cursor.into(),
                ],
            );

            b.switch_to_block(loop_bb);
            let cur_val_ptr = b.block_params(loop_bb)[0];
            let cur_mask_ptr = b.block_params(loop_bb)[1];
            let cursor = b.block_params(loop_bb)[2];

            let word = match site.offset {
                DriveOffset::Imm(_) => Cursor::Word(cursor),
                DriveOffset::Dyn { .. } => Cursor::Bits(cursor),
            };
            let mask = self.write_word(site, cur_val_ptr, src_nwords, word, 64);

            let b = &mut self.b;
            b.ins().store(mem(), mask, cur_mask_ptr, 0);
            let next_val_ptr = b.ins().iadd_imm_u(cur_val_ptr, 8);
            let next_mask_ptr = b.ins().iadd_imm_u(cur_mask_ptr, 8);
            let next_cursor = b.ins().iadd_imm_u(cursor, cursor_step);
            let is_lt = b
                .ins()
                .icmp(IntCC::UnsignedLessThan, next_val_ptr, end_val_ptr);
            b.ins().brif(
                is_lt,
                loop_bb,
                &[
                    next_val_ptr.into(),
                    next_mask_ptr.into(),
                    next_cursor.into(),
                ],
                post_bb,
                &[],
            );
            b.switch_to_block(post_bb);
        }

        if let Some(rem_size) = SixBitSize::last_word_size(site.src_size) {
            let b = &mut self.b;
            let cur_val_ptr = b
                .ins()
                .iadd_imm_u(start_val_ptr, (src_full_nwords * 8) as i64);
            let cur_mask_ptr = b
                .ins()
                .iadd_imm_u(start_mask_ptr, (src_full_nwords * 8) as i64);
            let mask = self.write_word(
                site,
                cur_val_ptr,
                src_nwords,
                Cursor::Static(src_full_nwords * 64),
                rem_size as u32,
            );
            self.b.ins().store(mem(), mask, cur_mask_ptr, 0);
        }

        start_mask_ptr
    }

    /// Insert one `width`-bit word of a wide source, whose planes are at `val_ptr` and
    /// `val_ptr + src_nwords` words, at the position `cursor` names.
    fn write_word(
        &mut self,
        site: Site,
        val_ptr: Value,
        src_nwords: u32,
        cursor: Cursor,
        width: u32,
    ) -> Value {
        let (trim, boff, w0, w1) = match cursor {
            Cursor::Static(extra) => {
                let trim = self.trim(site, extra, width);
                let crosses = self.crosses(site, extra, width);
                let (boff, w0, w1) = self.locate(site, extra, crosses);
                (trim, boff, w0, w1)
            }
            Cursor::Word(cur) => {
                // A static drive walking its words: the bit within the word never changes, and
                // every full word is in range.
                let DriveOffset::Imm(off) = site.offset else {
                    unreachable!("a word cursor belongs to a static drive")
                };
                let boff = ((site.base_bit + off as usize) % 64) as u32;
                let w0 = WordRef { ptr: cur, off: 0 };
                let w1 = (boff > 0).then(|| w0.add(8));
                (Mask::Imm(u64::MAX), Amt::Imm(boff), w0, w1)
            }
            Cursor::Bits(extra) => {
                // A dynamic position is folded into the offset itself, so the rest of the
                // machinery sees an ordinary dynamic write.
                let b = &mut self.b;
                let DriveOffset::Dyn {
                    bit,
                    known,
                    may_wrap,
                } = site.offset
                else {
                    unreachable!("a bit cursor belongs to a dynamic drive")
                };
                let offset = DriveOffset::Dyn {
                    bit: b.ins().iadd(bit, extra),
                    known,
                    may_wrap,
                };
                let site = Site { offset, ..site };
                let trim = self.trim(site, 0, width);
                let crosses = self.crosses(site, 0, width);
                let (boff, w0, w1) = self.locate(site, 0, crosses);
                (trim, boff, w0, w1)
            }
        };
        let b = &mut self.b;

        let val = b.ins().load(I64, mem(), val_ptr, 0);
        let val = trim.and(b, val);

        let planes: Vec<(i32, Value)> = match site.mode {
            LogicMode::TwoValue => vec![(0, val)],
            LogicMode::FourValue => {
                let spc = val;
                let v = b.ins().load(I64, mem(), val_ptr, (src_nwords * 8) as i32);
                let v = trim.and(b, v);
                vec![(0, spc), ((site.plane_bits / 8) as i32, v)]
            }
        };

        let mut changed = None;
        for (plane_bytes, value) in planes {
            let p0 = w0.add(plane_bytes);
            let p1 = w1.map(|w| w.add(plane_bytes));
            let old0 = p0.load(b);
            let old1 = p1.map(|w| w.load(b));
            let (n0, n1, moved) = insert_field(b, old0, old1, value, trim, boff);
            if let (Some(p1), Some(n1)) = (p1, n1) {
                p1.store(b, n1);
            }
            p0.store(b, n0);
            changed = Some(match changed {
                None => moved,
                Some(c) => b.ins().bor(c, moved),
            });
        }
        changed.unwrap()
    }
}

/// Where one word of a wide source goes.
#[derive(Clone, Copy)]
enum Cursor {
    /// A fixed number of bits past the drive's start.
    Static(u32),
    /// The heap word itself, for a static drive walking its words.
    Word(Value),
    /// A run-time number of bits past the drive's start, for a dynamic drive doing the same.
    Bits(Value),
}

/// `base + w * 8`.
fn word_ptr(b: &mut FunctionBuilder, base: Value, w: Value) -> Value {
    let bytes = b.ins().ishl_imm_u(w, 3);
    b.ins().iadd(base, bytes)
}

/// OR together the bits of a wide changed-bits mask, written at a dynamic `off`, that fall inside
/// `bounds`. Only a partial watch on a wide signal driven at a dynamic offset needs this.
fn wide_trimmed_reduce_or(
    b: &mut FunctionBuilder,
    mask_ptr: Value,
    size: VectorSize,
    off: Value,
    bounds: Bounds,
    may_wrap: bool,
) -> Value {
    let nwords = size.get().div_ceil(64);
    let mut acc = None;
    for i in 0..nwords {
        let width = (size.get() - i * 64).min(64);
        let word_off = if i == 0 {
            off
        } else {
            b.ins().iadd_imm_u(off, (i * 64) as i64)
        };
        let trim = window_trim(b, word_off, width, bounds, may_wrap);
        let word = b.ins().load(I64, mem(), mask_ptr, (i * 8) as i32);
        let word = b.ins().band(word, trim);
        acc = Some(match acc {
            None => word,
            Some(a) => b.ins().bor(a, word),
        });
    }
    acc.unwrap()
}

/// The dynamic offset of a `DriveSlice`, from its index variable: the bit, and whether it is
/// known, which a four-valued index may not be.
///
/// An index wider than a word is out of range whenever any upper word is set, and that is folded
/// into the offset by driving it to the maximum, which every bound then rejects.
impl<'a, 'b> TrBuilder<'a, 'b> {
    pub(super) fn dyn_offset(&mut self, index: VariableKey) -> DriveOffset {
        let gl = self.compiler.gl;
        let size = gl.vars.size(index);
        let ptr = self.compiler.ptr;
        let cldctx = self.params.cldctx;
        let is_fv = index.mode() == LogicMode::FourValue;
        let b = &mut self.b;

        let (bit, known) = match SixBitSize::from_vector_size(size) {
            Some(_) => {
                let bit = b.use_var(self.vmap[&index]);
                let known = is_fv.then(|| {
                    let spc = b.use_var(self.spc_map[&index]);
                    b.ins().icmp_imm_u(IntCC::Equal, spc, mask_of(size.get()))
                });
                (bit, known)
            }
            None => {
                let nwords = size.get().div_ceil(64);
                let loc = self.wide_map[&index];
                let mut bit = wide_load(b, ptr, cldctx, loc, 0);
                let mut high = None;
                for i in 1..nwords {
                    let w = wide_load(b, ptr, cldctx, loc, i);
                    high = Some(match high {
                        None => w,
                        Some(h) => b.ins().bor(h, w),
                    });
                }
                if let Some(high) = high {
                    let all = b.ins().iconst(I64, -1);
                    bit = b.ins().select(high, all, bit);
                }
                let known = is_fv.then(|| {
                    let mut known = None;
                    for i in 0..nwords {
                        let spc = wide_load(b, ptr, cldctx, loc, nwords + i);
                        let top = if i + 1 == nwords {
                            super::top_i64(size.get())
                        } else {
                            -1
                        };
                        let k = b.ins().icmp_imm_u(IntCC::Equal, spc, top);
                        known = Some(match known {
                            None => k,
                            Some(a) => b.ins().band(a, k),
                        });
                    }
                    known.unwrap()
                });
                (bit, known)
            }
        };
        DriveOffset::Dyn {
            bit,
            known,
            may_wrap: size.get() >= 64,
        }
    }
}

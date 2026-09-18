//! Lowering of Vogls IR to Cranelift IR + in-process JIT compilation.
//!
//! One Cranelift function per Temporal Region (TR), using the `tail` calling
//! convention so active-region draining chains via guaranteed tail calls.
//! Schedule manipulation is emitted as internal CLIF helpers operating on the
//! `#[repr(C)]` ABI structs from [`crate::runtime`]; nothing is bound through
//! host symbols (the module stays cacheable).
//!
//! Coverage: every IR instruction is lowered — two-value and four-value, any
//! width (values above `WIDE_HEAP_THRESHOLD_WORDS` live in a heap scratch
//! region, the rest in stack slots). Wide / cold arithmetic is delegated to the
//! `vogls-bits` word-slice routines via the `wide_binop` shim. The only IR the
//! backend does not emit is the VCD dump family (`$dumpfile`/`$dumpvars`, a
//! bytecode-only feature), which fails cleanly at compile time.

mod terminator;
mod tr;

use std::io;
use std::mem::offset_of;
use std::sync::{Arc, Mutex};

use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    AbiParam, Block, InstBuilder, MemFlagsData, Signature, StackSlot, StackSlotData, StackSlotKind,
    Type, UserFuncName, Value, types,
};
use cranelift_codegen::isa::{CallConv, TargetFrontendConfig};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module, default_libcall_names};

use vogls_codegen::{HeapBuilder, HeapRef, SixBitSize};
use vogls_ir::dyn_format_string::DynFormatString;
use vogls_ir::time::{TimeFormat, TimeResolution};
use vogls_ir::watchers::WatchMap;
use vogls_ir::{
    BasicBlockKey, BasicBlockTerminator, GlobalContext, LogicMode, ShiftImmOp, SignalKey,
    TemporalRegionKey, VariableKey, VectorSize, WatchEdge,
};
use vogls_runtime::RtSignalKey;
use vogls_utils::VgHashMap;

use crate::ffi::FfiVec;
use crate::runtime::{ColdContextT, EventT, ScheduleT, layout};

use self::tr::TrBuilder;

#[repr(C)]
pub struct Params {
    pub heap_ptr: Value,
    pub schedule: Value,
    pub time: Value,
    pub listening: Value,
    pub last_active_time: Value,
    pub cldctx: Value,
}

impl Params {
    pub fn from_block_params(b: &mut FunctionBuilder, blk: Block) -> Self {
        let blk_params = b.block_params(blk);
        assert_eq!(blk_params.len(), 6);
        Self {
            heap_ptr: blk_params[0],
            schedule: blk_params[1],
            time: blk_params[2],
            listening: blk_params[3],
            last_active_time: blk_params[4],
            cldctx: blk_params[5],
        }
    }

    fn as_slice(&self) -> &[Value] {
        unsafe { std::mem::transmute::<&Self, &[Value; 6]>(self) }.as_slice()
    }
}

const I64: Type = types::I64;
const F64: Type = types::F64;

fn mem() -> MemFlagsData {
    MemFlagsData::trusted()
}

fn mem_ro() -> MemFlagsData {
    MemFlagsData::trusted().with_readonly()
}

fn cast() -> MemFlagsData {
    MemFlagsData::new()
}

/// Signal/heap information the lowering needs (built from `prepare_codegen`).
pub struct SignalInfo<'a> {
    /// Heap location per signal, indexed by `RtSignalKey::as_usize`.
    pub signal_to_heap: &'a [HeapRef],
    pub rt_signal_map: &'a VgHashMap<SignalKey, RtSignalKey>,
    /// Logic mode per signal, indexed by `RtSignalKey::as_usize`.
    pub signal_mode: &'a [LogicMode],
    /// `last_active_time` slot per signal that has a `LastUpdateTime` reader.
    pub lupdt_indexes: &'a VgHashMap<RtSignalKey, u64>,
}

impl SignalInfo<'_> {
    fn heap_ref(&self, sig: SignalKey) -> (HeapRef, RtSignalKey, LogicMode) {
        let rt = self.rt_signal_map[&sig];
        (
            self.signal_to_heap[rt.as_usize()],
            rt,
            self.signal_mode[rt.as_usize()],
        )
    }
}

// ---------------------------------------------------------------------------
// Signatures
// ---------------------------------------------------------------------------

struct Sigs {
    event: Signature,
    entry: Signature,
    grow: Signature,
    plugin_poke: Signature,
    fmt: Signature,
}

impl Sigs {
    fn new(ptr: Type) -> Self {
        let sysv = |params: &[Type], ret: Option<Type>| {
            let mut s = Signature::new(CallConv::SystemV);
            s.params.extend(params.iter().copied().map(AbiParam::new));
            if let Some(r) = ret {
                s.returns.push(AbiParam::new(r));
            }
            s
        };
        let mut event = sysv(&[ptr, ptr, I64, ptr, ptr, ptr], Some(types::I32));
        event.call_conv = CallConv::Tail;
        Self {
            event,
            entry: sysv(&[ptr, ptr, I64, ptr, ptr, ptr], Some(types::I32)),
            grow: sysv(&[ptr], None),
            plugin_poke: sysv(&[ptr, I64], None),
            fmt: sysv(&[ptr, I64, ptr], None),
        }
    }
}

// ---------------------------------------------------------------------------
// Compiler
// ---------------------------------------------------------------------------

pub struct Compiled {
    pub module: JITModule,
    pub entry: FuncId,
    pub procs: Vec<FuncId>,
    pub watch_offsets: Vec<u32>,
    pub watch_entries: Vec<(u32, FuncId)>,
    pub num_listening: usize,
    pub dyn_fmt_strs: Vec<DynFormatString>,
    pub read_mems: Vec<(HeapRef, vogls_ir::ReadMem)>,
    pub time_fmts: Vec<TimeFormat>,
    pub standing_procs: vogls_utils::VgHashSet<usize>,
    pub standing_arm_offsets: Vec<u32>,
}

impl Compiled {
    pub fn into_design(
        self,
        time_resolution: TimeResolution,
        heap_wide_ptr: u64,
        num_regions: u8,
    ) -> crate::runtime::ClifDesign {
        use crate::runtime::{ClifDesign, ClifWatchers, EmptyActiveEventQueueFn, EventT};
        let Compiled {
            module,
            entry: entry_id,
            procs: proc_ids,
            watch_offsets,
            watch_entries,
            dyn_fmt_strs,
            read_mems,
            time_fmts,
            standing_procs,
            standing_arm_offsets,
            num_listening: _,
        } = self;
        let entry: EmptyActiveEventQueueFn =
            unsafe { std::mem::transmute(module.get_finalized_function(entry_id)) };
        let procs = proc_ids
            .iter()
            .map(|&id| EventT::from_ptr(module.get_finalized_function(id)))
            .collect();
        let watch_entries = watch_entries
            .iter()
            .map(|&(offset, id)| (offset, EventT::from_ptr(module.get_finalized_function(id))))
            .collect();
        let watchers = ClifWatchers::new(watch_offsets, watch_entries);
        ClifDesign::from_parts(
            module,
            entry,
            procs,
            watchers,
            dyn_fmt_strs,
            read_mems,
            time_fmts,
            time_resolution,
            heap_wide_ptr,
            num_regions,
            standing_procs,
            standing_arm_offsets,
        )
    }
}

/// A registered listener: when `signal` is poked, if bit `offset` of `listening`
/// is set, clear it and schedule `target` into the active region.
struct Listener {
    offset: u32,
    target: FuncId,
    /// Which transition wakes it. A directed edge is only ever recorded on a two-valued signal,
    /// whose drives know the level each bit settles on and so can decide the edge themselves.
    edge: WatchEdge,
    /// The bit of the signal a directed edge watches. Always 0 for `WatchEdge::Any`, which covers
    /// the signal as a whole here.
    bit: u32,
}

/// The bits on which a four-valued write took `edge`, going from (`xspc`, `xval`) to
/// (`yspc`, `yval`).
///
/// This is `vogls_bits::edge::fv_posedge_u64` / `fv_negedge_u64` emitted inline, so a transition
/// through `x` or `z` counts the same way it does everywhere else: `0` to `x` is a posedge, `x` to
/// `0` a negedge, and `x` to `z` neither.
fn fv_edge_mask(
    b: &mut FunctionBuilder,
    edge: WatchEdge,
    xspc: Value,
    xval: Value,
    yspc: Value,
    yval: Value,
) -> Value {
    let not = |b: &mut FunctionBuilder, v: Value| b.ins().bxor_imm_u(v, -1);
    // Posedge reads the value planes as they are; negedge is the same shape with both inverted.
    let (xval, yval) = match edge {
        WatchEdge::Posedge => (xval, yval),
        WatchEdge::Negedge => (not(b, xval), not(b, yval)),
        WatchEdge::Any { .. } => unreachable!("Only a directed edge has a mask"),
    };

    // Leaving a known level: x was that level, and y is no longer definitely it.
    let nxval = not(b, xval);
    let left = b.ins().band(xspc, nxval);
    let nyspc = not(b, yspc);
    let reaches = b.ins().bor(nyspc, yval);
    let left = b.ins().band(left, reaches);

    // Arriving at one: x was not known, and y now definitely is that level.
    let nxspc = not(b, xspc);
    let arrived = b.ins().band(nxspc, yspc);
    let arrived = b.ins().band(arrived, yval);

    b.ins().bor(left, arrived)
}

/// The bits of a slice written at `off` that moved, as a mask of the *value* written rather than
/// of the signal -- so it is shifted back down and trimmed to `s_size`.
///
/// An unknown offset writes nothing, so it moves nothing.
fn slice_moved(
    b: &mut FunctionBuilder,
    cur: Value,
    new: Value,
    off: Value,
    off_known: Value,
    s_size: u32,
) -> Value {
    let d = b.ins().bxor(new, cur);
    let d = b.ins().ushr(d, off);
    let d = maskv(b, d, s_size);
    let zero = b.ins().iconst(I64, 0);
    b.ins().select(off_known, d, zero)
}

/// The changed-bits mask a drive produced: which of the bits it wrote actually moved. This is
/// what `Instruction::Drive` names as its destination, and it is as wide as the value written.
///
/// It is computed before the drive branches on whether anything changed, so it is defined whether
/// or not the write went ahead -- all zeroes in the latter case, which is exactly right.
pub enum DriveMask {
    /// A mask that fits one machine word.
    Word(Value),
    /// One word per `u64` of a value too wide for that.
    Words(Vec<Value>),
    /// The drive could not say. The destination is left all zeroes.
    Unknown,
}

/// What a drive can tell the wake set about the bits it wrote.
///
/// That is everything a watch needs: which of those bits moved, and which way. A drive passes
/// `None` in place of this only where it cannot produce it at all -- it is not a way of saying "no
/// drive happened", since every drive site has one.
#[derive(Clone, Copy)]
enum DrivenBits<'a> {
    /// A write that fits a single machine word, covering `size` bits from `bit` of the signal.
    Word {
        bit: u32,
        size: u32,
        /// The value plane, after and before. For a two-valued drive this is the whole value.
        new: Value,
        old: Value,
        /// Four-valued only: the `spc` plane after and before, saying which of those bits hold a
        /// known `0`/`1` rather than `x` or `z`. A two-valued drive's bits are all known by
        /// construction.
        spc: Option<(Value, Value)>,
    },
    /// A write into a signal too wide for one word, described word by word: `words[i]` is a word
    /// index within the signal, `moved[i]` the bits of it that changed and `new[i]` what it
    /// settled on. Only the words an edge watch looks at need be listed.
    Words {
        words: &'a [usize],
        moved: &'a [Value],
        new: &'a [Value],
    },
}

/// Wide (>64-bit) values with at most this many u64 words are stored in a
/// Cranelift stack slot; larger ones would blow the machine stack, so they are
/// placed in a compile-time-reserved scratch region of the runtime heap.
const WIDE_HEAP_THRESHOLD_WORDS: usize = 256;

/// Where a wide (>64-bit) value's words live within a TR. `Slot` is a Cranelift
/// stack slot (the common case); `Heap` is an absolute u64-word offset into the
/// runtime heap's scratch region (for values above `WIDE_HEAP_THRESHOLD_WORDS`).
/// The scratch region is reused across TRs (only one TR runs at a time), sized
/// to the largest TR's spilled footprint.
#[derive(Clone, Copy)]
enum WideLoc {
    Slot(StackSlot),
    Heap(u32),
}

impl WideLoc {
    fn addr(self, b: &mut FunctionBuilder, ptr: Type, cldctx: Value, word: u32) -> Value {
        match self {
            WideLoc::Slot(s) => b.ins().stack_addr(ptr, s, (word * 8) as i32),
            WideLoc::Heap(base) => {
                let heap_wide_ptr = b.ins().load(
                    ptr,
                    mem_ro(),
                    cldctx,
                    offset_of!(ColdContextT, heap_wide_ptr) as i32,
                );
                b.ins()
                    .iadd_imm_u(heap_wide_ptr, i64::from(base + word) * 8)
            }
        }
    }
}

type WideMap = VgHashMap<VariableKey, WideLoc>;

/// Load word `word` of a wide value, resolving its base through [`WideLoc::addr`]
/// (stack slot, or the cold-context wide-scratch pointer for a heap location).
fn wide_load(b: &mut FunctionBuilder, ptr: Type, cldctx: Value, loc: WideLoc, word: u32) -> Value {
    let a = loc.addr(b, ptr, cldctx, word);
    b.ins().load(I64, mem(), a, 0)
}
/// Store `val` into word `word` of a wide value (see [`wide_load`] for addressing).
fn wide_store(
    b: &mut FunctionBuilder,
    ptr: Type,
    cldctx: Value,
    loc: WideLoc,
    word: u32,
    val: Value,
) {
    let a = loc.addr(b, ptr, cldctx, word);
    b.ins().store(mem(), val, a, 0);
}

/// Max over all TRs of the total spilled-word footprint (vars above the heap
/// threshold), assuming every spilled var is live for the whole TR (no liveness
/// analysis). This sizes the heap scratch region; each TR reuses it from 0.
pub fn max_scratch_words(gl: &GlobalContext) -> usize {
    let mut max = 0usize;
    for (_pk, process) in gl.processes.iter() {
        for tr in process.regions.iter() {
            let mut seen = vogls_utils::VgHashSet::default();
            let mut visited = vogls_utils::VgHashSet::default();
            let mut total = 0usize;
            visited.insert(tr.entry());
            let mut stack = vec![tr.entry()];
            while let Some(k) = stack.pop() {
                let _ = gl.bbs[k].try_for_each_dst_var(|v| {
                    if seen.insert(v) {
                        let words = var_words(gl.vars.size(v), v.mode());
                        if words > WIDE_HEAP_THRESHOLD_WORDS {
                            total += words;
                        }
                    }
                    Ok::<(), ()>(())
                });
                gl.bbs[k].terminator.for_each_non_temporal_bb(|s| {
                    if visited.insert(s) {
                        stack.push(s);
                    }
                });
            }
            max = max.max(total);
        }
    }
    max
}

struct Compiler<'a> {
    module: JITModule,
    ptr: Type,
    fe: TargetFrontendConfig,
    sigs: Sigs,
    entry: FuncId,
    tr_funcs: VgHashMap<TemporalRegionKey, FuncId>,

    // Tailcall-able functions that first grow the target region or schedule queue, push an entry
    // and then go to the next event.
    //
    // Making this a tailcall-able function ensures that no stackframe is required in the hotpath.
    wait_region_grow: Option<FuncId>,
    wait_time_grow: Option<FuncId>,

    /// Listeners per signal (by `RtSignalKey::as_usize`), collected while
    /// lowering `Watch` terminators.
    listeners: Vec<Vec<Listener>>,
    num_listening: u32,
    /// Offset assigned to each `Watch` terminator, keyed by the BB it terminates.
    /// Populated by the listener pre-pass so drive sites can inline the wake set.
    /// The IR's watch map, shared with the bytecode backend: it numbers every watch and groups
    /// them by signal, so neither backend invents its own idea of what a watch covers.
    watch_map: WatchMap,
    num_plugins: usize,
    dyn_fmt_strs: Vec<DynFormatString>,
    read_mems: Vec<(HeapRef, vogls_ir::ReadMem)>,
    time_fmts: Vec<TimeFormat>,
    /// Process indices that are "standing": their listeners are armed at startup
    /// but their body must NOT run at t=0, so they are not seeded into the
    /// active region (mirrors the bytecode backend).
    standing_procs: vogls_utils::VgHashSet<usize>,
    /// Listener offsets to pre-arm at startup for the standing processes.
    standing_arm_offsets: Vec<u32>,
    /// Base u64-word offset into the runtime heap of the wide-value scratch
    /// region (for vars above `WIDE_HEAP_THRESHOLD_WORDS`).
    scratch_base: u32,

    gl: &'a GlobalContext,
    info: SignalInfo<'a>,
    heap_builder: &'a mut HeapBuilder,

    emit_clif: Option<Arc<Mutex<dyn io::Write + Send + Sync>>>,
    emit_disassembly: Option<Arc<Mutex<dyn io::Write + Send + Sync>>>,
}

impl<'a> Compiler<'a> {
    fn new(
        num_signals: usize,
        num_plugins: usize,
        watch_map: WatchMap,
        gl: &'a GlobalContext,
        info: SignalInfo<'a>,
        heap_builder: &'a mut HeapBuilder,
        emit_clif: Option<Arc<Mutex<dyn io::Write + Send + Sync>>>,
        emit_disassembly: Option<Arc<Mutex<dyn io::Write + Send + Sync>>>,
    ) -> Self {
        let mut fb = settings::builder();
        fb.set("opt_level", "speed").unwrap();
        // Cranelift's tail-call convention (used for temporal-region dispatch)
        // currently requires frame pointers to be preserved.
        fb.set("preserve_frame_pointers", "true").unwrap();
        let flags = settings::Flags::new(fb);
        let isa = cranelift_native::builder()
            .expect("unsupported host")
            .finish(flags)
            .expect("failed to build ISA");
        let module = JITModule::new(JITBuilder::with_isa(isa, default_libcall_names()));
        let ptr = module.target_config().pointer_type();
        let fe = module.target_config();
        let sigs = Sigs::new(ptr);
        Self {
            module,
            ptr,
            fe,
            sigs,
            entry: FuncId::from_u32(0),
            tr_funcs: VgHashMap::default(),
            wait_region_grow: None,
            wait_time_grow: None,
            listeners: (0..num_signals).map(|_| Vec::new()).collect(),
            num_listening: 0,
            watch_map,
            num_plugins,
            dyn_fmt_strs: Vec::new(),
            read_mems: Vec::new(),
            time_fmts: Vec::new(),
            standing_procs: vogls_utils::VgHashSet::default(),
            standing_arm_offsets: Vec::new(),
            scratch_base: 0,
            gl,
            info,
            heap_builder,
            emit_clif,
            emit_disassembly,
        }
    }

    fn declare(&mut self, name: &str, sig: &Signature) -> FuncId {
        self.module
            .declare_function(name, Linkage::Local, sig)
            .unwrap()
    }

    // --- schedule helpers (see runtime.rs layout) ---------------------------

    /// Push `event` onto `vec` inline, assuming spare capacity: no capacity
    /// check and no grow `call_indirect`.
    ///
    /// For the *active* region that holds unconditionally: it is pre-sized to
    /// the process count and never grows (see `ClifDesign::run`'s drain-based
    /// region advance and `new_state`'s reservation). For the other regions the
    /// `WaitRegion` site checks capacity first and diverts a full region to its
    /// `grow_and_push_in_region_then_next_event` helper. Either way no call is
    /// left on the hot path, which is what lets an inlined-drive or
    /// `WaitRegion` TR stay leaf (no frame / callee-save prologue).
    fn emit_push_inline(&mut self, b: &mut FunctionBuilder, vec: Value, event: Value) {
        let len = b
            .ins()
            .load(I64, mem(), vec, FfiVec::<EventT>::LEN_OFFSET as i32);
        self.emit_push_inline_at(b, vec, event, len);
    }

    /// As [`Self::emit_push_inline`], for a caller that has already loaded
    /// `vec.length` (the `WaitRegion` capacity check).
    fn emit_push_inline_at(
        &mut self,
        b: &mut FunctionBuilder,
        vec: Value,
        event: Value,
        len: Value,
    ) {
        let data = b
            .ins()
            .load(self.ptr, mem(), vec, FfiVec::<EventT>::PTR_OFFSET as i32);
        let esize = b.ins().iconst(self.ptr, layout::EVENT_SIZE as i64);
        let off = b.ins().imul(len, esize);
        let elem = b.ins().iadd(data, off);
        b.ins().store(mem(), event, elem, 0);
        let one = b.ins().iconst(I64, 1);
        let nl = b.ins().iadd(len, one);
        b.ins()
            .store(mem(), nl, vec, FfiVec::<EventT>::LEN_OFFSET as i32);
    }

    /// Schedule `event` at `time` on the future queue, then continue to the
    /// next active event.
    ///
    /// The `WaitRegion` treatment, for `Wait`/`VariableWait`: with spare
    /// capacity the timed event is stored inline, and a full queue hands
    /// `(event, time)` to `grow_and_push_future_then_next_event` through the
    /// cold context and tail-calls it. Growing is the only part that needs a
    /// call, so the TR keeps none on its hot path.
    fn emit_schedule_future_event(
        &mut self,
        b: &mut FunctionBuilder,
        params: &Params,
        event: Value,
        time: Value,
    ) {
        let future = ioff(b, self.ptr, params.schedule, layout::SCHED_FUTURE);

        let grow_bb = b.create_block();
        let push_bb = b.create_block();
        b.set_cold_block(grow_bb);

        let len = b
            .ins()
            .load(I64, mem(), future, FfiVec::<EventT>::LEN_OFFSET as i32);
        let cap = b
            .ins()
            .load(I64, mem(), future, FfiVec::<EventT>::CAP_OFFSET as i32);
        let full = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, len, cap);
        b.ins().brif(full, grow_bb, &[], push_bb, &[]);

        b.switch_to_block(grow_bb);
        b.ins().store(
            mem(),
            event,
            params.cldctx,
            layout::CTX_PENDING_EVENT as i32,
        );
        b.ins()
            .store(mem(), time, params.cldctx, layout::CTX_PENDING_TIME as i32);
        let helper = match self.wait_time_grow {
            None => {
                let id = self.declare(
                    "grow_and_push_future_then_next_event",
                    &self.sigs.event.clone(),
                );
                self.wait_time_grow = Some(id);
                id
            }
            Some(id) => id,
        };
        let helper_ref = self.module.declare_func_in_func(helper, b.func);
        b.ins().return_call(helper_ref, params.as_slice());

        b.switch_to_block(push_bb);
        self.emit_push_future_inline_at(b, params.schedule, future, event, time, len);
        self.tail_pop_next_or_return(b, params);
    }

    /// Store the timed event `(event, time)` at `future[len]` and bump the
    /// length, assuming spare capacity, then fold `time` into
    /// `schedule->next_time`.
    fn emit_push_future_inline_at(
        &mut self,
        b: &mut FunctionBuilder,
        schedule: Value,
        future: Value,
        event: Value,
        time: Value,
        len: Value,
    ) {
        let data = b
            .ins()
            .load(self.ptr, mem(), future, FfiVec::<EventT>::PTR_OFFSET as i32);
        let off = b.ins().imul_imm_u(len, layout::TIMED_EVENT_SIZE as i64);
        let elem = b.ins().iadd(data, off);
        b.ins()
            .store(mem(), event, elem, layout::TIMED_EVENT_EVENT as i32);
        b.ins()
            .store(mem(), time, elem, layout::TIMED_EVENT_TIME as i32);
        let nl = b.ins().iadd_imm_s(len, 1);
        b.ins()
            .store(mem(), nl, future, FfiVec::<EventT>::LEN_OFFSET as i32);
        // next_time = min(next_time, time); relies on invariant
        // "future empty => next_time == u64::MAX" (maintained by the driver).
        let nt = b
            .ins()
            .load(I64, mem(), schedule, layout::SCHED_NEXT_TIME as i32);
        let new_nt = b.ins().umin(nt, time);
        b.ins()
            .store(mem(), new_nt, schedule, layout::SCHED_NEXT_TIME as i32);
    }

    fn build_entry(&mut self, fb: &mut FunctionBuilderContext) {
        let mut ctx = self.module.make_context();
        ctx.func.signature = self.sigs.entry.clone();
        ctx.func.name = UserFuncName::user(0, self.entry.as_u32());
        {
            let mut b = FunctionBuilder::new(&mut ctx.func, fb);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let params = Params::from_block_params(&mut b, entry);

            self.pop_next_or_return(&mut b, &params, false);
            b.seal_all_blocks();
            b.finalize(self.fe);
        }
        if let Some(writer) = self.emit_clif.as_mut() {
            let mut writer = writer.lock().unwrap();
            writeln!(writer, "=== entry ===\n{}", ctx.func.display()).unwrap();
        }
        self.module.define_function(self.entry, &mut ctx).unwrap();
        self.module.clear_context(&mut ctx);
    }

    fn build_tr(
        &mut self,
        fb: &mut FunctionBuilderContext,
        process_idx: usize,
        tr_idx: usize,
        entry_bb: BasicBlockKey,
        bb_phis: &VgHashMap<BasicBlockKey, Vec<(VariableKey, VariableKey)>>,
    ) {
        let func_id = self.tr_funcs[&TemporalRegionKey::from_entry(entry_bb)];
        let mut ctx = self.module.make_context();
        let mut builder = TrBuilder::new(&mut ctx, self, fb, func_id, entry_bb);
        builder.lower(bb_phis);
        builder.finalize();
        if let Some(writer) = self.emit_clif.as_mut() {
            let mut writer = writer.lock().unwrap();
            writeln!(
                writer,
                "=== tr {process_idx}_{tr_idx} ===\n{}",
                ctx.func.display()
            )
            .unwrap();
        }
        self.module.define_function(func_id, &mut ctx).unwrap();
        if let Some(writer) = self.emit_disassembly.as_mut() {
            let mut writer = writer.lock().unwrap();
            if let Some(cc) = ctx.compiled_code() {
                if let Some(d) = cc.vcode.as_ref() {
                    writeln!(writer, "=== tr {process_idx}_{tr_idx} ===\n{d}",).unwrap();
                }
            }
        }
        self.module.clear_context(&mut ctx);
    }

    fn intern_fmt(&mut self, fmt: &DynFormatString) -> usize {
        let idx = self.dyn_fmt_strs.len();
        self.dyn_fmt_strs.push(fmt.clone());
        idx
    }

    /// Emit `(cldctx->fn_table.fmt)(cldctx->stdout, fmt_strs + i*sizeof, args)`,
    /// building a `bits_ref_t[]` on the stack. Two-value args occupy one word;
    /// four-value args are packed `spc|(val<<size)` for size<=32, or two words
    /// (spc then val) for size 33..=64 (matching the heap / `fmt` decoding).
    #[expect(clippy::too_many_arguments)]
    fn emit_fmt_call(
        &mut self,
        b: &mut FunctionBuilder,
        params: &Params,
        vmap: &VgHashMap<VariableKey, Variable>,
        spc_map: &VgHashMap<VariableKey, Variable>,
        wide_map: &WideMap,
        items: &[VariableKey],
        fmt_index: usize,
    ) {
        let n = items.len().max(1);
        // Per-arg value-slot byte offsets. Wide (>64) args live in their own
        // stack slot (heap layout), so they don't consume a value slot here.
        let mut offsets = Vec::with_capacity(items.len());
        let mut total = 0usize;
        for item in items {
            offsets.push(total);
            if self.gl.vars.size(*item).get() > 64 {
                continue;
            }
            let fv = item.mode() == LogicMode::FourValue;
            total += if fv && self.gl.vars.size(*item).get() > 32 {
                16
            } else {
                8
            };
        }
        let val_slot = b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            total.max(8) as u32,
            3,
        ));
        let arr_slot = b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            (n * layout::BITSREF_SIZEOF) as u32,
            3,
        ));
        for (i, item) in items.iter().enumerate() {
            let off = offsets[i] as i32;
            let size = self.gl.vars.size(*item).get();
            let fv = item.mode() == LogicMode::FourValue;
            // Pointer to this arg's bits: the wide slot (already in heap layout)
            // for >64, otherwise a freshly packed word in `val_slot`.
            let arg_ptr = if size > 64 {
                wide_map[item].addr(b, self.ptr, params.cldctx, 0)
            } else {
                if !fv {
                    let v = b.use_var(vmap[item]);
                    b.ins().stack_store(self.ptr, v, val_slot, off);
                } else if size <= 32 {
                    let v = b.use_var(vmap[item]);
                    let s = b.use_var(spc_map[item]);
                    let vh = b.ins().ishl_imm_u(v, size as i64);
                    let packed = b.ins().bor(vh, s);
                    b.ins().stack_store(self.ptr, packed, val_slot, off);
                } else {
                    let v = b.use_var(vmap[item]);
                    let s = b.use_var(spc_map[item]);
                    b.ins().stack_store(self.ptr, s, val_slot, off);
                    b.ins().stack_store(self.ptr, v, val_slot, off + 8);
                }
                b.ins().stack_addr(self.ptr, val_slot, off)
            };
            let szc = b.ins().iconst(types::I32, size as i64);
            b.ins().stack_store(
                self.ptr,
                szc,
                arr_slot,
                (i * layout::BITSREF_SIZEOF + layout::BITSREF_SIZE_OFF) as i32,
            );
            let modec = b.ins().iconst(types::I8, if fv { 1 } else { 0 });
            b.ins().stack_store(
                self.ptr,
                modec,
                arr_slot,
                (i * layout::BITSREF_SIZEOF + layout::BITSREF_MODE_OFF) as i32,
            );
            let p = arg_ptr;
            b.ins().stack_store(
                self.ptr,
                p,
                arr_slot,
                (i * layout::BITSREF_SIZEOF + layout::BITSREF_PTR_OFF) as i32,
            );
        }
        let cldctx = params.cldctx;
        let fmt_ptr = b.ins().load(
            self.ptr,
            mem(),
            cldctx,
            (layout::CTX_FN_TABLE + layout::FN_FMT) as i32,
        );
        let fmt_index = b.ins().iconst(I64, fmt_index as i64);
        let args_ptr = b.ins().stack_addr(self.ptr, arr_slot, 0);
        let sig = b.import_signature(self.sigs.fmt.clone());
        b.ins()
            .call_indirect(sig, fmt_ptr, &[cldctx, fmt_index, args_ptr]);
    }

    /// Pointer to a value's heap-layout words: the wide stack slot for >64
    /// values, or a freshly spilled temp ([spc, val] for four-value) for <=64.
    fn value_words_ptr(
        &mut self,
        b: &mut FunctionBuilder,
        key: VariableKey,
        vmap: &VgHashMap<VariableKey, Variable>,
        spc_map: &VgHashMap<VariableKey, Variable>,
        wide_map: &WideMap,
        cldctx: Value,
    ) -> Value {
        let size = self.gl.vars.size(key).get();
        if size > 64 {
            return wide_map[&key].addr(b, self.ptr, cldctx, 0);
        }
        let fv = key.mode() == LogicMode::FourValue;
        let words = if fv { 2 } else { 1 };
        let slot = b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            words * 8,
            3,
        ));
        if fv {
            let spc = b.use_var(spc_map[&key]);
            let val = b.use_var(vmap[&key]);
            b.ins().stack_store(self.ptr, spc, slot, 0);
            b.ins().stack_store(self.ptr, val, slot, 8);
        } else {
            let val = b.use_var(vmap[&key]);
            b.ins().stack_store(self.ptr, val, slot, 0);
        }
        b.ins().stack_addr(self.ptr, slot, 0)
    }

    /// Base pointers (heap-layout words) for a wide concat with a constant
    /// operand: the dst wide slot, the src operand (spilled to a temp slot if it
    /// is register-width — concat operand widths differ), and the immediate
    /// reserved in the runtime heap via `claim_constant` (baked in at init, so it
    /// is loaded rather than materialized as `iconst`s).
    #[expect(clippy::too_many_arguments)]
    fn concat_operand_ptrs(
        &mut self,
        b: &mut FunctionBuilder,
        dst: VariableKey,
        src: VariableKey,
        imm: &vogls_bits::Bits,
        vmap: &VgHashMap<VariableKey, Variable>,
        spc_map: &VgHashMap<VariableKey, Variable>,
        wide_map: &WideMap,
        params: &Params,
    ) -> (Value, Value, Value) {
        let dst_base = wide_map[&dst].addr(b, self.ptr, params.cldctx, 0);
        let src_base = self.value_words_ptr(b, src, vmap, spc_map, wide_map, params.cldctx);
        let imm_ref = self.heap_builder.claim_constant(dst.mode(), imm.clone());
        let imm_base = b
            .ins()
            .iadd_imm_u(params.heap_ptr, (imm_ref.offset.bit_offset / 8) as i64);
        (dst_base, src_base, imm_base)
    }

    /// Operand pointers for a wide binary-immediate reduction: the source's wide
    /// stack slot (the caller's op has a narrow arm, so `src` is always >64), and
    /// the immediate reserved as a heap constant (same `[spc, val]` layout the
    /// reduction reads).
    fn binop_imm_ptrs(
        &mut self,
        b: &mut FunctionBuilder,
        src: VariableKey,
        imm: &vogls_bits::Bits,
        mode: LogicMode,
        wide_map: &WideMap,
        params: &Params,
    ) -> (Value, Value) {
        let src_ptr = wide_map[&src].addr(b, self.ptr, params.cldctx, 0);
        let imm_ref = self.heap_builder.claim_constant(mode, imm.clone());
        let imm_ptr = b
            .ins()
            .iadd_imm_u(params.heap_ptr, (imm_ref.offset.bit_offset / 8) as i64);
        (src_ptr, imm_ptr)
    }

    /// Store an immediate's words to a temp slot (heap layout) and return a ptr.
    /// Used for the cold narrow `Power`/`RevPower` shim path, where the immediate
    /// is <= 64 bits: `claim_constant` would pack a small four-value value into a
    /// single word, but the shim reads the `[spc-word, val-word]` layout this
    /// always produces.
    fn materialize_imm_slot(
        &mut self,
        b: &mut FunctionBuilder,
        imm: &vogls_bits::Bits,
        is_fv: bool,
    ) -> Value {
        // The wide_binop shim reads four-value operands in `[spc-words,
        // val-words]` layout. A two-value immediate is fully known, so in a
        // four-value op it must be materialized with a leading all-ones spc
        // plane (masked to size) — otherwise the shim reads spc=0 => the whole
        // immediate is x, corrupting the result (e.g. `2**i` => x).
        let words: Vec<u64> = match imm.as_data_ref() {
            vogls_bits::BitsDataRef::InlineTv(v) => {
                if is_fv {
                    vec![mask_u64(imm.size().get()), v]
                } else {
                    vec![v]
                }
            }
            vogls_bits::BitsDataRef::SeparateTv(wds) => {
                if is_fv {
                    let sz = imm.size().get() as usize;
                    let mut out: Vec<u64> = (0..wds.len())
                        .map(|i| {
                            let bits = sz.saturating_sub(64 * i).min(64);
                            if bits >= 64 {
                                u64::MAX
                            } else {
                                (1u64 << bits) - 1
                            }
                        })
                        .collect();
                    out.extend_from_slice(wds);
                    out
                } else {
                    wds.to_vec()
                }
            }
            vogls_bits::BitsDataRef::InlineFv(spc, val) => vec![spc, val],
            vogls_bits::BitsDataRef::SeparateFv(wds) => wds.to_vec(),
        };
        let slot = b.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            (words.len().max(1) * 8) as u32,
            3,
        ));
        for (i, &wd) in words.iter().enumerate() {
            let c = b.ins().iconst(I64, wd as i64);
            b.ins().stack_store(self.ptr, c, slot, (i * 8) as i32);
        }
        b.ins().stack_addr(self.ptr, slot, 0)
    }

    /// `ShiftImm` (constant amount) via the `wide_binop` shim — handles any
    /// width and logic mode, including arithmetic shift right. The shim reads the
    /// amount as a `[known, amount]` word pair.
    #[expect(clippy::too_many_arguments)]
    fn emit_wide_shift_imm(
        &mut self,
        b: &mut FunctionBuilder,
        params: &Params,
        gl: &GlobalContext,
        op: ShiftImmOp,
        dst: VariableKey,
        src: VariableKey,
        amount: u32,
        vmap: &VgHashMap<VariableKey, Variable>,
        spc_map: &VgHashMap<VariableKey, Variable>,
        wide_map: &WideMap,
    ) {
        use crate::runtime::WideCode;
        let wcode = match op {
            ShiftImmOp::LogicalShiftLeft => WideCode::Lsl,
            ShiftImmOp::LogicalShiftRight => WideCode::Lsr,
            ShiftImmOp::ArithmeticShiftRight => WideCode::Asr,
        };
        let is_fv = dst.mode() == LogicMode::FourValue;
        let dsize = gl.vars.size(dst).get();
        let ssize = gl.vars.size(src).get();
        let src_ptr = self.value_words_ptr(b, src, vmap, spc_map, wide_map, params.cldctx);
        // amount operand: [known = 1, amount].
        let amt_slot =
            b.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 16, 3));
        let one = b.ins().iconst(I64, 1);
        let amtc = b.ins().iconst(I64, amount as i64);
        b.ins().stack_store(self.ptr, one, amt_slot, 0);
        b.ins().stack_store(self.ptr, amtc, amt_slot, 8);
        let amt_ptr = b.ins().stack_addr(self.ptr, amt_slot, 0);

        let dst_wide = dsize > 64;
        let dst_slot = if dst_wide {
            wide_map[&dst]
        } else {
            let words = if is_fv { 2 } else { 1 };
            WideLoc::Slot(b.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                words * 8,
                3,
            )))
        };
        let dst_ptr = dst_slot.addr(b, self.ptr, params.cldctx, 0);

        let cldctx = params.cldctx;
        let fnp = b.ins().load(
            self.ptr,
            mem(),
            cldctx,
            (layout::CTX_FN_TABLE + layout::FN_WIDE_BINOP) as i32,
        );
        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.extend(
            [
                types::I32,
                self.ptr,
                self.ptr,
                self.ptr,
                types::I32,
                types::I32,
            ]
            .map(AbiParam::new),
        );
        let sr = b.import_signature(sig);
        let opc = b.ins().iconst(types::I32, wcode.encode(is_fv) as i64);
        let dsc = b.ins().iconst(types::I32, dsize as i64);
        let ssc = b.ins().iconst(types::I32, ssize as i64);
        b.ins()
            .call_indirect(sr, fnp, &[opc, dst_ptr, src_ptr, amt_ptr, dsc, ssc]);

        if !dst_wide {
            if is_fv {
                let spc = wide_load(b, self.ptr, params.cldctx, dst_slot, 0);
                let val = wide_load(b, self.ptr, params.cldctx, dst_slot, 1);
                b.def_var(spc_map[&dst], spc);
                b.def_var(vmap[&dst], val);
            } else {
                let val = wide_load(b, self.ptr, params.cldctx, dst_slot, 0);
                b.def_var(vmap[&dst], val);
            }
        }
    }

    /// Lower a full-width `Drive`: poke-if-changed then store.
    /// Call the `real_op` transcendental shim via the FnTable.
    fn real_shim(
        &self,
        b: &mut FunctionBuilder,
        params: &Params,
        code: u32,
        a: Value,
        b2: Value,
    ) -> Value {
        let cldctx = params.cldctx;
        let fn_ptr = b.ins().load(
            self.ptr,
            mem(),
            cldctx,
            (layout::CTX_FN_TABLE + layout::FN_REAL_OP) as i32,
        );
        let code_c = b.ins().iconst(types::I32, code as i64);
        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.extend([types::I32, I64, I64].map(AbiParam::new));
        sig.returns.push(AbiParam::new(I64));
        let sr = b.import_signature(sig);
        let call = b.ins().call_indirect(sr, fn_ptr, &[code_c, a, b2]);
        b.inst_results(call)[0]
    }

    fn lower_drive_tv(
        &mut self,
        b: &mut FunctionBuilder,
        params: &Params,
        signal: SignalKey,
        src: Value,
        size: u32,
        offset: u32,
    ) -> DriveMask {
        let (href, rt, _mode) = self.info.heap_ref(signal);
        let bit = href.offset.bit_offset + offset as usize;
        let word = bit / 64;
        let shift = bit % 64;
        let crosses = shift + size as usize > 64;

        let oldw = b.ins().load(I64, mem(), params.heap_ptr, (word * 8) as i32);
        let old_field = if !crosses {
            let s = if shift == 0 {
                oldw
            } else {
                b.ins().ushr_imm_u(oldw, shift as i64)
            };
            maskv(b, s, size)
        } else {
            let w1 = b
                .ins()
                .load(I64, mem(), params.heap_ptr, ((word + 1) * 8) as i32);
            let lo = b.ins().ushr_imm_u(oldw, shift as i64);
            let hi = b.ins().ishl_imm_u(w1, (64 - shift) as i64);
            let comb = b.ins().bor(lo, hi);
            maskv(b, comb, size)
        };
        let changed = b.ins().icmp(IntCC::NotEqual, src, old_field);
        let moved = b.ins().bxor(src, old_field);
        let moved = maskv(b, moved, size);

        let guard = changed;

        let do_bb = b.create_block();
        let merge = b.create_block();
        b.ins().brif(guard, do_bb, &[], merge, &[]);

        b.switch_to_block(do_bb);
        let drive = Some(DrivenBits::Word {
            bit: offset,
            size,
            new: src,
            old: old_field,
            spc: None,
        });
        self.call_drive_signal(b, params, rt, drive);
        // store src into the signal field (read-modify-write).
        if !crosses {
            if size == 64 && shift == 0 {
                b.ins()
                    .store(mem(), src, params.heap_ptr, (word * 8) as i32);
            } else {
                let keep = !(mask_u64(size) << shift);
                let cur = b.ins().load(I64, mem(), params.heap_ptr, (word * 8) as i32);
                let cleared = b.ins().band_imm_u(cur, keep as i64);
                let masked_src = maskv(b, src, size);
                let placed = if shift == 0 {
                    masked_src
                } else {
                    b.ins().ishl_imm_u(masked_src, shift as i64)
                };
                let neww = b.ins().bor(cleared, placed);
                b.ins()
                    .store(mem(), neww, params.heap_ptr, (word * 8) as i32);
            }
        } else {
            let masked_src = maskv(b, src, size);
            let lo_size = 64 - shift;
            let cur0 = b.ins().load(I64, mem(), params.heap_ptr, (word * 8) as i32);
            let cleared0 = b.ins().band_imm_u(cur0, mask_u64(shift as u32) as i64);
            let placed0 = b.ins().ishl_imm_u(masked_src, shift as i64);
            let new0 = b.ins().bor(cleared0, placed0);
            b.ins()
                .store(mem(), new0, params.heap_ptr, (word * 8) as i32);
            let hi_size = size as usize - lo_size;
            let cur1 = b
                .ins()
                .load(I64, mem(), params.heap_ptr, ((word + 1) * 8) as i32);
            let cleared1 = b.ins().band_imm_u(cur1, (!mask_u64(hi_size as u32)) as i64);
            let hi_src = b.ins().ushr_imm_u(masked_src, lo_size as i64);
            let new1 = b.ins().bor(cleared1, hi_src);
            b.ins()
                .store(mem(), new1, params.heap_ptr, ((word + 1) * 8) as i32);
        }
        b.ins().jump(merge, &[]);
        b.switch_to_block(merge);
        DriveMask::Word(moved)
    }

    /// Full-width four-value drive: poke-if-changed + store.
    fn lower_drive_fv(
        &mut self,
        b: &mut FunctionBuilder,
        params: &Params,
        signal: SignalKey,
        src_val: Value,
        src_spc: Value,
        size: u32,
    ) -> DriveMask {
        let (href, rt, _) = self.info.heap_ref(signal);
        let heap = params.heap_ptr;
        let word = href.offset.bit_offset / 64;
        let shift = href.offset.bit_offset % 64;

        let do_bb = b.create_block();
        let merge = b.create_block();

        let moved;
        if size <= 32 {
            // Packed field of 2*size bits at [shift, shift + 2*size).
            let psize = 2 * size;
            let vh = b.ins().ishl_imm_u(src_val, size as i64);
            let new_packed = b.ins().bor(vh, src_spc);
            let oldw = b.ins().load(I64, mem(), heap, (word * 8) as i32);
            let of = if shift == 0 {
                oldw
            } else {
                b.ins().ushr_imm_u(oldw, shift as i64)
            };
            let old_field = maskv(b, of, psize);
            let changed = b.ins().icmp(IntCC::NotEqual, new_packed, old_field);
            // Both planes sit in this one word, so a bit moved if either of its halves did.
            let d = b.ins().bxor(new_packed, old_field);
            let dv = b.ins().ushr_imm_u(d, size as i64);
            let m = b.ins().bor(d, dv);
            moved = maskv(b, m, size);
            b.ins().brif(changed, do_bb, &[], merge, &[]);
            b.switch_to_block(do_bb);
            let old_spc = maskv(b, old_field, size);
            let old_val = b.ins().ushr_imm_u(old_field, size as i64);
            let drive = Some(DrivenBits::Word {
                bit: 0,
                size,
                new: src_val,
                old: old_val,
                spc: Some((src_spc, old_spc)),
            });
            self.call_drive_signal(b, params, rt, drive);
            if psize == 64 && shift == 0 {
                b.ins().store(mem(), new_packed, heap, (word * 8) as i32);
            } else {
                let keep = !(mask_u64(psize) << shift);
                let cur = b.ins().load(I64, mem(), heap, (word * 8) as i32);
                let cleared = b.ins().band_imm_u(cur, keep as i64);
                let placed = if shift == 0 {
                    new_packed
                } else {
                    b.ins().ishl_imm_u(new_packed, shift as i64)
                };
                let neww = b.ins().bor(cleared, placed);
                b.ins().store(mem(), neww, heap, (word * 8) as i32);
            }
        } else {
            // Split words: spc @ word, val @ word+1 (word-aligned).
            let old_spc = {
                let w = b.ins().load(I64, mem(), heap, (word * 8) as i32);
                maskv(b, w, size)
            };
            let old_val = {
                let w = b.ins().load(I64, mem(), heap, ((word + 1) * 8) as i32);
                maskv(b, w, size)
            };
            let c1 = b.ins().icmp(IntCC::NotEqual, src_spc, old_spc);
            let c2 = b.ins().icmp(IntCC::NotEqual, src_val, old_val);
            let changed = b.ins().bor(c1, c2);
            let ds = b.ins().bxor(src_spc, old_spc);
            let dv = b.ins().bxor(src_val, old_val);
            let m = b.ins().bor(ds, dv);
            moved = maskv(b, m, size);
            b.ins().brif(changed, do_bb, &[], merge, &[]);
            b.switch_to_block(do_bb);
            let drive = Some(DrivenBits::Word {
                bit: 0,
                size,
                new: src_val,
                old: old_val,
                spc: Some((src_spc, old_spc)),
            });
            self.call_drive_signal(b, params, rt, drive);
            let ms = maskv(b, src_spc, size);
            let mv = maskv(b, src_val, size);
            b.ins().store(mem(), ms, heap, (word * 8) as i32);
            b.ins().store(mem(), mv, heap, ((word + 1) * 8) as i32);
        }
        b.ins().jump(merge, &[]);
        b.switch_to_block(merge);
        DriveMask::Word(moved)
    }

    /// Inline the drive_signal body (mirrors build_drive_signal) at a drive
    /// site. With the active-region push no longer carrying a grow call, this
    /// body has no calls (absent plugins), so an inlined-drive TR stays leaf:
    /// no frame, no callee-save spills. Correct listener sets require the
    /// `collect_listeners` pre-pass to have run first.
    ///
    /// `drive` describes the write that got here, where the caller can: it is what lets an edge
    /// listener tell whether its bit moved, and which way. A drive that cannot produce it passes
    /// `None`, and a directed edge listener on such a signal is then unserviceable.
    fn call_drive_signal(
        &mut self,
        b: &mut FunctionBuilder,
        params: &Params,
        rt: RtSignalKey,
        drive: Option<DrivenBits<'_>>,
    ) {
        let idx = rt.as_usize();
        let lupdt = self.info.lupdt_indexes.get(&rt).copied();
        let listeners: Vec<(u32, FuncId, WatchEdge, u32)> = self.listeners[idx]
            .iter()
            .map(|l| (l.offset, l.target, l.edge, l.bit))
            .collect();
        assert!(
            drive.is_some()
                || listeners
                    .iter()
                    .all(|(_, _, e, _)| matches!(e, WatchEdge::Any { .. })),
            "Edge watch on a signal whose drive does not know the level it settles on"
        );
        let num_plugins = self.num_plugins;

        let (schedule, time, listening, last_active_time, cldctx) = (
            params.schedule,
            params.time,
            params.listening,
            params.last_active_time,
            params.cldctx,
        );

        if num_plugins > 0 {
            let plugins = b
                .ins()
                .load(self.ptr, mem(), cldctx, layout::CTX_PLUGINS as i32);
            let poke = b
                .ins()
                .load(self.ptr, mem(), cldctx, layout::CTX_PLUGIN_POKE as i32);
            let sig_ref = b.import_signature(self.sigs.plugin_poke.clone());
            let id = b.ins().iconst(I64, rt.as_u64() as i64);
            for i in 0..num_plugins {
                let pl = b.ins().iadd_imm_u(plugins, (i * PLUGIN_STATE_SIZE) as i64);
                b.ins().call_indirect(sig_ref, poke, &[pl, id]);
            }
        }

        if let Some(li) = lupdt {
            b.ins()
                .store(mem(), time, last_active_time, (li * 8) as i32);
        }

        for (offset, target, edge, watched_bit) in listeners {
            // @NOTE: A drive that does not reach the watched bit cannot have moved it, so there is
            // no edge for this listener and nothing to emit at all.
            if matches!(edge, WatchEdge::Posedge | WatchEdge::Negedge) {
                match drive.expect("Checked above") {
                    DrivenBits::Word { bit, size, .. } => {
                        if watched_bit < bit || watched_bit - bit >= size {
                            continue;
                        }
                    }
                    DrivenBits::Words { words, .. } => {
                        if !words.contains(&(watched_bit as usize / 64)) {
                            continue;
                        }
                    }
                }
            }

            let wake = b.create_block();
            let next = b.create_block();
            let w = b
                .ins()
                .load(I64, mem(), listening, ((offset / 64) * 8) as i32);
            let bit = b.ins().band_imm_u(w, 1i64 << (offset % 64));
            // @NOTE: The drive only reaches here once the value actually changed, so a bit that
            // settles high got there from low: the new level *is* the edge.
            let guard = match edge {
                WatchEdge::Any { .. } => bit,
                WatchEdge::Posedge | WatchEdge::Negedge => {
                    let armed = b.ins().icmp_imm_u(IntCC::NotEqual, bit, 0);
                    let invert = |b: &mut FunctionBuilder, v: Value| b.ins().bxor_imm_u(v, -1);
                    let (took_edge, shift) = match drive.expect("Checked above") {
                        DrivenBits::Word {
                            bit,
                            size,
                            new,
                            old,
                            spc,
                        } => {
                            let took = match spc {
                                // A four-valued bit can move between four levels, so the edge is
                                // worked out from both planes rather than read off the new value.
                                Some((new_spc, old_spc)) => {
                                    fv_edge_mask(b, edge, old_spc, old, new_spc, new)
                                }
                                None if size == 1 => {
                                    // @NOTE: A one-bit drive only gets here once it changed
                                    // something, and it has only the one bit to have changed: the
                                    // level it settles on is the edge, nothing further to check.
                                    match edge {
                                        WatchEdge::Posedge => new,
                                        _ => invert(b, new),
                                    }
                                }
                                None => {
                                    // @NOTE: A wider drive reaches here when *any* of its bits
                                    // changed, which says nothing about this one. A bit that held
                                    // its level across the write had no edge, whatever that level.
                                    let moved = b.ins().bxor(new, old);
                                    let level = match edge {
                                        WatchEdge::Posedge => new,
                                        _ => invert(b, new),
                                    };
                                    b.ins().band(moved, level)
                                }
                            };
                            (took, i64::from(watched_bit - bit))
                        }
                        // @NOTE: The drive already worked out which bits it moved, word by word,
                        // to decide whether to poke at all. The watched bit lives in exactly one
                        // of those words, so only that word's mask is of any interest here.
                        DrivenBits::Words { words, moved, new } => {
                            let i = words
                                .iter()
                                .position(|&w| w == watched_bit as usize / 64)
                                .expect("Checked above");
                            let level = match edge {
                                WatchEdge::Posedge => new[i],
                                _ => invert(b, new[i]),
                            };
                            let took = b.ins().band(moved[i], level);
                            (took, i64::from(watched_bit % 64))
                        }
                    };
                    let shifted = match shift {
                        0 => took_edge,
                        n => b.ins().ushr_imm_u(took_edge, n),
                    };
                    let bit0 = b.ins().band_imm_u(shifted, 1);
                    let matches = b.ins().icmp_imm_u(IntCC::NotEqual, bit0, 0);
                    b.ins().band(armed, matches)
                }
            };
            b.ins().brif(guard, wake, &[], next, &[]);
            b.switch_to_block(wake);
            let cleared = b.ins().bxor_imm_u(w, 1i64 << (offset % 64));
            b.ins()
                .store(mem(), cleared, listening, ((offset / 64) * 8) as i32);
            let active = ioff(b, self.ptr, schedule, layout::SCHED_ACTIVE);
            let fr = self.module.declare_func_in_func(target, b.func);
            let ta = b.ins().func_addr(self.ptr, fr);
            self.emit_push_inline(b, active, ta);
            b.ins().jump(next, &[]);
            b.switch_to_block(next);
        }
    }

    /// Store a `size`-bit (<=64) four-value (val, spc) pair into the signal's
    /// heap storage — packed (<=32) or split spc/val words (33..=64).
    fn fv_store(
        &self,
        b: &mut FunctionBuilder,
        heap: Value,
        href: HeapRef,
        size: u32,
        val: Value,
        spc: Value,
    ) {
        let word = href.offset.bit_offset / 64;
        let shift = href.offset.bit_offset % 64;
        if size <= 32 {
            let psize = 2 * size;
            let vh = b.ins().ishl_imm_u(val, size as i64);
            let packed = b.ins().bor(vh, spc);
            if psize == 64 && shift == 0 {
                b.ins().store(mem(), packed, heap, (word * 8) as i32);
            } else {
                let keep = !(mask_u64(psize) << shift);
                let cur = b.ins().load(I64, mem(), heap, (word * 8) as i32);
                let cleared = b.ins().band_imm_u(cur, keep as i64);
                let placed = if shift == 0 {
                    packed
                } else {
                    b.ins().ishl_imm_u(packed, shift as i64)
                };
                let neww = b.ins().bor(cleared, placed);
                b.ins().store(mem(), neww, heap, (word * 8) as i32);
            }
        } else {
            let ms = maskv(b, spc, size);
            let mv = maskv(b, val, size);
            b.ins().store(mem(), ms, heap, (word * 8) as i32);
            b.ins().store(mem(), mv, heap, ((word + 1) * 8) as i32);
        }
    }

    /// Partial/variable-offset drive into a wide (>64) signal, via the wide_drive
    /// shim (which does the read-modify-write and reports whether it changed).
    #[expect(clippy::too_many_arguments)]
    fn emit_wide_drive(
        &mut self,
        b: &mut FunctionBuilder,
        params: &Params,
        signal: SignalKey,
        src: VariableKey,
        offset: Value,
        off_known: Value,
        vmap: &VgHashMap<VariableKey, Variable>,
        spc_map: &VgHashMap<VariableKey, Variable>,
        wide_map: &WideMap,
    ) -> DriveMask {
        let (href, rt, mode) = self.info.heap_ref(signal);
        let heap = params.heap_ptr;
        let base_word = (href.offset.bit_offset / 64) as i64;
        let d_size = self.gl.signals[signal].size.get();
        let s_size = self.gl.vars.size(src).get();
        let is_fv = mode == LogicMode::FourValue;
        let src_ptr = self.value_words_ptr(b, src, vmap, spc_map, wide_map, params.cldctx);

        // @NOTE: The shim writes out of line, so what the signal held is gone once it returns.
        // Only the words an edge watch reads have to survive that, and usually there are none, so
        // this reloads just those -- rather than the whole signal -- on either side of the call.
        let watched_words: Vec<usize> = if is_fv {
            Vec::new()
        } else {
            let mut ws: Vec<usize> = self.listeners[rt.as_usize()]
                .iter()
                .filter(|l| l.edge.is_directed())
                .map(|l| l.bit as usize / 64)
                .collect();
            ws.sort_unstable();
            ws.dedup();
            ws
        };

        let do_bb = b.create_block();
        let merge = b.create_block();
        b.ins().brif(off_known, do_bb, &[], merge, &[]);
        b.switch_to_block(do_bb);
        let before: Vec<Value> = watched_words
            .iter()
            .map(|&w| {
                b.ins()
                    .load(I64, mem(), heap, ((base_word as usize + w) * 8) as i32)
            })
            .collect();

        let cldctx = params.cldctx;
        let fnp = b.ins().load(
            self.ptr,
            mem(),
            cldctx,
            (layout::CTX_FN_TABLE + layout::FN_WIDE_DRIVE) as i32,
        );
        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.extend(
            [
                self.ptr,
                types::I32,
                self.ptr,
                types::I32,
                types::I32,
                types::I32,
                types::I32,
            ]
            .map(AbiParam::new),
        );
        sig.returns.push(AbiParam::new(I64));
        let sr = b.import_signature(sig);
        let bw = b.ins().iconst(types::I32, base_word);
        let dsc = b.ins().iconst(types::I32, d_size as i64);
        let offc = b.ins().ireduce(types::I32, offset);
        let ssc = b.ins().iconst(types::I32, s_size as i64);
        let fvc = b.ins().iconst(types::I32, i64::from(is_fv));
        let call = b
            .ins()
            .call_indirect(sr, fnp, &[heap, bw, src_ptr, dsc, offc, ssc, fvc]);
        let changed = b.inst_results(call)[0];
        let ch = b.ins().icmp_imm_u(IntCC::NotEqual, changed, 0);

        let poke = ch;
        let drive_bb = b.create_block();
        b.ins().brif(poke, drive_bb, &[], merge, &[]);
        b.switch_to_block(drive_bb);
        let after: Vec<Value> = watched_words
            .iter()
            .map(|&w| {
                b.ins()
                    .load(I64, mem(), heap, ((base_word as usize + w) * 8) as i32)
            })
            .collect();
        let moved: Vec<Value> = before
            .iter()
            .zip(after.iter())
            .map(|(&x, &y)| b.ins().bxor(x, y))
            .collect();
        let drive = (!watched_words.is_empty()).then(|| DrivenBits::Words {
            words: &watched_words,
            moved: &moved,
            new: &after,
        });
        self.call_drive_signal(b, params, rt, drive);
        b.ins().jump(merge, &[]);
        b.switch_to_block(merge);
        // @NOTE: The shim writes out of line and reports only whether anything changed, so which
        // bits moved is not recoverable here without reading the whole signal back twice.
        DriveMask::Unknown
    }

    /// Extract `d_size` bits at bit `offset` from a wide source (pointed to by
    /// `src_ptr`, `s_size` bits) into `dst`, via the wide_slice shim. Used for
    /// wide Slice / SliceImm / Probe. `fill_with_x` (Slice) forces a four-value
    /// dst; an unknown offset yields all-x.
    #[expect(clippy::too_many_arguments)]
    fn emit_wide_slice(
        &mut self,
        b: &mut FunctionBuilder,
        params: &Params,
        dst: VariableKey,
        src_ptr: Value,
        offset: Value,
        off_known: Value,
        s_size: u32,
        src_is_fv: bool,
        fill_with_x: bool,
        vmap: &VgHashMap<VariableKey, Variable>,
        spc_map: &VgHashMap<VariableKey, Variable>,
        wide_map: &WideMap,
    ) {
        let d_size = self.gl.vars.size(dst).get();
        let dst_is_fv = src_is_fv || fill_with_x;
        let dnw = nwords(d_size) as usize;
        let dst_words = if dst_is_fv { 2 * dnw } else { dnw };
        let dst_wide = d_size > 64;
        let dst_slot = if dst_wide {
            wide_map[&dst]
        } else {
            WideLoc::Slot(b.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                (dst_words * 8) as u32,
                3,
            )))
        };
        let dst_ptr = dst_slot.addr(b, self.ptr, params.cldctx, 0);

        let do_bb = b.create_block();
        let zero_bb = b.create_block();
        let merge = b.create_block();
        b.ins().brif(off_known, do_bb, &[], zero_bb, &[]);

        b.switch_to_block(do_bb);
        let cldctx = params.cldctx;
        let fnp = b.ins().load(
            self.ptr,
            mem(),
            cldctx,
            (layout::CTX_FN_TABLE + layout::FN_WIDE_SLICE) as i32,
        );
        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.extend(
            [
                self.ptr,
                self.ptr,
                types::I32,
                types::I32,
                types::I32,
                types::I32,
                types::I32,
            ]
            .map(AbiParam::new),
        );
        let sr = b.import_signature(sig);
        let offc = b.ins().ireduce(types::I32, offset);
        let dsc = b.ins().iconst(types::I32, d_size as i64);
        let ssc = b.ins().iconst(types::I32, s_size as i64);
        let sfv = b.ins().iconst(types::I32, i64::from(src_is_fv));
        let fx = b.ins().iconst(types::I32, i64::from(fill_with_x));
        b.ins()
            .call_indirect(sr, fnp, &[dst_ptr, src_ptr, offc, dsc, ssc, sfv, fx]);
        b.ins().jump(merge, &[]);

        // Unknown offset -> all x (zero all dst words).
        b.switch_to_block(zero_bb);
        let z = b.ins().iconst(I64, 0);
        for i in 0..dst_words {
            wide_store(b, self.ptr, params.cldctx, dst_slot, i as u32, z);
        }
        b.ins().jump(merge, &[]);
        b.switch_to_block(merge);

        if !dst_wide {
            if dst_is_fv {
                let spc = wide_load(b, self.ptr, params.cldctx, dst_slot, 0);
                let val = wide_load(b, self.ptr, params.cldctx, dst_slot, 1);
                b.def_var(spc_map[&dst], spc);
                b.def_var(vmap[&dst], val);
            } else {
                let val = wide_load(b, self.ptr, params.cldctx, dst_slot, 0);
                b.def_var(vmap[&dst], val);
            }
        }
    }

    /// Partial drive: insert `s_size` bits of `src_val`(/`src_spc`) at bit offset
    /// `off` into a signal of width `d_size` (<=64), poking if the field changes.
    /// Handles constant offsets (Drive) and runtime offsets (DriveSlice); an
    /// unknown four-value offset (`off_known` false) suppresses the write.
    #[expect(clippy::too_many_arguments)]
    fn drive_partial(
        &mut self,
        b: &mut FunctionBuilder,
        params: &Params,
        signal: SignalKey,
        src_val: Value,
        src_spc: Option<Value>,
        s_size: u32,
        off: Value,
        off_known: Value,
    ) -> DriveMask {
        let (href, rt, mode) = self.info.heap_ref(signal);
        let d_size = self.gl.signals[signal].size.get();
        let heap = params.heap_ptr;
        let base_bit = href.offset.bit_offset;
        let do_bb = b.create_block();
        let merge = b.create_block();
        let moved;
        if mode == LogicMode::TwoValue {
            let cur = read_heap_field(b, heap, base_bit, d_size);
            // Mask to the field width: an insert whose bits land at/after d_size
            // (e.g. an out-of-range index like a[1] on a 1-bit reg) must be a
            // no-op, not leak bits or spuriously poke.
            let new = insert_bits(b, cur, src_val, off, s_size);
            let new = maskv(b, new, d_size);
            let changed = b.ins().icmp(IntCC::NotEqual, new, cur);
            let guard = b.ins().band(changed, off_known);
            moved = slice_moved(b, cur, new, off, off_known, s_size);
            b.ins().brif(guard, do_bb, &[], merge, &[]);
            b.switch_to_block(do_bb);
            // @NOTE: Where the insert landed is only known at run time, but it does not need
            // to be: `new` and `cur` are the whole field either way, so the bits that moved fall
            // out of comparing them whatever the offset turned out to be.
            let drive = Some(DrivenBits::Word {
                bit: 0,
                size: d_size,
                new,
                old: cur,
                spc: None,
            });
            self.call_drive_signal(b, params, rt, drive);
            write_heap_field(b, heap, base_bit, d_size, new);
        } else {
            let (cur_val, cur_spc) = fv_load(b, heap, href, d_size);
            let sspc = src_spc.unwrap_or(src_val);
            // Mask to the field width so out-of-range inserted bits neither leak
            // into neighbouring bits nor spuriously mark the field changed.
            let new_val = insert_bits(b, cur_val, src_val, off, s_size);
            let new_val = maskv(b, new_val, d_size);
            let new_spc = insert_bits(b, cur_spc, sspc, off, s_size);
            let new_spc = maskv(b, new_spc, d_size);
            let c1 = b.ins().icmp(IntCC::NotEqual, new_val, cur_val);
            let c2 = b.ins().icmp(IntCC::NotEqual, new_spc, cur_spc);
            let ch = b.ins().bor(c1, c2);
            let guard = b.ins().band(ch, off_known);
            let dv = b.ins().bxor(new_val, cur_val);
            let ds = b.ins().bxor(new_spc, cur_spc);
            let both = b.ins().bor(dv, ds);
            let zero = b.ins().iconst(I64, 0);
            moved = slice_moved(b, zero, both, off, off_known, s_size);
            b.ins().brif(guard, do_bb, &[], merge, &[]);
            b.switch_to_block(do_bb);
            let drive = Some(DrivenBits::Word {
                bit: 0,
                size: d_size,
                new: new_val,
                old: cur_val,
                spc: Some((new_spc, cur_spc)),
            });
            self.call_drive_signal(b, params, rt, drive);
            self.fv_store(b, heap, href, d_size, new_val, new_spc);
        }
        b.ins().jump(merge, &[]);
        b.switch_to_block(merge);
        DriveMask::Word(moved)
    }

    /// Inline instructions for "get next event or return" as a tailcall.
    fn tail_pop_next_or_return(&mut self, b: &mut FunctionBuilder, params: &Params) {
        self.pop_next_or_return(b, params, true);
    }

    /// Inline instructions for "get next event or return".
    ///
    /// It is important to do this inline as it is very hot and out-of-line may force preserving
    /// tail-ABI args in the prologue of each TR.
    fn pop_next_or_return(&mut self, b: &mut FunctionBuilder, params: &Params, tail: bool) {
        let blk_pop = b.create_block();
        let blk_return = b.create_block();
        b.set_cold_block(blk_return);

        let active = ioff(
            b,
            self.ptr,
            params.schedule,
            offset_of!(ScheduleT, active_region),
        );

        // If active.len != 0
        //   true  -> blk_pop
        //   false -> blk_return
        let active_len = b
            .ins()
            .load(I64, mem(), active, FfiVec::<EventT>::LEN_OFFSET as i32);
        b.ins().brif(active_len, blk_pop, &[], blk_return, &[]);

        b.switch_to_block(blk_pop);
        // active.length -= 1;
        let new_active_len = b.ins().iadd_imm_s(active_len, -1);
        b.ins().store(
            mem(),
            new_active_len,
            active,
            FfiVec::<EventT>::LEN_OFFSET as i32,
        );

        // next_event = active.ptr[active.length]
        let active_ptr = b
            .ins()
            .load(self.ptr, mem(), active, FfiVec::<EventT>::PTR_OFFSET as i32);
        let off = b
            .ins()
            .imul_imm_u(new_active_len, size_of::<EventT>() as i64);
        let elem = b.ins().iadd(active_ptr, off);
        let next_event = b.ins().load(self.ptr, mem(), elem, 0);

        let signature = b.import_signature(self.sigs.event.clone());
        if tail {
            b.ins()
                .return_call_indirect(signature, next_event, params.as_slice());
        } else {
            let call = b
                .ins()
                .call_indirect(signature, next_event, params.as_slice());
            let r = b.inst_results(call)[0];
            b.ins().return_(&[r]);
        }

        // No new active events, return.
        b.switch_to_block(blk_return);
        let zero = b.ins().iconst(types::I32, 0);
        b.ins().return_(&[zero]);
    }
}

/// `size_of::<RuntimePluginState>()` == `size_of::<Box<dyn RuntimePlugin>>()`
/// (a fat pointer). Used for plugin-array indexing in `drive_signal`.
const PLUGIN_STATE_SIZE: usize = std::mem::size_of::<vogls_runtime::plugins::RuntimePluginState>();

// ---------------------------------------------------------------------------
// Value helpers
// ---------------------------------------------------------------------------

fn mask_u64(size: u32) -> u64 {
    if size >= 64 {
        u64::MAX
    } else {
        (1u64 << size) - 1
    }
}

/// Number of 64-bit words needed for `size` bits.
fn nwords(size: u32) -> u32 {
    size.div_ceil(64)
}

fn var_words(size: VectorSize, mode: LogicMode) -> usize {
    let words = nwords(size.get()) as usize;
    match mode {
        LogicMode::TwoValue => words,
        LogicMode::FourValue => words * 2,
    }
}

/// Mask (as i64) for the top (most-significant) word of a `size`-bit value.
fn top_i64(size: u32) -> i64 {
    let r = size % 64;
    (if r == 0 { u64::MAX } else { (1u64 << r) - 1 }) as i64
}

fn mask_of(size: u32) -> i64 {
    mask_u64(size) as i64
}
fn maskv(b: &mut FunctionBuilder, v: Value, size: u32) -> Value {
    if size >= 64 {
        v
    } else {
        b.ins().band_imm_u(v, mask_of(size))
    }
}
fn maskvsbs(b: &mut FunctionBuilder, v: Value, size: SixBitSize) -> Value {
    if size == SixBitSize::N64 {
        v
    } else {
        b.ins().band_imm_u(v, size.mask(u64::MAX) as i64)
    }
}

fn ioff(b: &mut FunctionBuilder, _ptr: Type, base: Value, off: usize) -> Value {
    if off == 0 {
        base
    } else {
        b.ins().iadd_imm_u(base, off as i64)
    }
}

/// Insert `ins_size` low bits of `ins` at runtime bit offset `off` into `cur`.
fn insert_bits(
    b: &mut FunctionBuilder,
    cur: Value,
    ins: Value,
    off: Value,
    ins_size: u32,
) -> Value {
    let m = mask_of(ins_size);
    let mc = b.ins().iconst(I64, m);
    let mask_sh = b.ins().ishl(mc, off);
    let notm = b.ins().bnot(mask_sh);
    let cleared = b.ins().band(cur, notm);
    let insm = b.ins().band_imm_u(ins, m);
    let placed = b.ins().ishl(insm, off);
    b.ins().bor(cleared, placed)
}

/// Read a `size`-bit (<=64) field at absolute heap bit position `bit`.
fn read_heap_field(b: &mut FunctionBuilder, heap: Value, bit: usize, size: u32) -> Value {
    let word = bit / 64;
    let shift = bit % 64;
    let w0 = b.ins().load(I64, mem(), heap, (word * 8) as i32);
    if shift + size as usize <= 64 {
        let s = if shift == 0 {
            w0
        } else {
            b.ins().ushr_imm_u(w0, shift as i64)
        };
        maskv(b, s, size)
    } else {
        let w1 = b.ins().load(I64, mem(), heap, ((word + 1) * 8) as i32);
        let lo = b.ins().ushr_imm_u(w0, shift as i64);
        let hi = b.ins().ishl_imm_u(w1, (64 - shift) as i64);
        let comb = b.ins().bor(lo, hi);
        maskv(b, comb, size)
    }
}

/// Write a `size`-bit (<=64) field `val` at absolute heap bit position `bit` (RMW).
fn write_heap_field(b: &mut FunctionBuilder, heap: Value, bit: usize, size: u32, val: Value) {
    let word = bit / 64;
    let shift = bit % 64;
    let crosses = shift + size as usize > 64;
    if !crosses {
        if size == 64 && shift == 0 {
            b.ins().store(mem(), val, heap, (word * 8) as i32);
        } else {
            let keep = !(mask_u64(size) << shift);
            let cur = b.ins().load(I64, mem(), heap, (word * 8) as i32);
            let cleared = b.ins().band_imm_u(cur, keep as i64);
            let masked = maskv(b, val, size);
            let placed = if shift == 0 {
                masked
            } else {
                b.ins().ishl_imm_u(masked, shift as i64)
            };
            let neww = b.ins().bor(cleared, placed);
            b.ins().store(mem(), neww, heap, (word * 8) as i32);
        }
    } else {
        let masked = maskv(b, val, size);
        let lo_size = 64 - shift;
        let cur0 = b.ins().load(I64, mem(), heap, (word * 8) as i32);
        let cleared0 = b.ins().band_imm_u(cur0, mask_u64(shift as u32) as i64);
        let placed0 = b.ins().ishl_imm_u(masked, shift as i64);
        let new0 = b.ins().bor(cleared0, placed0);
        b.ins().store(mem(), new0, heap, (word * 8) as i32);
        let hi_size = size as usize - lo_size;
        let cur1 = b.ins().load(I64, mem(), heap, ((word + 1) * 8) as i32);
        let cleared1 = b.ins().band_imm_u(cur1, (!mask_u64(hi_size as u32)) as i64);
        let hi_src = b.ins().ushr_imm_u(masked, lo_size as i64);
        let new1 = b.ins().bor(cleared1, hi_src);
        b.ins().store(mem(), new1, heap, ((word + 1) * 8) as i32);
    }
}

/// Extract a `d_size`-bit four-value field at runtime bit offset `off` from a
/// source held as `src_nwords` value words at `val_ptr` (and, for four-value
/// sources, `src_nwords` special words at `spc_ptr`). Bits at or past `s_size`,
/// and the whole field when `off_known` is false, read as x. Shared by the
/// variable-offset `Slice` (wide source) and `ProbeSlice` (heap source); mirrors
/// `vogls-codegen-c/src/slice.rs`. dst is always four-value.
#[expect(clippy::too_many_arguments)]
fn dyn_slice_read(
    b: &mut FunctionBuilder,
    val_ptr: Value,
    spc_ptr: Option<Value>,
    off: Value,
    off_known: Value,
    s_size: u32,
    d_size: u32,
    base_bit: u32,
) -> (Value, Value) {
    let d_mask = mask_of(d_size);
    // Bit 0 of the field sits `base_bit` bits into the first word (a signal need
    // not be 64-bit-word-aligned), so logical offset `off` reads at absolute bit
    // `off + base_bit`; the source spans `span` words from `base`.
    let span = nwords(base_bit + s_size);
    let maxw = b.ins().iconst(I64, (span - 1) as i64);
    // Funnel-read a 64-bit window at bit offset `off` from words at `base`.
    let funnel = |b: &mut FunctionBuilder, base: Value| -> Value {
        // Clamp the word index so an out-of-range `off` (e.g. a negative index
        // such as `a[-1]` => off = 0xFFFF_FFFF) can never load out of bounds.
        // The `oob` select at the end turns the (then-garbage) result into x.
        let pos = if base_bit == 0 {
            off
        } else {
            b.ins().iadd_imm_u(off, base_bit as i64)
        };
        let word_raw = b.ins().ushr_imm_u(pos, 6); // (off + base_bit) / 64
        let word = b.ins().umin(word_raw, maxw); // clamp into [0, span-1]
        let bit = b.ins().band_imm_u(pos, 63); // (off + base_bit) % 64
        let byte = b.ins().ishl_imm_u(word, 3); // word * 8
        let lo_addr = b.ins().iadd(base, byte);
        let lo = b.ins().load(I64, mem(), lo_addr, 0);
        // Next word, also clamped into range so the load never goes out of bounds.
        let next = b.ins().iadd_imm_u(word, 1);
        let clamped = b.ins().umin(next, maxw);
        let nbyte = b.ins().ishl_imm_u(clamped, 3);
        let hi_addr = b.ins().iadd(base, nbyte);
        let hi = b.ins().load(I64, mem(), hi_addr, 0);
        let lo_sh = b.ins().ushr(lo, bit);
        let c64 = b.ins().iconst(I64, 64);
        let inv = b.ins().isub(c64, bit);
        let hi_sh = b.ins().ishl(hi, inv);
        // Only fold in the high word when bit != 0 and the next word is in range.
        let bit_nz = b.ins().icmp_imm_u(IntCC::NotEqual, bit, 0);
        let in_b = b
            .ins()
            .icmp_imm_u(IntCC::UnsignedLessThan, next, span as i64);
        let cond = b.ins().band(bit_nz, in_b);
        let zero = b.ins().iconst(I64, 0);
        let hi_c = b.ins().select(cond, hi_sh, zero);
        b.ins().bor(lo_sh, hi_c)
    };
    let val_raw = funnel(b, val_ptr);
    let val_m = b.ins().band_imm_u(val_raw, d_mask);
    let diff = s_size - d_size;
    let spc_raw = if let Some(sp) = spc_ptr {
        let f = funnel(b, sp);
        b.ins().band_imm_u(f, d_mask)
    } else {
        // Two-value source: known in-range mask = off<=diff ? mask : mask>>(off-diff).
        let le = b
            .ins()
            .icmp_imm_u(IntCC::UnsignedLessThanOrEqual, off, diff as i64);
        let mc = b.ins().iconst(I64, d_mask);
        let over = b.ins().iadd_imm_u(off, -(diff as i64));
        let sh = b.ins().ushr(mc, over);
        b.ins().select(le, mc, sh)
    };
    let zero = b.ins().iconst(I64, 0);
    let oob = b
        .ins()
        .icmp_imm_u(IntCC::UnsignedGreaterThanOrEqual, off, s_size as i64);
    let v1 = b.ins().select(oob, zero, val_m);
    let val = b.ins().select(off_known, v1, zero);
    let s1 = b.ins().select(oob, zero, spc_raw);
    let spc = b.ins().select(off_known, s1, zero);
    (val, spc)
}

// ---------------------------------------------------------------------------
// Four-value helpers — (val, spc) pair. spc bit = 1 means KNOWN; spc=0 special.
// (0,0)=x  (0,1)=z  (1,0)=0  (1,1)=1.  Inputs are assumed masked to `size`.
// ---------------------------------------------------------------------------

/// Load a four-value signal from the heap into `(val, spc)`.
fn fv_load(b: &mut FunctionBuilder, heap: Value, href: HeapRef, size: u32) -> (Value, Value) {
    let word = href.offset.bit_offset / 64;
    let shift = href.offset.bit_offset % 64;
    if size <= 32 {
        let loaded = b.ins().load(I64, mem(), heap, (word * 8) as i32);
        let shifted = if shift == 0 {
            loaded
        } else {
            b.ins().ushr_imm_u(loaded, shift as i64)
        };
        let field = maskv(b, shifted, 2 * size);
        let spc = maskv(b, field, size);
        let valh = b.ins().ushr_imm_u(field, size as i64);
        let val = maskv(b, valh, size);
        (val, spc)
    } else {
        let ws = b.ins().load(I64, mem(), heap, (word * 8) as i32);
        let spc = maskv(b, ws, size);
        let wv = b.ins().load(I64, mem(), heap, ((word + 1) * 8) as i32);
        let val = maskv(b, wv, size);
        (val, spc)
    }
}

pub fn compile<'a>(
    gl: &'a GlobalContext,
    info: SignalInfo<'a>,
    heap_builder: &'a mut HeapBuilder,
    num_plugins: usize,
    emit_clif: Option<Arc<Mutex<dyn io::Write + Send + Sync>>>,
    emit_disassembly: Option<Arc<Mutex<dyn io::Write + Send + Sync>>>,
) -> Result<Compiled, String> {
    let num_signals = info.signal_to_heap.len();
    let mut c = Compiler::new(
        num_signals,
        num_plugins,
        WatchMap::new(&gl.bbs),
        gl,
        info,
        heap_builder,
        emit_clif,
        emit_disassembly,
    );
    // drive_fn ids, filled below.
    let mut fb = FunctionBuilderContext::new();

    c.entry = c.declare("empty_active_event_queue", &c.sigs.entry.clone());

    let event_sig = c.sigs.event.clone();
    let mut procs = Vec::new();
    for (pi, (_k, process)) in gl.processes.iter().enumerate() {
        let mut fst = None;
        for (ti, tr) in process.regions.iter().enumerate() {
            let fid = c.declare(&format!("tr_{pi}_{ti}"), &event_sig);
            fst.get_or_insert(fid);
            c.tr_funcs.insert(*tr, fid);
        }
        procs.push(fst.unwrap());
    }
    c.build_entry(&mut fb);

    // Listeners come straight from the watch map, which has already numbered every watch and
    // sorted them by signal, so a drive site can inline its complete wake set.
    c.num_listening = c.watch_map.num_watches() as u32;
    let listeners: Vec<(usize, Listener)> = c
        .watch_map
        .watchers()
        .iter()
        .filter_map(|(condition, index)| {
            // @NOTE: The watch map covers every block in the slot map, and some belong to no
            // process: the gate-level uart-aes bench leaves exactly one temporal region detached,
            // at every optimization level, so this is not the optimizer dropping it. A watch whose
            // target region no process lists can never be armed, so nothing can wake it and there
            // is no function to wake it into. It keeps its index -- and so its bit in `listening`
            // -- which just goes unused. Where those regions come from is still unexplained.
            let target = *c.tr_funcs.get(&c.watch_map.watch_target(*index))?;
            let rt = c.info.rt_signal_map[&condition.signal];
            let listener = Listener {
                offset: *index as u32,
                target,
                edge: condition.edge,
                bit: condition.offset,
            };
            Some((rt.as_usize(), listener))
        })
        .collect();
    for (rt, listener) in listeners {
        c.listeners[rt].push(listener);
    }

    // A process is "standing" (armed but not run at t=0) only if at least one watcher does NOT
    // trigger a t=0 poke -- matches the bytecode filter. Its arming Watch is the first one whose
    // watch set equals the standing set, in the order `build_tr` discovers them, so this walk has
    // to keep visiting blocks in that order even though the listeners no longer come from it.
    for (pi, (_k, process)) in gl.processes.iter().enumerate() {
        let Some(standing) = process.standing.as_deref().filter(|conditions| {
            conditions
                .iter()
                .any(|c| !gl.signals[c.signal].triggers_t0_poke())
        }) else {
            continue;
        };
        'process: for tr in process.regions.iter() {
            let mut seen = vogls_utils::VgHashSet::default();
            seen.insert(tr.entry());
            let mut order = vec![tr.entry()];
            let mut stack = vec![tr.entry()];
            while let Some(k) = stack.pop() {
                c.gl.bbs[k].terminator.for_each_non_temporal_bb(|s| {
                    if seen.insert(s) {
                        order.push(s);
                        stack.push(s);
                    }
                });
            }
            for &k in &order {
                let BasicBlockTerminator::Watch(_, conditions) = &c.gl.bbs[k].terminator else {
                    continue;
                };
                if standing.len() == conditions.len()
                    && standing.iter().zip(conditions.iter()).all(|(a, b)| a == b)
                {
                    c.standing_procs.insert(pi);
                    c.standing_arm_offsets
                        .push(c.watch_map.get_watch_index(k) as u32);
                    break 'process;
                }
            }
        }
    }

    // Lower TR bodies.
    for (pi, (_k, process)) in gl.processes.iter().enumerate() {
        let mut bb_phis = VgHashMap::default();
        let mut stack = Vec::new();
        let mut seen = vogls_utils::VgHashSet::default();
        vogls_codegen::insert_bb_phis(&process.regions, gl, &mut stack, &mut seen, &mut bb_phis);
        for (ti, tr) in process.regions.iter().enumerate() {
            c.build_tr(&mut fb, pi, ti, tr.entry(), &bb_phis);
        }
    }

    // These functions provide tailcall-able variants for `Wait`, `VariableWait` and `WaitRegion`
    // when the event doesn't fit in the capacity anymore.
    if let Some(func_id) = c.wait_region_grow {
        let grow_sig = c.sigs.grow.clone();
        let mut ctx = c.module.make_context();
        ctx.func.signature = c.sigs.event.clone();
        ctx.func.name = UserFuncName::user(0, func_id.as_u32());
        {
            let mut b = FunctionBuilder::new(&mut ctx.func, &mut fb);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let params = Params::from_block_params(&mut b, entry);

            // region_vec = &schedule->regions[pending_region - 1]
            let region = b
                .ins()
                .load(I64, mem(), params.cldctx, layout::CTX_PENDING_REGION as i32);
            let regions_base =
                b.ins()
                    .load(c.ptr, mem(), params.schedule, layout::SCHED_REGIONS as i32);
            let idx = b.ins().iadd_imm_s(region, -1);
            let off = b.ins().imul_imm_u(idx, size_of::<FfiVec<EventT>>() as i64);
            let region_vec = b.ins().iadd(regions_base, off);

            // (*region_vec->grow)(region_vec)
            let grow_fn = b.ins().load(
                c.ptr,
                mem(),
                region_vec,
                FfiVec::<EventT>::GROW_OFFSET as i32,
            );
            let gr = b.import_signature(grow_sig);
            b.ins().call_indirect(gr, grow_fn, &[region_vec]);

            let event = b.ins().load(
                c.ptr,
                mem(),
                params.cldctx,
                layout::CTX_PENDING_EVENT as i32,
            );
            c.emit_push_inline(&mut b, region_vec, event);

            c.tail_pop_next_or_return(&mut b, &params);
            b.seal_all_blocks();
            b.finalize(c.fe);
        }
        if let Some(writer) = c.emit_clif.as_mut() {
            let mut writer = writer.lock().unwrap();
            writeln!(
                writer,
                "=== grow_and_push_in_region_then_next_event ===\n{}",
                ctx.func.display()
            )
            .unwrap();
        }
        c.module.define_function(func_id, &mut ctx).unwrap();
        c.module.clear_context(&mut ctx);
    }
    if let Some(func_id) = c.wait_time_grow {
        let grow_sig = c.sigs.grow.clone();
        let mut ctx = c.module.make_context();
        ctx.func.signature = c.sigs.event.clone();
        ctx.func.name = UserFuncName::user(0, func_id.as_u32());
        {
            let mut b = FunctionBuilder::new(&mut ctx.func, &mut fb);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let params = Params::from_block_params(&mut b, entry);

            let future = ioff(&mut b, c.ptr, params.schedule, layout::SCHED_FUTURE);

            // (*future->grow)(future)
            let grow_fn = b
                .ins()
                .load(c.ptr, mem(), future, FfiVec::<EventT>::GROW_OFFSET as i32);
            let gr = b.import_signature(grow_sig);
            b.ins().call_indirect(gr, grow_fn, &[future]);

            let event = b.ins().load(
                c.ptr,
                mem(),
                params.cldctx,
                layout::CTX_PENDING_EVENT as i32,
            );
            let time = b
                .ins()
                .load(I64, mem(), params.cldctx, layout::CTX_PENDING_TIME as i32);
            let len = b
                .ins()
                .load(I64, mem(), future, FfiVec::<EventT>::LEN_OFFSET as i32);
            c.emit_push_future_inline_at(&mut b, params.schedule, future, event, time, len);

            c.tail_pop_next_or_return(&mut b, &params);
            b.seal_all_blocks();
            b.finalize(c.fe);
        }
        if let Some(writer) = c.emit_clif.as_mut() {
            let mut writer = writer.lock().unwrap();
            writeln!(
                writer,
                "=== grow_and_push_future_then_next_event ===\n{}",
                ctx.func.display()
            )
            .unwrap();
        }
        c.module.define_function(func_id, &mut ctx).unwrap();
        c.module.clear_context(&mut ctx);
    }

    c.module.finalize_definitions().unwrap();

    // Flatten the per-signal listener sets into CSR form for `ClifWatchers`.
    let mut watch_offsets = Vec::with_capacity(c.listeners.len() + 1);
    let mut watch_entries = Vec::new();
    watch_offsets.push(0u32);
    for sig_listeners in &c.listeners {
        for l in sig_listeners {
            watch_entries.push((l.offset, l.target));
        }
        watch_offsets.push(watch_entries.len() as u32);
    }

    Ok(Compiled {
        module: c.module,
        entry: c.entry,
        procs,
        watch_offsets,
        watch_entries,
        num_listening: c.num_listening as usize,
        dyn_fmt_strs: c.dyn_fmt_strs,
        read_mems: c.read_mems,
        time_fmts: c.time_fmts,
        standing_procs: c.standing_procs,
        standing_arm_offsets: c.standing_arm_offsets,
    })
}

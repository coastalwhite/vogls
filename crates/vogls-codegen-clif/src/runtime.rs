use std::ffi::c_void;
use std::io::{Write as _, stderr, stdout};
use std::mem::ManuallyDrop;
use std::ptr::NonNull;

use crate::ffi::FfiVec;
use cranelift_jit::JITModule;
use vogls_codegen::HeapRef;
use vogls_ir::dyn_format_string::DynFormatString;
use vogls_ir::{Bits, GlobalContext, Mode, ReadMem, VectorSize};
use vogls_runtime::plugins::{RuntimePlugin, RuntimePluginState};
use vogls_runtime::{RtSignalKey, RuntimeState, SimulationIo};
use vogls_utils::{SyncWrapper, TableKey};

type Time = u64;
type HeapPtr = Option<NonNull<u64>>;
type Listening = Option<NonNull<u64>>;
type LastActiveTime = Option<NonNull<u64>>;

/// Field offsets / sizes of the `#[repr(C)]` ABI structs, computed from the
/// actual layouts so the JIT lowering can emit struct field accesses without
/// hard-coding (and without drifting from the Rust definitions). A child module
/// can read the private fields of its ancestor module, so `offset_of!` works
/// here even though the struct fields are private.
pub mod layout {
    use super::{ColdContextT, EventT, FnTable, ScheduleT, TimedEventT};
    use std::mem::{offset_of, size_of};

    /// Pointer / word size in bytes on the target (JIT == host == 64-bit here).
    pub const WORD: usize = size_of::<usize>();

    pub const EVENT_SIZE: usize = size_of::<EventT>();
    pub const TIMED_EVENT_SIZE: usize = size_of::<TimedEventT>();
    pub const TIMED_EVENT_EVENT: usize = offset_of!(TimedEventT, event);
    pub const TIMED_EVENT_TIME: usize = offset_of!(TimedEventT, time);

    pub const SCHED_ACTIVE: usize = offset_of!(ScheduleT, active_region);
    pub const SCHED_REGIONS: usize = offset_of!(ScheduleT, regions);
    pub const SCHED_FUTURE: usize = offset_of!(ScheduleT, future);
    pub const SCHED_NEXT_TIME: usize = offset_of!(ScheduleT, next_time);

    pub const CTX_FN_TABLE: usize = offset_of!(ColdContextT, fn_table);
    pub const CTX_FMT_STRS: usize = offset_of!(ColdContextT, fmt_strs);
    pub const CTX_PLUGINS: usize = offset_of!(ColdContextT, plugins);
    pub const CTX_PLUGIN_POKE: usize = offset_of!(ColdContextT, plugin_poke_signal);
    pub const CTX_HEAP_LEN: usize = offset_of!(ColdContextT, heap_len);
    pub const CTX_READMEMS: usize = offset_of!(ColdContextT, readmems);
    pub const CTX_READMEM: usize = offset_of!(ColdContextT, readmem);
    pub const CTX_FST_POKE: usize = offset_of!(ColdContextT, fst_poke);
    pub const CTX_ICOUNT: usize = offset_of!(ColdContextT, icount);
    pub const CTX_STDOUT: usize = offset_of!(ColdContextT, stdout);
    pub const CTX_STDERR: usize = offset_of!(ColdContextT, stderr);

    pub const BITSREF_SIZEOF: usize = size_of::<super::BitsRefT>();
    pub const BITSREF_SIZE_OFF: usize = offset_of!(super::BitsRefT, size);
    pub const BITSREF_MODE_OFF: usize = offset_of!(super::BitsRefT, mode);
    pub const BITSREF_PTR_OFF: usize = offset_of!(super::BitsRefT, ptr);
    pub const DYN_FMT_STRING_SIZEOF: usize =
        size_of::<vogls_ir::dyn_format_string::DynFormatString>();

    pub const FN_FMT: usize = offset_of!(FnTable, fmt);
    pub const FN_RTL_UNIFORM: usize = offset_of!(FnTable, rtl_dist_uniform);
    pub const FN_RTL_NORMAL: usize = offset_of!(FnTable, rtl_dist_normal);
    pub const FN_RTL_EXPONENTIAL: usize = offset_of!(FnTable, rtl_dist_exponential);
    pub const FN_RTL_POISSON: usize = offset_of!(FnTable, rtl_dist_poisson);
    pub const FN_RTL_CHI_SQUARE: usize = offset_of!(FnTable, rtl_dist_chi_square);
    pub const FN_RTL_T: usize = offset_of!(FnTable, rtl_dist_t);
    pub const FN_RTL_ERLANG: usize = offset_of!(FnTable, rtl_dist_erlang);
    pub const FN_REAL_OP: usize = offset_of!(FnTable, real_op);
    pub const FN_WIDE_BINOP: usize = offset_of!(FnTable, wide_binop);
    pub const FN_WIDE_DRIVE: usize = offset_of!(FnTable, wide_drive);
    pub const FN_WIDE_SLICE: usize = offset_of!(FnTable, wide_slice);

    /// Size of a `readmems` entry: `(HeapRef, ReadMem)`.
    pub const READMEM_ENTRY_SIZE: usize = size_of::<(super::HeapRef, vogls_ir::ReadMem)>();
}

/// Return value of a compiled event/entry function.
///
/// * `0`: keep running (bail out to the driver loop).
/// * `1..=255`: exit the simulation with code `value - 1`.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct ReturnValue(std::ffi::c_int);

impl ReturnValue {
    pub const CONTINUE: Self = Self(0);
    pub const STOP: Self = Self(1);

    fn should_exit(self) -> bool {
        self.0 > 0
    }

    fn is_ok(self) -> bool {
        self.0 <= 1
    }
}

/// The entry trampoline: pops one event off the active region and runs it (the
/// compiled code chains the rest itself via tail calls).
pub type EmptyActiveEventQueueFn = extern "C" fn(
    HeapPtr,
    NonNull<ScheduleT>,
    Time,
    Listening,
    LastActiveTime,
    NonNull<ColdContextT>,
) -> ReturnValue;

/// A per-signal "poke" routine used by [`ClifDesign::poke_signal`].
pub type DriveFn =
    extern "C" fn(NonNull<ScheduleT>, Time, Listening, LastActiveTime, NonNull<ColdContextT>);

#[repr(C)]
pub struct BitsRefT {
    size: u32,
    mode: u8,
    ptr: NonNull<u64>,
}

#[repr(C)]
pub struct FnTable {
    fmt: extern "C" fn(
        NonNull<Box<dyn std::io::Write + Send + Sync>>,
        *const DynFormatString,
        *const BitsRefT,
    ),

    rtl_dist_uniform: extern "C" fn(
        seed: i32,
        start: i32,
        end: i32,
        NonNull<Box<dyn std::io::Write + Send + Sync>>,
    ) -> u64,
    rtl_dist_normal: extern "C" fn(
        seed: i32,
        mean: i32,
        df: i32,
        NonNull<Box<dyn std::io::Write + Send + Sync>>,
    ) -> u64,
    rtl_dist_exponential:
        extern "C" fn(seed: i32, mean: i32, NonNull<Box<dyn std::io::Write + Send + Sync>>) -> u64,
    rtl_dist_poisson:
        extern "C" fn(seed: i32, mean: i32, NonNull<Box<dyn std::io::Write + Send + Sync>>) -> u64,
    rtl_dist_chi_square:
        extern "C" fn(seed: i32, dof: i32, NonNull<Box<dyn std::io::Write + Send + Sync>>) -> u64,
    rtl_dist_t:
        extern "C" fn(seed: i32, dof: i32, NonNull<Box<dyn std::io::Write + Send + Sync>>) -> u64,
    rtl_dist_erlang: extern "C" fn(
        seed: i32,
        k_stage: i32,
        mean: i32,
        NonNull<Box<dyn std::io::Write + Send + Sync>>,
    ) -> u64,

    /// Transcendental real ops, dispatched by [`real_code`] on the f64 bit
    /// patterns `a`/`b` (b unused for unary ops).
    real_op: extern "C" fn(u32, u64, u64) -> u64,

    /// Wide / cold binary ops, dispatched by [`wide_code`] on heap-layout word
    /// arrays. `(op, dst, lhs, rhs, dsize, ssize)`.
    wide_binop: extern "C" fn(u32, *mut u64, *const u64, *const u64, u32, u32),

    /// Partial write into a wide signal: (heap, base_word, src, dsize, offset, ssize, is_fv) -> changed.
    wide_drive: extern "C" fn(*mut u64, u32, *const u64, u32, u32, u32, u32) -> u64,

    /// Extract dsize bits at offset from a wide source: (dst, src, offset, dsize, ssize, src_is_fv, fill_with_x).
    wide_slice: extern "C" fn(*mut u64, *const u64, u32, u32, u32, u32, u32),
}

/// Op codes for the [`real_op`] shim, shared with the lowering.
pub mod real_code {
    pub const POW: u32 = 0;
    pub const LN: u32 = 1;
    pub const LOG10: u32 = 2;
    pub const EXP: u32 = 3;
    pub const SIN: u32 = 4;
    pub const COS: u32 = 5;
    pub const TAN: u32 = 6;
    pub const ASIN: u32 = 7;
    pub const ACOS: u32 = 8;
    pub const ATAN: u32 = 9;
    pub const SINH: u32 = 10;
    pub const COSH: u32 = 11;
    pub const TANH: u32 = 12;
    pub const ASINH: u32 = 13;
    pub const ACOSH: u32 = 14;
    pub const ATANH: u32 = 15;
    pub const ATAN2: u32 = 16;
    pub const HYPOT: u32 = 17;
}

extern "C" fn real_op(op: u32, a: u64, b: u64) -> u64 {
    use real_code as c;
    let x = f64::from_bits(a);
    let y = f64::from_bits(b);
    let r = match op {
        c::POW => x.powf(y),
        c::LN => x.ln(),
        c::LOG10 => x.log10(),
        c::EXP => x.exp(),
        c::SIN => x.sin(),
        c::COS => x.cos(),
        c::TAN => x.tan(),
        c::ASIN => x.asin(),
        c::ACOS => x.acos(),
        c::ATAN => x.atan(),
        c::SINH => x.sinh(),
        c::COSH => x.cosh(),
        c::TANH => x.tanh(),
        c::ASINH => x.asinh(),
        c::ACOSH => x.acosh(),
        c::ATANH => x.atanh(),
        c::ATAN2 => x.atan2(y),
        c::HYPOT => x.hypot(y),
        _ => f64::NAN,
    };
    r.to_bits()
}

/// Op codes for the [`wide_binop`] shim. The value passed is `op * 2 + is_fv`.
pub mod wide_code {
    pub const ADD: u32 = 0;
    pub const SUB: u32 = 1;
    pub const MUL: u32 = 2;
    pub const DIV: u32 = 3;
    pub const MOD: u32 = 4;
    pub const POW: u32 = 5;
    pub const LSL: u32 = 6;
    pub const LSR: u32 = 7;
    pub const ASR: u32 = 8;
    pub const ULE: u32 = 9;
    pub const MIN: u32 = 10;
    pub const MAX: u32 = 11;

    #[inline]
    pub const fn code(op: u32, is_fv: bool) -> u32 {
        op * 2 + is_fv as u32
    }
}

/// Wide (and cold narrow) binary ops, delegated to the `vogls-bits` word-slice
/// routines the bytecode interpreter also uses. Operands/dst are pointers to
/// heap-layout word arrays (two-value: `n` words; four-value: `n` special words
/// then `n` value words). For shifts, `rhs` points to a single word holding the
/// shift amount. `dsize`/`ssize` are the dst / operand bit widths.
extern "C" fn wide_binop(
    op: u32,
    dst: *mut u64,
    lhs: *const u64,
    rhs: *const u64,
    dsize: u32,
    ssize: u32,
) {
    use vogls_bits::arithmetic as a;
    use vogls_bits::comparison as cmp;
    use vogls_bits::shift as sh;
    use wide_code as w;

    let is_fv = (op & 1) != 0;
    let operation = op >> 1;
    let ds = VectorSize::new(dsize).unwrap();
    let ss = VectorSize::new(ssize).unwrap();
    let snw = (ssize as usize).div_ceil(64);
    let dnw = (dsize as usize).div_ceil(64);
    let sw = if is_fv { 2 * snw } else { snw };
    let dw = if is_fv { 2 * dnw } else { dnw };

    // SAFETY: the JIT passes valid stack-slot pointers sized per the layout above.
    unsafe {
        let d = std::slice::from_raw_parts_mut(dst, dw);
        let l = std::slice::from_raw_parts(lhs, sw);
        match operation {
            w::ADD => {
                let r = std::slice::from_raw_parts(rhs, sw);
                if is_fv {
                    a::fv_addition(d, l, r, ds)
                } else {
                    a::tv_addition(d, l, r, ds)
                }
            }
            w::SUB => {
                let r = std::slice::from_raw_parts(rhs, sw);
                if is_fv {
                    a::fv_subtraction(d, l, r, ds)
                } else {
                    a::tv_subtraction(d, l, r, ds)
                }
            }
            w::MUL => {
                let r = std::slice::from_raw_parts(rhs, sw);
                if is_fv {
                    a::fv_multiplication(d, l, r, ds)
                } else {
                    a::tv_multiplication(d, l, r, ds)
                }
            }
            w::DIV => {
                let r = std::slice::from_raw_parts(rhs, sw);
                let mut m = vec![0u64; dw];
                if is_fv {
                    a::fv_division(d, &mut m, l, r, ds)
                } else {
                    a::tv_division(d, &mut m, l, r, ds)
                }
            }
            w::MOD => {
                let r = std::slice::from_raw_parts(rhs, sw);
                let mut q = vec![0u64; dw];
                if is_fv {
                    a::fv_division(&mut q, d, l, r, ds)
                } else {
                    a::tv_division(&mut q, d, l, r, ds)
                }
            }
            w::POW => {
                let r = std::slice::from_raw_parts(rhs, sw);
                if is_fv {
                    a::fv_power(d, l, r, ds)
                } else {
                    a::tv_power(d, l, r, ds)
                }
            }
            // For shifts, rhs points to `[amt_known, amt_val]`; an unknown
            // (four-value) amount yields an all-x result.
            w::LSL | w::LSR | w::ASR => {
                if is_fv && *rhs == 0 {
                    d.fill(0);
                } else {
                    let amt = *rhs.add(1) as u32;
                    match operation {
                        w::LSL if is_fv => sh::fv_l_logical_shift_left(d, l, amt, ds),
                        w::LSL => sh::tv_l_logical_shift_left(d, l, amt, ds),
                        w::LSR if is_fv => sh::fv_l_logical_shift_right(d, l, amt, ds),
                        w::LSR => sh::tv_l_logical_shift_right(d, l, amt, ds),
                        _ if is_fv => sh::fv_l_arithmetic_shift_right(d, l, amt, ds),
                        _ => sh::tv_l_arithmetic_shift_right(d, l, amt, ds),
                    }
                }
            }
            w::ULE => {
                let r = std::slice::from_raw_parts(rhs, sw);
                if is_fv {
                    let res = cmp::fv_l_unsigned_leq(l, r, ss);
                    d[0] = res.spc() as u64;
                    d[1] = res.val() as u64;
                } else {
                    d[0] = cmp::tv_gtu64_unsigned_leq(l, r, ss) as u64;
                }
            }
            w::MIN | w::MAX => {
                let r = std::slice::from_raw_parts(rhs, sw);
                let is_max = operation == w::MAX;
                if is_fv {
                    if a::fv_contains_special(l, ss) || a::fv_contains_special(r, ss) {
                        d.fill(0);
                    } else {
                        let l_le_r = cmp::tv_gtu64_unsigned_leq(&l[snw..], &r[snw..], ss);
                        let pick_l = if is_max { !l_le_r } else { l_le_r };
                        d.copy_from_slice(if pick_l { l } else { r });
                    }
                } else {
                    let l_le_r = cmp::tv_gtu64_unsigned_leq(l, r, ss);
                    let pick_l = if is_max { !l_le_r } else { l_le_r };
                    d.copy_from_slice(if pick_l { l } else { r });
                }
            }
            _ => {}
        }
    }
}

/// Partial write of a (>64) signal: insert `src` (`ssize` bits) at bit `offset`
/// into the signal stored at `heap[base_word..]` (`dsize` bits). Four-value
/// signals set the special and value planes separately. Returns whether the
/// stored value changed (for the poke decision).
extern "C" fn wide_drive(
    heap: *mut u64,
    base_word: u32,
    src: *const u64,
    dsize: u32,
    offset: u32,
    ssize: u32,
    is_fv: u32,
) -> u64 {
    use vogls_bits::set_subslice::tv_l_set;
    let ds = VectorSize::new(dsize).unwrap();
    let ss = VectorSize::new(ssize).unwrap();
    let dnw = (dsize as usize).div_ceil(64);
    let snw = (ssize as usize).div_ceil(64);
    // SAFETY: the JIT passes a valid heap pointer and word-sized src slot.
    // `tv_l_set` returns whether it changed anything (only touches affected
    // words — cheap even for large memory signals).
    unsafe {
        let base = heap.add(base_word as usize);
        if is_fv != 0 {
            let spc_dst = std::slice::from_raw_parts_mut(base, dnw);
            let spc_src = std::slice::from_raw_parts(src, snw);
            let c1 = tv_l_set(spc_dst, spc_src, ds, offset, ss);
            let val_dst = std::slice::from_raw_parts_mut(base.add(dnw), dnw);
            let val_src = std::slice::from_raw_parts(src.add(snw), snw);
            let c2 = tv_l_set(val_dst, val_src, ds, offset, ss);
            u64::from(c1 || c2)
        } else {
            let dst = std::slice::from_raw_parts_mut(base, dnw);
            let s = std::slice::from_raw_parts(src, snw);
            u64::from(tv_l_set(dst, s, ds, offset, ss))
        }
    }
}

/// Extract `dsize` bits at bit `offset` from a (wide) source into `dst`.
/// `src_is_fv` selects the four-value vs two-value source layout; `fill_with_x`
/// (used by Slice) makes out-of-range bits x and forces a four-value dst.
extern "C" fn wide_slice(
    dst: *mut u64,
    src: *const u64,
    offset: u32,
    dsize: u32,
    ssize: u32,
    src_is_fv: u32,
    fill_with_x: u32,
) {
    use vogls_bits::slice::{fv_ll_slice, tv_ll_slice};
    let ds = VectorSize::new(dsize).unwrap();
    let ss = VectorSize::new(ssize).unwrap();
    let snw = (ssize as usize).div_ceil(64);
    let dnw = (dsize as usize).div_ceil(64);
    let src_is_fv = src_is_fv != 0;
    let fill_x = fill_with_x != 0;
    let src_words = if src_is_fv { 2 * snw } else { snw };
    let dst_words = if src_is_fv || fill_x { 2 * dnw } else { dnw };
    // SAFETY: the JIT passes word-sized dst/src pointers per the layout above.
    unsafe {
        let d = std::slice::from_raw_parts_mut(dst, dst_words);
        let s = std::slice::from_raw_parts(src, src_words);
        if src_is_fv {
            fv_ll_slice(d, s, offset, ds, ss, fill_x);
        } else {
            tv_ll_slice(d, s, offset, ds, ss, fill_x);
        }
    }
}

impl FnTable {
    fn new() -> Self {
        Self {
            fmt,
            rtl_dist_uniform: random_shims::rtl_dist_uniform,
            rtl_dist_normal: random_shims::rtl_dist_normal,
            rtl_dist_exponential: random_shims::rtl_dist_exponential,
            rtl_dist_poisson: random_shims::rtl_dist_poisson,
            rtl_dist_chi_square: random_shims::rtl_dist_chi_square,
            rtl_dist_t: random_shims::rtl_dist_t,
            rtl_dist_erlang: random_shims::rtl_dist_erlang,
            real_op,
            wide_binop,
            wide_drive,
            wide_slice,
        }
    }
}

#[repr(C)]
pub struct ColdContextT {
    fn_table: FnTable,

    fmt_strs: *const DynFormatString,

    plugins: *mut Box<dyn RuntimePlugin>,
    plugin_poke_signal: extern "C" fn(NonNull<RuntimePluginState>, usize),

    heap_len: usize,
    readmems: *const (HeapRef, ReadMem),
    readmem: extern "C" fn(*mut u64, usize, u8, NonNull<(HeapRef, ReadMem)>),

    pub heap_wide_ptr: *mut u64,

    fst_poke: *mut u64,

    icount: u64,

    stdout: NonNull<Box<dyn std::io::Write + Send + Sync>>,
    stderr: NonNull<Box<dyn std::io::Write + Send + Sync>>,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct EventT(*const c_void);

unsafe impl Sync for EventT {}
unsafe impl Send for EventT {}

impl EventT {
    /// Wrap a finalized JIT function pointer as a schedulable event.
    pub fn from_ptr(ptr: *const u8) -> Self {
        Self(ptr.cast())
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct TimedEventT {
    event: EventT,
    time: Time,
}

#[repr(C)]
pub struct ScheduleT {
    pub active_region: FfiVec<EventT>,
    pub regions: *mut FfiVec<EventT>,
    pub future: FfiVec<TimedEventT>,
    pub next_time: Time,
}

#[derive(Clone)]
pub struct Schedule {
    active_region: Vec<EventT>,
    regions: Box<[Vec<EventT>]>,
    future: Vec<TimedEventT>,
    next_time: Time,
}

impl Schedule {
    fn with_t<T>(&mut self, mut f: impl FnMut(&mut ScheduleT) -> T) -> T {
        let active_region = std::mem::take(&mut self.active_region);
        let regions = std::mem::take(&mut self.regions);
        let future = std::mem::take(&mut self.future);

        let mut tregions = regions
            .into_iter()
            .map(|r| r.into())
            .collect::<Vec<FfiVec<EventT>>>();

        let mut t = ScheduleT {
            active_region: active_region.into(),
            regions: tregions.as_mut_ptr(),
            future: future.into(),
            next_time: self.next_time,
        };
        let result = f(&mut t);
        self.active_region = t.active_region.into();
        self.regions = tregions.into_iter().map(|r| r.into()).collect();
        self.future = t.future.into();
        self.next_time = t.next_time;
        result
    }
}

pub struct ClifDesignState {
    schedule: Schedule,
    listening: Vec<u64>,
    pub runtime: RuntimeState,
    pub plugins: Vec<RuntimePluginState>,
}

impl Clone for ClifDesignState {
    fn clone(&self) -> Self {
        Self {
            schedule: self.schedule.clone(),
            listening: self.listening.clone(),
            runtime: self.runtime.clone(),
            plugins: self.plugins.iter().map(|p| p.as_ref().clone()).collect(),
        }
    }
}

struct JITCode(SyncWrapper<ManuallyDrop<JITModule>>);

impl JITCode {
    pub fn new(module: JITModule) -> Self {
        Self(SyncWrapper::new(ManuallyDrop::new(module)))
    }
}

impl Drop for JITCode {
    fn drop(&mut self) {
        let mut module = self.0.get_mut();

        // SAFETY: Only called once. Namely, in this drop.
        let module = unsafe { ManuallyDrop::take(&mut module) };

        // SAFETY:
        // This is only safe if no-one is still called into or has references into the JIT-ed
        // pages. We enforce this by never giving out references to the and only allowing people to
        // call into the code with a `&ClifDesign` which ensures no `&mut ClifDesign` is live.
        //
        // There are no `Clone`s on JITCode and this is the exclusive owner of / view into the
        // data. After this drop, no references to the JIT-ed code exist anymore.
        unsafe { module.free_memory() };
    }
}

pub struct ClifWatchers {
    offsets: Vec<u32>,
    entries: Vec<(u32, EventT)>,
}

impl ClifWatchers {
    pub fn new(offsets: Vec<u32>, entries: Vec<(u32, EventT)>) -> Self {
        debug_assert!(!offsets.is_empty());
        debug_assert_eq!(offsets[offsets.len() - 1] as usize, entries.len());
        Self { offsets, entries }
    }

    fn get(&self, signal: usize) -> &[(u32, EventT)] {
        let start = self.offsets[signal] as usize;
        let end = self.offsets[signal + 1] as usize;
        &self.entries[start..end]
    }
}

/// The compiled design: owns the JIT module (keeping code pages mapped) plus the
/// resolved entry / process / drive function pointers.
pub struct ClifDesign {
    // Kept alive to keep the JIT-ed pages alive.
    #[expect(dead_code)]
    module: JITCode,
    entry: EmptyActiveEventQueueFn,
    /// One process entry-point per process, in process order (the `PROCS` array
    /// equivalent).
    procs: Vec<EventT>,
    #[expect(dead_code)]
    drive_fns: Vec<DriveFn>,
    /// Per-signal listener wake sets, consulted by [`Self::poke_signal`].
    watchers: ClifWatchers,
    dyn_fmt_strs: Vec<DynFormatString>,
    read_mems: Vec<(HeapRef, ReadMem)>,
    heap_wide_ptr: u64,
    num_regions: u8,
    /// Standing processes: armed at startup but not seeded into the active
    /// region (their body must not run at t=0).
    standing_procs: vogls_utils::VgHashSet<usize>,
    /// Listener offsets to pre-arm at startup for the standing processes.
    standing_arm_offsets: Vec<u32>,
}

impl ClifDesign {
    /// Assemble a design from a finalized JIT module and its resolved pointers.
    #[expect(clippy::too_many_arguments)]
    pub fn from_parts(
        module: JITModule,
        entry: EmptyActiveEventQueueFn,
        procs: Vec<EventT>,
        drive_fns: Vec<DriveFn>,
        watchers: ClifWatchers,
        dyn_fmt_strs: Vec<DynFormatString>,
        read_mems: Vec<(HeapRef, ReadMem)>,
        heap_wide_ptr: u64,
        num_regions: u8,
        standing_procs: vogls_utils::VgHashSet<usize>,
        standing_arm_offsets: Vec<u32>,
    ) -> Self {
        Self {
            module: JITCode::new(module),
            entry,
            procs,
            drive_fns,
            watchers,
            dyn_fmt_strs,
            heap_wide_ptr,
            read_mems,
            num_regions,
            standing_procs,
            standing_arm_offsets,
        }
    }

    pub fn new_state(
        &self,
        num_listening: usize,
        num_regions: u8,
        runtime: RuntimeState,
        gl: &GlobalContext,
    ) -> ClifDesignState {
        // Reserve one active slot per process up front: a process occupies the
        // active region at most once at a time (its listener bit is cleared on
        // wake before re-arming), so `num_processes` is a hard upper bound and
        // the active region never needs to grow. The drain-based region advance
        // in `run` preserves this reserved buffer.
        let mut active_region = Vec::with_capacity(gl.processes.len());
        active_region.extend(
            (0..gl.processes.len())
                .filter(|i| !self.standing_procs.contains(i))
                .map(|i| self.procs[i].clone()),
        );
        let schedule = Schedule {
            active_region,
            regions: std::iter::repeat_n(Vec::new(), num_regions.into()).collect(),
            future: Vec::new(),
            next_time: u64::MAX,
        };
        // Standing processes are not seeded above; instead pre-arm their
        // listeners so their body runs only when a watched signal is poked.
        let mut listening = vec![0u64; num_listening.div_ceil(64)];
        for &off in &self.standing_arm_offsets {
            listening[off as usize / 64] |= 1u64 << (off % 64);
        }

        ClifDesignState {
            schedule,
            listening,
            runtime,
            plugins: Vec::new(),
        }
    }

    #[expect(clippy::result_unit_err)]
    pub fn run(
        &self,
        state: &mut ClifDesignState,
        io: &mut SimulationIo,
        max_time: u64,
    ) -> Result<(), ()> {
        let empty_active_event_queue_fn = self.entry;
        let heap_wide_ptr = unsafe {
            state
                .runtime
                .heap
                .0
                .as_mut_ptr()
                .add(self.heap_wide_ptr as usize)
        };
        let mut cldctx = ColdContextT {
            fn_table: FnTable::new(),
            fmt_strs: self.dyn_fmt_strs.as_ptr(),
            plugins: state.plugins.as_mut_ptr().cast(),
            plugin_poke_signal,
            heap_len: state.runtime.heap.0.len(),
            readmems: self.read_mems.as_ptr(),
            heap_wide_ptr,
            readmem: read_mem,
            fst_poke: state.runtime.tvl_first_write.as_mut_ptr(),
            icount: state.runtime.instruction_count,
            stdout: NonNull::from_mut(&mut io.stdout),
            stderr: NonNull::from_mut(&mut io.stderr),
        };
        let return_value = state.schedule.with_t(|schedule| {
            'main_loop: loop {
                let return_value = empty_active_event_queue_fn(
                    NonNull::new(state.runtime.heap.0.as_mut_ptr()),
                    NonNull::new(schedule as *mut ScheduleT).unwrap(),
                    state.runtime.time,
                    NonNull::new(state.listening.as_mut_ptr()),
                    NonNull::new(state.runtime.last_active_time.as_mut_ptr()),
                    NonNull::new(&mut cldctx as *mut ColdContextT).unwrap(),
                );

                if return_value.should_exit() {
                    return return_value;
                }

                for i in 0..self.num_regions as usize {
                    let region = unsafe { schedule.regions.add(i).as_mut() }.unwrap();
                    if region.len() > 0 {
                        let active = &mut schedule.active_region;
                        debug_assert!(active.capacity() >= region.len());

                        // SAFETY: It is an invariant that each region contains at most #Procs
                        // events, and active preallocates that much capacity. Therefore, this is
                        // safe.
                        unsafe { active.extend_from_slice_unchecked(region.as_ref()) };
                        region.clear();
                        continue 'main_loop;
                    }
                }

                for plugin in state.plugins.iter_mut() {
                    plugin.timestep(&mut state.runtime);
                }

                if schedule.future.len() == 0 {
                    return ReturnValue::CONTINUE;
                }

                if schedule.next_time.wrapping_add(1) < state.runtime.time {
                    eprintln!("Time overflow!");
                    return ReturnValue::STOP;
                }

                if schedule.next_time > max_time {
                    state.runtime.time = max_time;
                    break ReturnValue::CONTINUE;
                }

                let mut active: Vec<_> = std::mem::take(&mut schedule.active_region).into();
                let mut future: Vec<_> = std::mem::take(&mut schedule.future).into();

                state.runtime.time = schedule.next_time;
                let mut next_time = Time::MAX;
                active.extend(
                    future
                        .extract_if(.., |te| {
                            let is_next_timestep = te.time == schedule.next_time;
                            if !is_next_timestep {
                                next_time = te.time.min(next_time);
                            }
                            is_next_timestep
                        })
                        .map(|te| te.event),
                );

                schedule.active_region = active.into();
                schedule.future = future.into();
                schedule.next_time = next_time;

                if schedule.active_region.len() == 0 {
                    break ReturnValue::CONTINUE;
                }
            }
        });

        state.runtime.instruction_count = cldctx.icount;
        for plugin in state.plugins.iter_mut() {
            plugin.finish(&mut state.runtime);
        }
        if return_value.should_exit() {
            if return_value.is_ok() {
                writeln!(io.stdout, "[FINISH]").unwrap();
                return Ok(());
            } else {
                return Err(());
            }
        }
        Ok(())
    }

    pub fn poke_signal(&self, state: &mut ClifDesignState, signal: RtSignalKey) {
        for &(offset, ref target) in self.watchers.get(signal.as_usize()) {
            let word = offset as usize / 64;
            let bit = 1u64 << (offset % 64);
            if state.listening[word] & bit != 0 {
                state.listening[word] ^= bit;
                state.schedule.active_region.push(target.clone());
            }
        }
    }
}

extern "C" fn plugin_poke_signal(mut plugin: NonNull<RuntimePluginState>, signal: usize) {
    unsafe { plugin.as_mut() }.poke_signal(RtSignalKey::from_usize(signal).unwrap());
}

extern "C" fn read_mem(
    heap: *mut u64,
    heap_len: usize,
    mode: u8,
    ptr: NonNull<(HeapRef, ReadMem)>,
) {
    let (heap_ref, read_mem) = unsafe { ptr.as_ref() };
    let heap = unsafe { std::slice::from_raw_parts_mut(heap, heap_len) };
    let mode = if mode == 0 {
        Mode::TwoValue
    } else {
        Mode::FourValue
    };
    vogls_runtime::readmem::read_mem(
        &read_mem.path,
        heap,
        *heap_ref,
        mode,
        read_mem.offset,
        read_mem.limit,
        read_mem.stride,
        read_mem.binary,
    )
    .unwrap();
}

extern "C" fn fmt(
    mut file: NonNull<Box<dyn std::io::Write + Send + Sync>>,
    dyn_fmt: *const DynFormatString,
    bits: *const BitsRefT,
) {
    let file = unsafe { file.as_mut() };
    let dyn_fmt = unsafe { dyn_fmt.as_ref() }.unwrap();
    let args = (0..dyn_fmt.arguments().len()).map(|i| {
        let ref_t = unsafe { bits.add(i).as_ref() }.unwrap();
        let size = VectorSize::new(ref_t.size).unwrap();
        if ref_t.mode == 0 && size.get() <= 64 {
            Bits::from_u64(size, *unsafe { ref_t.ptr.as_ref() })
        } else if ref_t.mode == 1 && size.get() <= 32 {
            Bits::from_four_value_u64(
                size,
                *unsafe { ref_t.ptr.as_ref() } as u32,
                (*unsafe { ref_t.ptr.as_ref() } >> size.get()) as u32,
            )
        } else if ref_t.mode == 1 {
            Bits::from_boxed_slice(
                Mode::FourValue,
                size,
                (0..2 * size.get().div_ceil(64))
                    .map(|j| *unsafe { ref_t.ptr.add(j as usize).as_ref() })
                    .collect(),
            )
        } else {
            Bits::from_boxed_slice(
                Mode::TwoValue,
                size,
                (0..size.get().div_ceil(64))
                    .map(|j| *unsafe { ref_t.ptr.add(j as usize).as_ref() })
                    .collect(),
            )
        }
    });
    dyn_fmt.write_to(file, args).unwrap();
}

mod random_shims {
    use std::ptr::NonNull;

    fn combine_seed_result(seed: i32, result: i32) -> u64 {
        (u64::from(seed.cast_unsigned()) << 32) | u64::from(result.cast_unsigned())
    }

    pub extern "C" fn rtl_dist_uniform(
        mut seed: i32,
        start: i32,
        end: i32,
        _io: NonNull<Box<dyn std::io::Write + Send + Sync>>,
    ) -> u64 {
        let result = vogls_runtime::random::rtl_dist_uniform(&mut seed, start, end);
        combine_seed_result(seed, result)
    }
    pub extern "C" fn rtl_dist_normal(
        mut seed: i32,
        mean: i32,
        df: i32,
        _io: NonNull<Box<dyn std::io::Write + Send + Sync>>,
    ) -> u64 {
        let result = vogls_runtime::random::rtl_dist_normal(&mut seed, mean, df);
        combine_seed_result(seed, result)
    }
    pub extern "C" fn rtl_dist_exponential(
        mut seed: i32,
        mean: i32,
        mut io: NonNull<Box<dyn std::io::Write + Send + Sync>>,
    ) -> u64 {
        let mut warning = None;
        let result = vogls_runtime::random::rtl_dist_exponential(&mut seed, mean, &mut warning);
        if let Some(warning) = warning {
            use std::io::Write;
            let io = unsafe { io.as_mut() };
            _ = writeln!(io, "WARNING: {}", warning.as_str());
        }
        combine_seed_result(seed, result)
    }
    pub extern "C" fn rtl_dist_poisson(
        mut seed: i32,
        mean: i32,
        mut io: NonNull<Box<dyn std::io::Write + Send + Sync>>,
    ) -> u64 {
        let mut warning = None;
        let result = vogls_runtime::random::rtl_dist_poisson(&mut seed, mean, &mut warning);
        if let Some(warning) = warning {
            use std::io::Write;
            let io = unsafe { io.as_mut() };
            _ = writeln!(io, "WARNING: {}", warning.as_str());
        }
        combine_seed_result(seed, result)
    }
    pub extern "C" fn rtl_dist_chi_square(
        mut seed: i32,
        dof: i32,
        mut io: NonNull<Box<dyn std::io::Write + Send + Sync>>,
    ) -> u64 {
        let mut warning = None;
        let result = vogls_runtime::random::rtl_dist_chi_square(&mut seed, dof, &mut warning);
        if let Some(warning) = warning {
            use std::io::Write;
            let io = unsafe { io.as_mut() };
            _ = writeln!(io, "WARNING: {}", warning.as_str());
        }
        combine_seed_result(seed, result)
    }
    pub extern "C" fn rtl_dist_t(
        mut seed: i32,
        dof: i32,
        mut io: NonNull<Box<dyn std::io::Write + Send + Sync>>,
    ) -> u64 {
        let mut warning = None;
        let result = vogls_runtime::random::rtl_dist_t(&mut seed, dof, &mut warning);
        if let Some(warning) = warning {
            use std::io::Write;
            let io = unsafe { io.as_mut() };
            _ = writeln!(io, "WARNING: {}", warning.as_str());
        }
        combine_seed_result(seed, result)
    }
    pub extern "C" fn rtl_dist_erlang(
        mut seed: i32,
        k_stage: i32,
        mean: i32,
        mut io: NonNull<Box<dyn std::io::Write + Send + Sync>>,
    ) -> u64 {
        let mut warning = None;
        let result = vogls_runtime::random::rtl_dist_erlang(&mut seed, k_stage, mean, &mut warning);
        if let Some(warning) = warning {
            use std::io::Write;
            let io = unsafe { io.as_mut() };
            _ = writeln!(io, "WARNING: {}", warning.as_str());
        }
        combine_seed_result(seed, result)
    }
}

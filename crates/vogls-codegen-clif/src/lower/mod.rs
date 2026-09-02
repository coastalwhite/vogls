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

use std::mem::offset_of;

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
use vogls_ir::{
    BasicBlockKey, BasicBlockTerminator, GlobalContext, LogicMode, ShiftImmOp, SignalKey,
    TemporalRegionKey, VariableKey, VectorSize,
};
use vogls_runtime::RtSignalKey;
use vogls_utils::{TableKey, VgHashMap};

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
    drive: Signature,
    next_event: Signature,
    push: Signature,
    sfe: Signature,
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
            drive: sysv(&[ptr, I64, ptr, ptr, ptr], None),
            next_event: sysv(&[ptr], Some(ptr)),
            push: sysv(&[ptr, ptr], None),
            sfe: sysv(&[ptr, I64, ptr], None),
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
    pub drive_fns: Vec<FuncId>,
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
        use crate::runtime::{ClifDesign, ClifWatchers, DriveFn, EmptyActiveEventQueueFn, EventT};
        let Compiled {
            module,
            entry: entry_id,
            procs: proc_ids,
            drive_fns: drive_ids,
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
        let drive_fns = drive_ids
            .iter()
            .map(|&id| unsafe {
                std::mem::transmute::<*const u8, DriveFn>(module.get_finalized_function(id))
            })
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
            drive_fns,
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
    next_event: FuncId,
    push: FuncId,
    sfe: FuncId,
    entry: FuncId,
    tr_funcs: VgHashMap<TemporalRegionKey, FuncId>,
    drive_fn_ids: Vec<FuncId>,
    /// Listeners per signal (by `RtSignalKey::as_usize`), collected while
    /// lowering `Watch` terminators.
    listeners: Vec<Vec<Listener>>,
    num_listening: u32,
    /// Offset assigned to each `Watch` terminator, keyed by the BB it terminates.
    /// Populated by the listener pre-pass so drive sites can inline the wake set.
    watch_offset: VgHashMap<BasicBlockKey, u32>,
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

    disassembly: bool,
}

impl<'a> Compiler<'a> {
    fn new(
        num_signals: usize,
        num_plugins: usize,
        gl: &'a GlobalContext,
        info: SignalInfo<'a>,
        heap_builder: &'a mut HeapBuilder,
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
        let disassembly = std::env::var_os("VOGLS_DISASM").is_some();
        Self {
            module,
            ptr,
            fe,
            sigs,
            next_event: FuncId::from_u32(0),
            push: FuncId::from_u32(0),
            sfe: FuncId::from_u32(0),
            entry: FuncId::from_u32(0),
            tr_funcs: VgHashMap::default(),
            drive_fn_ids: Vec::new(),
            listeners: (0..num_signals).map(|_| Vec::new()).collect(),
            num_listening: 0,
            watch_offset: VgHashMap::default(),
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
            disassembly,
        }
    }

    fn declare(&mut self, name: &str, sig: &Signature) -> FuncId {
        self.module
            .declare_function(name, Linkage::Local, sig)
            .unwrap()
    }

    // --- schedule helpers (see runtime.rs layout) ---------------------------

    fn build_next_event(&mut self, fb: &mut FunctionBuilderContext) {
        let mut ctx = self.module.make_context();
        ctx.func.signature = self.sigs.next_event.clone();
        ctx.func.name = UserFuncName::user(0, self.next_event.as_u32());
        {
            let mut b = FunctionBuilder::new(&mut ctx.func, fb);
            let entry = b.create_block();
            let empty = b.create_block();
            let nonempty = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let schedule = b.block_params(entry)[0];
            let active = ioff(&mut b, self.ptr, schedule, layout::SCHED_ACTIVE);
            let len = b
                .ins()
                .load(I64, mem(), active, FfiVec::<EventT>::LEN_OFFSET as i32);
            b.ins().brif(len, nonempty, &[], empty, &[]);
            b.switch_to_block(empty);
            let zero = b.ins().iconst(self.ptr, 0);
            b.ins().return_(&[zero]);
            b.switch_to_block(nonempty);
            let one = b.ins().iconst(I64, 1);
            let new_len = b.ins().isub(len, one);
            b.ins()
                .store(mem(), new_len, active, FfiVec::<EventT>::LEN_OFFSET as i32);
            let data = b
                .ins()
                .load(self.ptr, mem(), active, FfiVec::<EventT>::PTR_OFFSET as i32);
            let esize = b.ins().iconst(self.ptr, layout::EVENT_SIZE as i64);
            let off = b.ins().imul(new_len, esize);
            let elem = b.ins().iadd(data, off);
            let event = b.ins().load(self.ptr, mem(), elem, 0);
            b.ins().return_(&[event]);
            b.seal_all_blocks();
            b.finalize(self.fe);
        }
        self.module
            .define_function(self.next_event, &mut ctx)
            .unwrap();
        self.module.clear_context(&mut ctx);
    }

    fn build_push(&mut self, fb: &mut FunctionBuilderContext) {
        let grow_sig = self.sigs.grow.clone();
        let mut ctx = self.module.make_context();
        ctx.func.signature = self.sigs.push.clone();
        ctx.func.name = UserFuncName::user(0, self.push.as_u32());
        {
            let mut b = FunctionBuilder::new(&mut ctx.func, fb);
            let entry = b.create_block();
            let grow_bb = b.create_block();
            let store_bb = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let vec = b.block_params(entry)[0];
            let event = b.block_params(entry)[1];
            let len = b
                .ins()
                .load(I64, mem(), vec, FfiVec::<EventT>::LEN_OFFSET as i32);
            let cap = b
                .ins()
                .load(I64, mem(), vec, FfiVec::<EventT>::CAP_OFFSET as i32);
            let full = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, len, cap);
            b.ins().brif(full, grow_bb, &[], store_bb, &[]);
            b.switch_to_block(grow_bb);
            let grow_fn = b
                .ins()
                .load(self.ptr, mem(), vec, FfiVec::<EventT>::GROW_OFFSET as i32);
            let gr = b.import_signature(grow_sig);
            b.ins().call_indirect(gr, grow_fn, &[vec]);
            b.ins().jump(store_bb, &[]);
            b.switch_to_block(store_bb);
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
            b.ins().return_(&[]);
            b.seal_all_blocks();
            b.finalize(self.fe);
        }
        self.module.define_function(self.push, &mut ctx).unwrap();
        self.module.clear_context(&mut ctx);
    }

    /// Inline the body of `event_vec_push` into the *active* region at a call
    /// site. The active region is pre-sized to the process count and never grows
    /// (see `ClifDesign::run`'s drain-based region advance and `new_state`'s
    /// reservation), so we omit the capacity check + grow `call_indirect`
    /// entirely. This leaves no call on the listener-wake path, which is what
    /// lets an inlined-drive TR stay leaf (no frame / callee-save prologue).
    fn emit_push_inline(&mut self, b: &mut FunctionBuilder, vec: Value, event: Value) {
        let len = b
            .ins()
            .load(I64, mem(), vec, FfiVec::<EventT>::LEN_OFFSET as i32);
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

    fn build_sfe(&mut self, fb: &mut FunctionBuilderContext) {
        let grow_sig = self.sigs.grow.clone();
        let mut ctx = self.module.make_context();
        ctx.func.signature = self.sigs.sfe.clone();
        ctx.func.name = UserFuncName::user(0, self.sfe.as_u32());
        {
            let mut b = FunctionBuilder::new(&mut ctx.func, fb);
            let entry = b.create_block();
            let grow_bb = b.create_block();
            let store_bb = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let schedule = b.block_params(entry)[0];
            let time = b.block_params(entry)[1];
            let event = b.block_params(entry)[2];
            let future = ioff(&mut b, self.ptr, schedule, layout::SCHED_FUTURE);
            let len = b
                .ins()
                .load(I64, mem(), future, FfiVec::<EventT>::LEN_OFFSET as i32);
            let cap = b
                .ins()
                .load(I64, mem(), future, FfiVec::<EventT>::CAP_OFFSET as i32);
            let full = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, len, cap);
            b.ins().brif(full, grow_bb, &[], store_bb, &[]);
            b.switch_to_block(grow_bb);
            let grow_fn = b.ins().load(
                self.ptr,
                mem(),
                future,
                FfiVec::<EventT>::GROW_OFFSET as i32,
            );
            let gr = b.import_signature(grow_sig);
            b.ins().call_indirect(gr, grow_fn, &[future]);
            b.ins().jump(store_bb, &[]);
            b.switch_to_block(store_bb);
            let data = b
                .ins()
                .load(self.ptr, mem(), future, FfiVec::<EventT>::PTR_OFFSET as i32);
            let esize = b.ins().iconst(self.ptr, layout::TIMED_EVENT_SIZE as i64);
            let off = b.ins().imul(len, esize);
            let elem = b.ins().iadd(data, off);
            b.ins()
                .store(mem(), event, elem, layout::TIMED_EVENT_EVENT as i32);
            b.ins()
                .store(mem(), time, elem, layout::TIMED_EVENT_TIME as i32);
            let one = b.ins().iconst(I64, 1);
            let nl = b.ins().iadd(len, one);
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
            b.ins().return_(&[]);
            b.seal_all_blocks();
            b.finalize(self.fe);
        }
        self.module.define_function(self.sfe, &mut ctx).unwrap();
        self.module.clear_context(&mut ctx);
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
        self.module.define_function(self.entry, &mut ctx).unwrap();
        self.module.clear_context(&mut ctx);
    }

    // --- TR lowering --------------------------------------------------------

    /// Pre-pass: walk one TR's reachable blocks in the *same* DFS order as
    /// `build_tr`, assigning each `Watch` its listener offset and populating
    /// `self.listeners` / `self.watch_offset`. Running this over all TRs before
    /// lowering means drive sites can inline the complete listener wake set.
    fn collect_listeners(
        &mut self,
        process_idx: usize,
        entry_bb: BasicBlockKey,
        standing: Option<&[SignalKey]>,
    ) {
        let mut seen = vogls_utils::VgHashSet::default();
        seen.insert(entry_bb);
        let mut order = vec![entry_bb];
        let mut stack = vec![entry_bb];
        while let Some(k) = stack.pop() {
            self.gl.bbs[k].terminator.for_each_non_temporal_bb(|s| {
                if seen.insert(s) {
                    order.push(s);
                    stack.push(s);
                }
            });
        }
        for &k in &order {
            if let BasicBlockTerminator::Watch(tr, signals) = &self.gl.bbs[k].terminator {
                let target = self.tr_funcs[tr];
                let offset = self.num_listening;
                self.num_listening += 1;
                for sig in signals.iter() {
                    let rt = self.info.rt_signal_map[sig];
                    self.listeners[rt.as_usize()].push(Listener { offset, target });
                }
                self.watch_offset.insert(k, offset);
                // If this process is standing and this is its arming Watch (the
                // watch set equals the standing set), record the offset so the
                // listener is pre-armed at startup instead of the body running.
                if let Some(sset) = standing {
                    if !self.standing_procs.contains(&process_idx)
                        && sset.len() == signals.len()
                        && sset.iter().zip(signals.iter()).all(|(a, b)| a == b)
                    {
                        self.standing_procs.insert(process_idx);
                        self.standing_arm_offsets.push(offset);
                    }
                }
            }
        }
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
        self.module.define_function(func_id, &mut ctx).unwrap();
        if self.disassembly {
            if let Some(cc) = ctx.compiled_code() {
                if let Some(d) = cc.vcode.as_ref() {
                    eprintln!(
                        "=== tr {process_idx}_{tr_idx} (func {}) ===\n{d}",
                        func_id.as_u32()
                    );
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
    ) {
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

        // fst_poke: force a poke the first time a two-value signal is written.
        let guard = {
            let fp = b
                .ins()
                .load(self.ptr, mem(), params.cldctx, layout::CTX_FST_POKE as i32);
            let idx = rt.as_usize();
            let w = b.ins().load(I64, mem(), fp, ((idx / 64) * 8) as i32);
            let sh = b.ins().ushr_imm_u(w, (idx % 64) as i64);
            let bit = b.ins().band_imm_u(sh, 1);
            let never = b.ins().icmp_imm_u(IntCC::Equal, bit, 0);
            b.ins().bor(never, changed)
        };

        let do_bb = b.create_block();
        let merge = b.create_block();
        b.ins().brif(guard, do_bb, &[], merge, &[]);

        b.switch_to_block(do_bb);
        self.call_drive_signal(b, params, rt);
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
    }

    /// Full-width four-value drive: poke-if-changed (no fst_poke for FV) + store.
    fn lower_drive_fv(
        &mut self,
        b: &mut FunctionBuilder,
        params: &Params,
        signal: SignalKey,
        src_val: Value,
        src_spc: Value,
        size: u32,
    ) {
        let (href, rt, _) = self.info.heap_ref(signal);
        let heap = params.heap_ptr;
        let word = href.offset.bit_offset / 64;
        let shift = href.offset.bit_offset % 64;

        let do_bb = b.create_block();
        let merge = b.create_block();

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
            b.ins().brif(changed, do_bb, &[], merge, &[]);
            b.switch_to_block(do_bb);
            self.call_drive_signal(b, params, rt);
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
            b.ins().brif(changed, do_bb, &[], merge, &[]);
            b.switch_to_block(do_bb);
            self.call_drive_signal(b, params, rt);
            let ms = maskv(b, src_spc, size);
            let mv = maskv(b, src_val, size);
            b.ins().store(mem(), ms, heap, (word * 8) as i32);
            b.ins().store(mem(), mv, heap, ((word + 1) * 8) as i32);
        }
        b.ins().jump(merge, &[]);
        b.switch_to_block(merge);
    }

    /// Inline the drive_signal body (mirrors build_drive_signal) at a drive
    /// site. With the active-region push no longer carrying a grow call, this
    /// body has no calls (absent plugins), so an inlined-drive TR stays leaf:
    /// no frame, no callee-save spills. Correct listener sets require the
    /// `collect_listeners` pre-pass to have run first.
    fn call_drive_signal(&mut self, b: &mut FunctionBuilder, params: &Params, rt: RtSignalKey) {
        let idx = rt.as_usize();
        let is_tv = self.info.signal_mode[idx] == LogicMode::TwoValue;
        let lupdt = self.info.lupdt_indexes.get(&rt).copied();
        let listeners: Vec<(u32, FuncId)> = self.listeners[idx]
            .iter()
            .map(|l| (l.offset, l.target))
            .collect();
        let num_plugins = self.num_plugins;

        let (schedule, time, listening, last_active_time, cldctx) = (
            params.schedule,
            params.time,
            params.listening,
            params.last_active_time,
            params.cldctx,
        );

        if is_tv {
            let fp = b
                .ins()
                .load(self.ptr, mem(), cldctx, layout::CTX_FST_POKE as i32);
            let w = b.ins().load(I64, mem(), fp, ((idx / 64) * 8) as i32);
            let nw = b.ins().bor_imm_u(w, 1i64 << (idx % 64));
            b.ins().store(mem(), nw, fp, ((idx / 64) * 8) as i32);
        }

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

        for (offset, target) in listeners {
            let wake = b.create_block();
            let next = b.create_block();
            let w = b
                .ins()
                .load(I64, mem(), listening, ((offset / 64) * 8) as i32);
            let bit = b.ins().band_imm_u(w, 1i64 << (offset % 64));
            b.ins().brif(bit, wake, &[], next, &[]);
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
    ) {
        let (href, rt, mode) = self.info.heap_ref(signal);
        let heap = params.heap_ptr;
        let base_word = (href.offset.bit_offset / 64) as i64;
        let d_size = self.gl.signals[signal].size.get();
        let s_size = self.gl.vars.size(src).get();
        let is_fv = mode == LogicMode::FourValue;
        let src_ptr = self.value_words_ptr(b, src, vmap, spc_map, wide_map, params.cldctx);

        let do_bb = b.create_block();
        let merge = b.create_block();
        b.ins().brif(off_known, do_bb, &[], merge, &[]);
        b.switch_to_block(do_bb);

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

        // Two-value signals also poke on the first write (fst_poke).
        let poke = if is_fv {
            ch
        } else {
            let fp = b
                .ins()
                .load(self.ptr, mem(), cldctx, layout::CTX_FST_POKE as i32);
            let idx = rt.as_usize();
            let w = b.ins().load(I64, mem(), fp, ((idx / 64) * 8) as i32);
            let sh = b.ins().ushr_imm_u(w, (idx % 64) as i64);
            let bit = b.ins().band_imm_u(sh, 1);
            let never = b.ins().icmp_imm_u(IntCC::Equal, bit, 0);
            b.ins().bor(never, ch)
        };
        let drive_bb = b.create_block();
        b.ins().brif(poke, drive_bb, &[], merge, &[]);
        b.switch_to_block(drive_bb);
        self.call_drive_signal(b, params, rt);
        b.ins().jump(merge, &[]);
        b.switch_to_block(merge);
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
    ) {
        let (href, rt, mode) = self.info.heap_ref(signal);
        let d_size = self.gl.signals[signal].size.get();
        let heap = params.heap_ptr;
        let base_bit = href.offset.bit_offset;
        let do_bb = b.create_block();
        let merge = b.create_block();
        if mode == LogicMode::TwoValue {
            let cur = read_heap_field(b, heap, base_bit, d_size);
            // Mask to the field width: an insert whose bits land at/after d_size
            // (e.g. an out-of-range index like a[1] on a 1-bit reg) must be a
            // no-op, not leak bits or spuriously poke.
            let new = insert_bits(b, cur, src_val, off, s_size);
            let new = maskv(b, new, d_size);
            let changed = b.ins().icmp(IntCC::NotEqual, new, cur);
            // fst_poke: match the two-value full/partial drive path.
            let cldctx = params.cldctx;
            let fp = b
                .ins()
                .load(self.ptr, mem(), cldctx, layout::CTX_FST_POKE as i32);
            let idx = rt.as_usize();
            let w = b.ins().load(I64, mem(), fp, ((idx / 64) * 8) as i32);
            let sh = b.ins().ushr_imm_u(w, (idx % 64) as i64);
            let bit = b.ins().band_imm_u(sh, 1);
            let never = b.ins().icmp_imm_u(IntCC::Equal, bit, 0);
            let poke = b.ins().bor(never, changed);
            let guard = b.ins().band(poke, off_known);
            b.ins().brif(guard, do_bb, &[], merge, &[]);
            b.switch_to_block(do_bb);
            self.call_drive_signal(b, params, rt);
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
            b.ins().brif(guard, do_bb, &[], merge, &[]);
            b.switch_to_block(do_bb);
            self.call_drive_signal(b, params, rt);
            self.fv_store(b, heap, href, d_size, new_val, new_spc);
        }
        b.ins().jump(merge, &[]);
        b.switch_to_block(merge);
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

    /// Generate the per-signal poke routine `drive_signal_{rt}`.
    fn build_drive_signal(
        &mut self,
        fb: &mut FunctionBuilderContext,
        rt: RtSignalKey,
        func_id: FuncId,
    ) {
        let idx = rt.as_usize();
        let is_tv = self.info.signal_mode[idx] == LogicMode::TwoValue;
        let lupdt = self.info.lupdt_indexes.get(&rt).copied();
        let listeners: Vec<(u32, FuncId)> = self.listeners[idx]
            .iter()
            .map(|l| (l.offset, l.target))
            .collect();
        let num_plugins = self.num_plugins;
        let plugin_sig = self.sigs.plugin_poke.clone();

        let mut ctx = self.module.make_context();
        let dis = std::env::var_os("VOGLS_DISASM").is_some();
        if dis {
            ctx.set_disasm(true);
        }
        ctx.func.signature = self.sigs.drive.clone();
        ctx.func.name = UserFuncName::user(0, func_id.as_u32());
        {
            let mut b = FunctionBuilder::new(&mut ctx.func, fb);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let p: Vec<Value> = b.block_params(entry).to_vec();
            let (schedule, time, listening, last_active_time, cldctx) =
                (p[0], p[1], p[2], p[3], p[4]);

            // fst_poke: mark this two-value signal as written.
            if is_tv {
                let fp = b
                    .ins()
                    .load(self.ptr, mem(), cldctx, layout::CTX_FST_POKE as i32);
                let w = b.ins().load(I64, mem(), fp, ((idx / 64) * 8) as i32);
                let nw = b.ins().bor_imm_u(w, 1i64 << (idx % 64));
                b.ins().store(mem(), nw, fp, ((idx / 64) * 8) as i32);
            }

            // Poke each plugin with this signal id.
            if num_plugins > 0 {
                let plugins = b
                    .ins()
                    .load(self.ptr, mem(), cldctx, layout::CTX_PLUGINS as i32);
                let poke = b
                    .ins()
                    .load(self.ptr, mem(), cldctx, layout::CTX_PLUGIN_POKE as i32);
                let sig_ref = b.import_signature(plugin_sig);
                let id = b.ins().iconst(I64, rt.as_u64() as i64);
                for i in 0..num_plugins {
                    let pl = b.ins().iadd_imm_u(plugins, (i * PLUGIN_STATE_SIZE) as i64);
                    b.ins().call_indirect(sig_ref, poke, &[pl, id]);
                }
            }

            // last_active_time for signals with a LastUpdateTime reader.
            if let Some(li) = lupdt {
                b.ins()
                    .store(mem(), time, last_active_time, (li * 8) as i32);
            }

            // Wake armed listeners.
            for (offset, target) in listeners {
                let wake = b.create_block();
                let next = b.create_block();
                let w = b
                    .ins()
                    .load(I64, mem(), listening, ((offset / 64) * 8) as i32);
                // Test the bit directly (isel: `test $mask, reg; jnz`) rather than
                // shifting it down first, and keep `w` live for the clear below.
                let bit = b.ins().band_imm_u(w, 1i64 << (offset % 64));
                b.ins().brif(bit, wake, &[], next, &[]);
                b.switch_to_block(wake);
                let cleared = b.ins().bxor_imm_u(w, 1i64 << (offset % 64));
                b.ins()
                    .store(mem(), cleared, listening, ((offset / 64) * 8) as i32);
                let active = ioff(&mut b, self.ptr, schedule, layout::SCHED_ACTIVE);
                let fr = self.module.declare_func_in_func(target, b.func);
                let ta = b.ins().func_addr(self.ptr, fr);
                self.emit_push_inline(&mut b, active, ta);
                b.ins().jump(next, &[]);
                b.switch_to_block(next);
            }

            b.ins().return_(&[]);
            b.seal_all_blocks();
            b.finalize(self.fe);
        }
        self.module.define_function(func_id, &mut ctx).unwrap();
        if dis {
            if let Some(cc) = ctx.compiled_code() {
                if let Some(d) = cc.vcode.as_ref() {
                    eprintln!(
                        "=== drive_signal {} (func {}) ===\n{}",
                        rt.as_usize(),
                        func_id.as_u32(),
                        d
                    );
                }
            }
        }
        self.module.clear_context(&mut ctx);
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
) -> Result<Compiled, String> {
    let num_signals = info.signal_to_heap.len();
    let mut c = Compiler::new(num_signals, num_plugins, gl, info, heap_builder);
    // drive_fn ids, filled below.
    let mut fb = FunctionBuilderContext::new();

    c.next_event = c.declare("next_event", &c.sigs.next_event.clone());
    c.push = c.declare("event_vec_push", &c.sigs.push.clone());
    c.sfe = c.declare("schedule_future_event", &c.sigs.sfe.clone());
    c.entry = c.declare("empty_active_event_queue", &c.sigs.entry.clone());

    let event_sig = c.sigs.event.clone();
    let drive_sig = c.sigs.drive.clone();
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
    let mut drive_fn_ids = Vec::new();
    for i in 0..num_signals {
        drive_fn_ids.push(c.declare(&format!("drive_signal_{i}"), &drive_sig));
    }
    c.drive_fn_ids = drive_fn_ids.clone();

    c.build_next_event(&mut fb);
    c.build_push(&mut fb);
    c.build_sfe(&mut fb);
    c.build_entry(&mut fb);

    // Pre-pass: collect all listeners (in the same order build_tr discovers
    // them) so drive sites can inline the complete wake set.
    for (pi, (_k, process)) in gl.processes.iter().enumerate() {
        // A process is "standing" (armed but not run at t=0) only if at least
        // one watcher does NOT trigger a t=0 poke — matches the bytecode filter.
        let standing = process
            .standing
            .as_deref()
            .filter(|w| w.iter().any(|s| !gl.signals[*s].triggers_t0_poke()));
        for tr in process.regions.iter() {
            c.collect_listeners(pi, tr.entry(), standing);
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

    // Generate drive_signal_{i} bodies now that listeners are collected.
    for i in 0..num_signals {
        c.build_drive_signal(
            &mut fb,
            RtSignalKey::from_usize(i).unwrap(),
            drive_fn_ids[i],
        );
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
        drive_fns: drive_fn_ids,
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

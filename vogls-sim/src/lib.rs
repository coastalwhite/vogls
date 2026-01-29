use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::num::NonZeroUsize;

use slotmap::{SlotMap, new_key_type};
use vogls_bits::arithmetic::{FvLogicValue, fv_pack_u64, fv_set_no_special, fv_unpack_u64};
use vogls_bits::load::load_partial_u64;
use vogls_bits::set_subslice::{tv_l_set, tv_s_set};
use vogls_bits::store::store_partial_u64;
use vogls_bits::{BitsDataRef, get_disjoint_dst_s1_s2, get_disjoint_dst_src};
use vogls_ir::dyn_format_string::{Base, Padding, format_bits};
use vogls_ir::vcd::NetType;
use vogls_ir::{Bits, LogicMode, SCALAR_VSIZE, SignalKey, TIME_VSIZE, VectorSize};

mod execution;
mod instruction;

pub use instruction::*;

new_key_type! { pub struct ListenerKey; }

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct VmProcessKey(pub u64);

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub enum DispatchKey {
    Signal(VmSignalKey),
    Process(VmProcessKey),
}

pub struct Regions {
    pub active: Vec<Event>,
    pub other_dispatched: Vec<HashMap<DispatchKey, usize>>,
    pub other: Vec<Vec<Event>>,
}

impl Regions {
    pub fn new(num_additional_regions: usize) -> Self {
        Self {
            active: Vec::new(),
            other_dispatched: vec![HashMap::new(); num_additional_regions],
            other: vec![Vec::new(); num_additional_regions],
        }
    }
}

pub type Timestamp = u64;
pub type InstanceId = u64;

pub struct Context {
    time: Timestamp,
    logic_mode: LogicMode,
    pub stdout: Box<dyn std::io::Write>,
    pub stderr: Box<dyn std::io::Write>,
    pub instruction_count: u64,
    pub event_count: u64,
    pub itrace: bool,
}

impl Context {
    pub fn new(
        logic_mode: LogicMode,
        stdout: Box<dyn std::io::Write>,
        stderr: Box<dyn std::io::Write>,
    ) -> Self {
        Self {
            time: 0,
            logic_mode,
            stdout,
            stderr,
            instruction_count: 0,
            event_count: 0,
            itrace: false,
        }
    }
}

pub struct Stack(Box<[u64]>);

impl Stack {
    fn get(&self, at: StackRef) -> &[u8] {
        &bytemuck::cast_slice::<u64, u8>(&self.0)[at.offset.0..]
            [..at.size.get().div_ceil(8) as usize]
    }
    fn get_mut(&mut self, at: StackRef) -> &mut [u8] {
        &mut bytemuck::cast_slice_mut::<u64, u8>(&mut self.0)[at.offset.0..]
            [..at.size.get().div_ceil(8) as usize]
    }

    fn get_u64_slice(&self, at: StackOffset, nwords: usize) -> &[u64] {
        debug_assert_eq!(at.0 % 8, 0);
        &self.0[at.0 / 8..][..nwords]
    }
    fn get_mut_u64_slice(&mut self, at: StackOffset, nwords: usize) -> &mut [u64] {
        debug_assert_eq!(at.0 % 8, 0);
        &mut self.0[at.0 / 8..][..nwords]
    }

    fn get_u64(&self, at: StackOffset) -> u64 {
        self.get_u64_slice(at, 1)[0]
    }
    fn get_mut_u64(&mut self, at: StackOffset) -> &mut u64 {
        &mut self.get_mut_u64_slice(at, 1)[0]
    }

    fn load_exact_tv_u32(&self, at: StackOffset) -> u32 {
        self.get_tv_u64(at.to_ref(VectorSize::new(32).unwrap())) as u32
    }
    fn load_exact_fv_u32(&self, at: StackOffset) -> (u32, u32) {
        let (spc, val) = self.get_fv_u64(at.to_ref(VectorSize::new(32).unwrap()));
        (spc as u32, val as u32)
    }

    fn get_tv_u64(&self, at: StackRef) -> u64 {
        debug_assert!(at.size.get() <= 64);
        if at.size.get() <= 32 {
            load_partial_u64(self.get(at), at.size)
        } else {
            self.get_u64(at.offset)
        }
    }
    fn set_tv_u64(&mut self, at: StackRef, value: u64) -> u64 {
        debug_assert!(at.size.get() <= 64);
        if at.size.get() <= 32 {
            let old = load_partial_u64(self.get(at), at.size);
            store_partial_u64(self.get_mut(at), value, at.size);
            old
        } else {
            std::mem::replace(self.get_mut_u64(at.offset), value)
        }
    }
    fn set_tv_bool(&mut self, at: StackOffset, value: bool) {
        self.set_tv_u64(at.to_ref(SCALAR_VSIZE), value.into());
    }

    fn get_fv_item(&self, at: StackOffset) -> FvLogicValue {
        let (spc, val) = self.get_fv_u64(at.to_ref(SCALAR_VSIZE));
        FvLogicValue::from_repr(((spc as u8) << 1) | (val as u8))
    }
    fn get_fv_u64(&self, at: StackRef) -> (u64, u64) {
        debug_assert!(at.size.get() <= 64);
        if at.size.get() <= 16 {
            let dsize = at.size.checked_mul(VectorSize::new(2).unwrap()).unwrap();
            let src = self.get(at.offset.to_ref(dsize));
            fv_unpack_u64(load_partial_u64(src, dsize), at.size)
        } else {
            let [spc, val] = self.get_u64_slice(at.offset, 2) else {
                unreachable!()
            };
            (*spc, *val)
        }
    }
    fn set_fv_u64(&mut self, at: StackRef, spc: u64, val: u64) -> (u64, u64) {
        debug_assert!(at.size.get() <= 64);
        if at.size.get() <= 16 {
            let dsize = at.size.checked_mul(VectorSize::new(2).unwrap()).unwrap();
            let dst = self.get_mut(at.offset.to_ref(dsize));
            let old = load_partial_u64(dst, dsize);
            store_partial_u64(dst, fv_pack_u64(spc, val, at.size), dsize);
            fv_unpack_u64(old, at.size)
        } else {
            let s = self.get_mut_u64_slice(at.offset, 2);
            (
                std::mem::replace(&mut s[0], spc),
                std::mem::replace(&mut s[1], val),
            )
        }
    }

    fn get_disjoint_u64_dst_src(
        &mut self,
        dst: (StackOffset, usize),
        src: (StackOffset, usize),
    ) -> (&mut [u64], &[u64]) {
        debug_assert_eq!(dst.0.0 % 8, 0);
        debug_assert_eq!(src.0.0 % 8, 0);
        get_disjoint_dst_src(&mut self.0, dst.0.0 / 8, dst.1, src.0.0 / 8, src.1)
    }

    fn get_disjoint_u64_dst_s1_s2(
        &mut self,
        dst: (StackOffset, usize),
        src1: (StackOffset, usize),
        src2: (StackOffset, usize),
    ) -> (&mut [u64], &[u64], &[u64]) {
        debug_assert_eq!(dst.0.0 % 8, 0);
        debug_assert_eq!(src1.0.0 % 8, 0);
        debug_assert_eq!(src2.0.0 % 8, 0);
        get_disjoint_dst_s1_s2(
            &mut self.0,
            dst.0.0 / 8,
            dst.1,
            src1.0.0 / 8,
            src1.1,
            src2.0.0 / 8,
            src2.1,
        )
    }

    fn get_disjoint_u8_dst_src(&mut self, dst: StackRef, src: StackRef) -> (&mut [u8], &[u8]) {
        let dst_bytes = dst.size.get().div_ceil(8) as usize;
        let src_bytes = src.size.get().div_ceil(8) as usize;
        get_disjoint_dst_src(
            bytemuck::cast_slice_mut(&mut self.0),
            dst.offset.0,
            dst_bytes,
            src.offset.0,
            src_bytes,
        )
    }

    fn get_disjoint_u8_dst_s1_s2(
        &mut self,
        dst: StackRef,
        src1: StackRef,
        src2: StackRef,
    ) -> (&mut [u8], &[u8], &[u8]) {
        get_disjoint_dst_s1_s2(
            bytemuck::cast_slice_mut(&mut self.0),
            dst.offset.0,
            dst.size.get().div_ceil(8) as usize,
            src1.offset.0,
            src1.size.get().div_ceil(8) as usize,
            src2.offset.0,
            src2.size.get().div_ceil(8) as usize,
        )
    }

    fn load_tv_bits(&self, at: StackRef) -> Bits {
        // @Performance: We should make a specialized path for u64
        Bits::load_from_slice(self.get(at), at.size)
    }
    fn load_fv_bits(&self, at: StackRef) -> Bits {
        if at.size.get() <= 32 {
            let (spc, val) = self.get_fv_u64(at);
            Bits::from_four_value_u64(at.size, spc as u32, val as u32)
        } else {
            Bits::from_boxed_slice(
                vogls_ir::Mode::FourValue,
                at.size,
                self.get_u64_slice(at.offset, 2 * at.size.get().div_ceil(64) as usize)
                    .into(),
            )
        }
    }

    pub fn store_bits(&mut self, dst: StackRef, logic_mode: LogicMode, value: &Bits) {
        match (value.as_data_ref(), logic_mode) {
            (BitsDataRef::InlineTv(v), LogicMode::TwoValue) => _ = self.set_tv_u64(dst, v),
            (BitsDataRef::InlineTv(v), LogicMode::FourValue) => {
                _ = self.set_fv_u64(dst, 1u64.unbounded_shl(dst.size.get()).wrapping_sub(1), v)
            }
            (BitsDataRef::InlineFv(..), LogicMode::TwoValue) => unreachable!(),
            (BitsDataRef::InlineFv(spc, val), LogicMode::FourValue) => {
                _ = self.set_fv_u64(dst, spc as u64, val as u64);
            }
            (BitsDataRef::SeparateTv(items), LogicMode::TwoValue)
            | (BitsDataRef::SeparateFv(items), LogicMode::FourValue) => {
                self.get_mut_u64_slice(dst.offset, items.len())
                    .copy_from_slice(items);
            }
            (BitsDataRef::SeparateTv(items), LogicMode::FourValue) => {
                let target = self.get_mut_u64_slice(dst.offset, items.len() * 2);
                fv_set_no_special(target, dst.size);
                target[items.len()..].copy_from_slice(items);
            }
            (BitsDataRef::SeparateFv(..), LogicMode::TwoValue) => unreachable!(),
        }
    }

    fn set_fv_scalar(&mut self, at: StackOffset, value: FvLogicValue) {
        let (spc, val) = ((value as u64) >> 1, (value as u64) & 1);
        self.set_fv_u64(at.to_ref(SCALAR_VSIZE), spc, val);
    }
}

#[derive(Clone, Debug)]
pub struct Event {
    /// Which process is scheduled.
    pub process: VmProcessKey,
    /// Where to start execution.
    pub ip: usize,
}

#[derive(Debug)]
pub struct ScheduledEvent {
    pub at: Timestamp,
    pub event: Event,
}

impl PartialEq for ScheduledEvent {
    fn eq(&self, other: &Self) -> bool {
        self.at == other.at
    }
}
impl Eq for ScheduledEvent {}
impl PartialOrd for ScheduledEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.at.partial_cmp(&self.at)
    }
}
impl Ord for ScheduledEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        other.at.cmp(&self.at)
    }
}

enum EvalOutcome {
    Next,
    Error,
    Exit,
}

fn update_watchers(
    sig: VmSignalKey,
    stack: &Stack,
    signals: &[StackRef],
    watches: &mut [Vec<ListenerKey>],
    listeners: &mut SlotMap<ListenerKey, Event>,
    regions: &mut Regions,
    trace: Option<&mut vogls_trace::Trace>,
) {
    let start = regions.active.len();
    let watchers = &mut watches[sig.0 as usize];
    for watcher in watchers.iter() {
        if let Some(event) = listeners.remove(*watcher) {
            regions.active.push(event);
        }
    }
    watchers.clear();

    if let Some(trace) = trace {
        let woken_start = trace.woken.len() as u64;
        trace
            .woken
            .extend(regions.active[start..].iter().map(|e| e.process.0));
        let woken_range = woken_start..trace.woken.len() as u64;
        trace.driven.push((
            sig.0,
            stack.load_tv_bits(signals[sig.0 as usize]),
            woken_range,
        ));
    }
}

pub fn drive_bits(
    stack: &mut Stack,
    dst: StackRef,
    src: StackRef,
    partial: Option<u32>,
    logic_mode: LogicMode,
) -> bool {
    if partial.is_some() || dst.size != src.size {
        let partial = partial.unwrap_or(0);

        return match logic_mode {
            LogicMode::TwoValue if dst.size.get() <= 32 => {
                let (dst_s, src_s) = stack.get_disjoint_u8_dst_src(dst, src);
                tv_s_set(dst_s, src_s, dst.size, partial, src.size)
            }
            LogicMode::TwoValue => {
                let mut src_s = [0u64];
                let (dst_s, src_s) = if src.size.get() <= 32 {
                    src_s[0] = stack.get_tv_u64(src);
                    (
                        stack.get_mut_u64_slice(dst.offset, dst.size.get().div_ceil(64) as usize),
                        &src_s[..],
                    )
                } else {
                    let dst_nwords = dst.size.get().div_ceil(64) as usize;
                    let src_nwords = src.size.get().div_ceil(64) as usize;
                    stack.get_disjoint_u64_dst_src(
                        (dst.offset, dst_nwords),
                        (src.offset, src_nwords),
                    )
                };

                tv_l_set(dst_s, src_s, dst.size, partial, src.size)
            }
            LogicMode::FourValue if dst.size.get() <= 16 => {
                let (src_spc, src_val) = stack.get_fv_u64(src);
                let (old_spc, old_val) = stack.get_fv_u64(dst);

                let mask = (1u64 << src.size.get()) - 1;
                let mask = mask << partial;
                let new_spc = (src_spc << partial) | (old_spc & !mask);
                let new_val = (src_val << partial) | (old_val & !mask);
                stack.set_fv_u64(dst, new_spc, new_val);
                old_spc != new_spc || old_val != new_val
            }
            _ => {
                let mut src_s = [0u64, 0u64];
                let dst_nwords = dst.size.get().div_ceil(64) as usize;
                let (dst_s, src_s) = if src.size.get() <= 16 {
                    (src_s[0], src_s[1]) = stack.get_fv_u64(src);
                    (
                        stack.get_mut_u64_slice(dst.offset, 2 * dst_nwords),
                        &src_s[..],
                    )
                } else {
                    let src_nwords = src.size.get().div_ceil(64) as usize;
                    stack.get_disjoint_u64_dst_src(
                        (dst.offset, 2 * dst_nwords),
                        (src.offset, 2 * src_nwords),
                    )
                };

                let mut updated = false;
                updated |= tv_l_set(
                    &mut dst_s[..dst_nwords],
                    &src_s[..src_s.len() / 2],
                    dst.size,
                    partial,
                    src.size,
                );
                updated |= tv_l_set(
                    &mut dst_s[dst_nwords..],
                    &src_s[src_s.len() / 2..],
                    dst.size,
                    partial,
                    src.size,
                );
                updated
            }
        };
    }

    let size = dst.size;
    match logic_mode {
        LogicMode::TwoValue if size.get() <= 32 => {
            let src = stack.get_tv_u64(src);
            let dst = stack.set_tv_u64(dst, src);
            dst != src
        }
        LogicMode::FourValue if size.get() <= 16 => {
            let (spc, val) = stack.get_fv_u64(src);
            let (dspc, dval) = stack.set_fv_u64(dst, spc, val);
            dspc != spc || val != dval
        }
        LogicMode::TwoValue | LogicMode::FourValue => {
            let mut nwords = size.get().div_ceil(64) as usize;
            if logic_mode == LogicMode::FourValue {
                nwords *= 2;
            }

            let (dst, src) =
                stack.get_disjoint_u64_dst_src((dst.offset, nwords), (src.offset, nwords));
            let mut updated = false;
            for i in 0..nwords {
                updated |= dst[i] != src[i];
                dst[i] = src[i];
            }
            updated
        }
    }
}

#[derive(Debug, Clone)]
pub struct VcdScope {
    pub name: String,
    pub items: Vec<VcdScopeItem>,
}

impl VcdScope {
    fn lower(v: &vogls_ir::vcd::VcdScope, map: &HashMap<SignalKey, VmSignalKey>) -> VcdScope {
        VcdScope {
            name: v.name.clone(),
            items: v
                .items
                .iter()
                .map(|i| VcdScopeItem::lower(i, map))
                .collect(),
        }
    }

    fn write_to(&self, f: &mut impl std::io::Write, info: &[SignalInfo]) -> std::io::Result<()> {
        let Self { name, items } = self;
        writeln!(f, "$scope module {name} $end")?;
        for item in items {
            item.write_to(f, info)?;
        }
        writeln!(f, "$upscope $end")?;
        Ok(())
    }

    fn extend_into(
        &self,
        tracked: &mut HashMap<VmSignalKey, Option<NonZeroUsize>>,
        values: &mut Vec<VmSignalKey>,
    ) {
        for i in &self.items {
            i.extend_into(tracked, values);
        }
    }
}

impl VcdScopeItem {
    fn write_to(&self, f: &mut impl std::io::Write, info: &[SignalInfo]) -> std::io::Result<()> {
        match self {
            VcdScopeItem::Scope(scope) => scope.write_to(f, info),
            VcdScopeItem::Variable(k) => {
                let VcdVariable {
                    signal,
                    ty,
                    msb,
                    lsb,
                } = k;
                let SignalInfo { name } = &info[signal.0 as usize];
                let size = VectorSize::new((msb.abs_diff(*lsb) + 1) as u32).unwrap();
                let idx = k.signal.0;
                write!(f, "$var ")?;
                f.write_all(
                    match ty {
                        NetType::Integer => "integer",
                        NetType::Register => "reg",
                        NetType::Wire => "wire",
                    }
                    .as_bytes(),
                )?;
                write!(f, " {size} W{idx:X} {name} ")?;
                if size.get() > 1 {
                    write!(f, "[{msb}:{lsb}] ")?;
                }
                writeln!(f, "$end")
            }
        }
    }

    fn extend_into(
        &self,
        tracked: &mut HashMap<VmSignalKey, Option<NonZeroUsize>>,
        values: &mut Vec<VmSignalKey>,
    ) {
        match self {
            VcdScopeItem::Scope(s) => s.extend_into(tracked, values),
            VcdScopeItem::Variable(k) => {
                tracked.entry(k.signal).or_insert_with(|| {
                    values.push(k.signal);
                    Some(NonZeroUsize::new(values.len()).unwrap())
                });
            }
        }
    }
}

impl VcdScopeItem {
    fn lower(v: &vogls_ir::vcd::VcdScopeItem, map: &HashMap<SignalKey, VmSignalKey>) -> Self {
        match v {
            vogls_ir::vcd::VcdScopeItem::Scope(v) => Self::Scope(VcdScope::lower(v, map)),
            vogls_ir::vcd::VcdScopeItem::Variable(v) => Self::Variable(VcdVariable {
                signal: map[&v.signal],
                ty: v.ty,
                msb: v.msb,
                lsb: v.lsb,
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VcdVariable {
    pub signal: VmSignalKey,
    pub ty: NetType,
    pub msb: i64,
    pub lsb: i64,
}

#[derive(Debug, Clone)]
pub enum VcdScopeItem {
    Scope(VcdScope),
    Variable(VcdVariable),
}

pub struct VcdOutput {
    start_ts: Timestamp,
    last_ts: Timestamp,
    paused: bool,
    scope: VcdScope,
    tracked: HashMap<VmSignalKey, Option<NonZeroUsize>>,
    updated_this_time_step: Vec<VmSignalKey>,
    writer: Box<dyn std::io::Write>,
}
impl VcdOutput {
    fn dump_time_step(
        &mut self,
        ctx: &Context,
        stack: &Stack,
        signals: &[StackRef],
        signal_info: &[SignalInfo],
        finish: bool,
    ) -> std::io::Result<()> {
        let f = &mut self.writer;
        if self.start_ts == ctx.time {
            writeln!(f, "$version Generated by VoGLS $end")?;
            // @TODO
            writeln!(f, "$date @TODO $end")?;
            writeln!(f, "$timescale 1ns $end")?;
            self.scope.write_to(f, signal_info)?;
            writeln!(f, "$enddefinitions $end")?;
        }

        // Only print for the timestamp if something actually happened.
        let mut show_for_timestamp = !self.updated_this_time_step.is_empty();
        show_for_timestamp |= finish;
        show_for_timestamp &= self.last_ts != ctx.time;
        if !show_for_timestamp {
            return Ok(());
        }

        self.last_ts = ctx.time;
        writeln!(f, "#{}", ctx.time)?;
        for signal in &self.updated_this_time_step {
            let bits = signals[signal.0 as usize];
            let idx = signal.0;
            let bits = stack.load_tv_bits(bits);
            if bits.size().get() > 1 {
                f.write_all(&[b'b'])?;
            }
            format_bits(f, &bits, Padding::ZeroPaddedToSize, Base::Binary).unwrap();
            if bits.size().get() > 1 {
                f.write_all(&[b' '])?;
            }
            writeln!(f, "W{idx:X}")?;
            *self.tracked.get_mut(signal).unwrap() = None;
        }

        self.updated_this_time_step.clear();
        Ok(())
    }
}

impl Event {
    fn evaluate(
        mut self,
        ctx: &mut Context,
        processes: &[VmProcess],
        schedule: &mut BTreeMap<Timestamp, Vec<Event>>,
        regions: &mut Regions,
        signals: &mut [StackRef],
        listeners: &mut SlotMap<ListenerKey, Event>,
        watches: &mut [Vec<ListenerKey>],
        stack: &mut Stack,
        vcd: &mut Option<VcdOutput>,
        mut trace: Option<&mut vogls_trace::Trace>,
    ) -> EvalOutcome {
        let Event {
            process: process_key,
            ip,
        } = &mut self;

        let process = &processes[process_key.0 as usize];

        loop {
            let instr = &process.instructions[*ip];

            *ip += 1;
            ctx.instruction_count += 1;

            let outcome = 'instruction: {
                use VmInstruction as I;
                match instr {
                    I::Constant(dst, value) => execution::exec_constant(stack, *dst, value),

                    I::TvUnary(dst, op, src) => {
                        execution::tv::exec_tv_unary(stack, *dst, *op, *src)
                    }
                    I::TvResize(dst, op, src) => {
                        execution::tv::exec_tv_resize(stack, *dst, *op, *src)
                    }
                    I::TvBinaryArithmetic(dst, op, lhs, rhs) => {
                        execution::tv::exec_tv_bin_arith(stack, *dst, *op, *lhs, *rhs)
                    }
                    I::TvBinaryComparison(dst, op, lhs, rhs) => {
                        execution::tv::exec_tv_bin_cmp(stack, *dst, *op, *lhs, *rhs)
                    }
                    I::TvShift(dst, op, src, offset) => {
                        execution::tv::exec_tv_shift(stack, *dst, *op, *src, *offset)
                    }
                    I::TvSelectBit(dst, src, idx) => {
                        execution::tv::exec_tv_select_bit(stack, *dst, *src, *idx)
                    }
                    I::TvConcat(dst, lhs, rhs) => {
                        execution::tv::exec_tv_concat(stack, *dst, *lhs, *rhs)
                    }

                    I::FvUnary(dst, op, src) => {
                        execution::fv::exec_fv_unary(stack, *dst, *op, *src)
                    }
                    I::FvResize(dst, op, src) => {
                        execution::fv::exec_fv_resize(stack, *dst, *op, *src)
                    }
                    I::FvBinaryArithmetic(dst, op, lhs, rhs) => {
                        execution::fv::exec_fv_bin_arith(stack, *dst, *op, *lhs, *rhs)
                    }
                    I::FvBinaryComparison(dst, op, lhs, rhs) => {
                        execution::fv::exec_fv_bin_cmp(stack, *dst, *op, *lhs, *rhs)
                    }
                    I::FvShift(dst, op, src, offset) => {
                        execution::fv::exec_fv_shift(stack, *dst, *op, *src, *offset)
                    }
                    I::FvSelectBit(dst, src, idx) => {
                        execution::fv::exec_fv_select_bit(stack, *dst, *src, *idx)
                    }
                    I::FvConcat(dst, lhs, rhs) => {
                        execution::fv::exec_fv_concat(stack, *dst, *lhs, *rhs)
                    }

                    I::TvToFv(dst, src) => {
                        let size = dst.size;
                        if size.get() <= 32 {
                            let v = stack.get_tv_u64(src.to_ref(size));
                            stack.set_fv_u64(
                                *dst,
                                1u64.unbounded_shl(size.get()).wrapping_sub(1),
                                v,
                            );
                        } else {
                            let nwords = size.get().div_ceil(64) as usize;
                            let (dst, src) = stack
                                .get_disjoint_u64_dst_src((dst.offset, nwords * 2), (*src, nwords));
                            fv_set_no_special(dst, size);
                            dst[nwords..].copy_from_slice(src);
                        }
                    }
                    I::FvToTv(dst, src) => {
                        let size = dst.size;
                        if size.get() <= 32 {
                            let (spc, val) = stack.get_fv_u64(src.to_ref(size));
                            stack.set_tv_u64(*dst, spc & val);
                        } else {
                            let nwords = size.get().div_ceil(64) as usize;
                            let (dst, src) = stack
                                .get_disjoint_u64_dst_src((dst.offset, nwords), (*src, nwords * 2));
                            for i in 0..nwords {
                                dst[i] = src[i] & src[nwords + i];
                            }
                        }
                    }

                    I::Intrinsic(dst, op, args) => {
                        use VmIntrinsicOp as O;

                        match op.as_ref() {
                            O::Display(f) => {
                                f.write_to(
                                    &mut ctx.stdout,
                                    args.iter().map(|(o, s)| stack.load_tv_bits(o.to_ref(*s))),
                                )
                                .unwrap();
                            }
                            O::AssertTv(f) => {
                                let condition =
                                    stack.get_tv_u64(args[0].0.to_ref(SCALAR_VSIZE)) != 0;
                                if !condition {
                                    f.write_to(
                                        &mut ctx.stdout,
                                        args[1..].iter().map(|(o, s)| {
                                            Bits::load_from_slice(stack.get(o.to_ref(*s)), *s)
                                        }),
                                    )
                                    .unwrap();
                                    break 'instruction Some(EvalOutcome::Error);
                                }
                            }
                            O::AssertFv(f) => {
                                let condition = stack.get_fv_item(args[0].0) == FvLogicValue::L1;
                                if !condition {
                                    f.write_to(
                                        &mut ctx.stdout,
                                        args[1..]
                                            .iter()
                                            .map(|(o, s)| stack.load_fv_bits(o.to_ref(*s))),
                                    )
                                    .unwrap();
                                    break 'instruction Some(EvalOutcome::Error);
                                }
                            }
                            O::VcdOpenFile(path) => {
                                if vcd.is_some() {
                                    writeln!(&mut ctx.stderr, "ERR! VCD opened a second file")
                                        .unwrap();
                                    break 'instruction Some(EvalOutcome::Error);
                                }

                                *vcd = Some(VcdOutput {
                                    start_ts: ctx.time,
                                    last_ts: Timestamp::MAX,
                                    paused: false,
                                    scope: VcdScope {
                                        name: "top".to_string(),
                                        items: Vec::new(),
                                    },
                                    tracked: HashMap::new(),
                                    updated_this_time_step: Vec::new(),
                                    writer: Box::new(std::fs::File::create(path).unwrap()),
                                });
                            }
                            O::VcdAppendModule(scope) => {
                                let Some(vcd) = vcd.as_mut() else {
                                    writeln!(
                                        &mut ctx.stderr,
                                        "ERR! Dumping vars without having a VCD file open"
                                    )
                                    .unwrap();
                                    break 'instruction Some(EvalOutcome::Error);
                                };
                                if vcd.start_ts != ctx.time {
                                    writeln!(
                                        &mut ctx.stderr,
                                        "ERR! Dumping vars over several simulation times"
                                    )
                                    .unwrap();
                                    break 'instruction Some(EvalOutcome::Error);
                                }

                                scope
                                    .extend_into(&mut vcd.tracked, &mut vcd.updated_this_time_step);
                                vcd.scope = scope.clone();
                            }
                            O::VcdPause => _ = vcd.as_mut().map(|vcd| vcd.paused = true),
                            O::VcdResume => _ = vcd.as_mut().map(|vcd| vcd.paused = false),
                            O::Time => _ = stack.set_tv_u64(dst.to_ref(TIME_VSIZE), ctx.time),
                            O::Finish => {
                                writeln!(&mut ctx.stdout, "[FINISH]").unwrap();
                                break 'instruction Some(EvalOutcome::Exit);
                            }
                        }
                    }
                    I::Drive(sig, src, partial) => {
                        let partial = match (partial, ctx.logic_mode) {
                            (None, _) => None,
                            (Some(offset), LogicMode::TwoValue) => {
                                Some(stack.load_exact_tv_u32(*offset))
                            }
                            (Some(offset), LogicMode::FourValue) => {
                                let (spc, val) = stack.load_exact_fv_u32(*offset);
                                if !spc != 0 {
                                    break 'instruction None;
                                }
                                Some(val)
                            }
                        };

                        let updated = drive_bits(
                            stack,
                            signals[sig.0 as usize],
                            *src,
                            partial,
                            ctx.logic_mode,
                        );

                        if updated {
                            update_watchers(
                                *sig,
                                stack,
                                signals,
                                watches,
                                listeners,
                                regions,
                                trace.as_deref_mut(),
                            );
                            if let Some(vcd) = vcd.as_mut()
                                && !vcd.paused
                                && let Some(idx) = vcd.tracked.get_mut(sig)
                            {
                                idx.get_or_insert_with(|| {
                                    vcd.updated_this_time_step.push(*sig);
                                    NonZeroUsize::new(vcd.updated_this_time_step.len()).unwrap()
                                });
                            }
                        }
                    }
                    I::Wait(time) => {
                        schedule.entry(ctx.time + time.0).or_default().push(self);
                        if let Some(trace) = trace.as_deref_mut() {
                            let vogls_trace::Event::Evaluation(_, _, stop_reason) =
                                trace.events.last_mut().unwrap()
                            else {
                                unreachable!();
                            };
                            *stop_reason = vogls_trace::EventStopReason::Wait(ctx.time + time.0);
                        }
                        if ctx.itrace {
                            instr.itrace(stack, signals, ctx.logic_mode);
                        }
                        return EvalOutcome::Next;
                    }
                    I::WaitRegion(region) => {
                        if *region == 0 {
                            regions.active.push(self);
                        } else {
                            regions.other[*region as usize - 1].push(self);
                        }
                        if let Some(trace) = trace.as_deref_mut() {
                            let vogls_trace::Event::Evaluation(_, _, stop_reason) =
                                trace.events.last_mut().unwrap()
                            else {
                                unreachable!();
                            };
                            *stop_reason = vogls_trace::EventStopReason::WaitRegion(*region);
                        }
                        if ctx.itrace {
                            instr.itrace(stack, signals, ctx.logic_mode);
                        }
                        return EvalOutcome::Next;
                    }
                    I::Watch(watch_signals) => {
                        let listener_key = listeners.insert(self);
                        for signal in watch_signals {
                            watches[signal.0 as usize].push(listener_key);
                        }
                        if let Some(trace) = trace.as_mut() {
                            let watch_range_start = trace.watches.len() as u64;
                            trace.watches.extend(watch_signals.iter().map(|s| s.0));
                            let vogls_trace::Event::Evaluation(_, _, stop_reason) =
                                trace.events.last_mut().unwrap()
                            else {
                                unreachable!();
                            };
                            *stop_reason = vogls_trace::EventStopReason::WatchSignals(
                                watch_range_start..trace.watches.len() as u64,
                            );
                        }
                        if ctx.itrace {
                            instr.itrace(stack, signals, ctx.logic_mode);
                        }
                        return EvalOutcome::Next;
                    }

                    I::Jump(offset) => *ip = *offset,
                    I::Branch(cond, true_offset, false_offset) => {
                        let is_true = stack.get_tv_u64(cond.to_ref(SCALAR_VSIZE)) & 1 != 0;
                        if is_true {
                            *ip = *true_offset;
                        } else {
                            *ip = *false_offset;
                        }
                    }
                    I::Halt => {
                        break 'instruction Some(EvalOutcome::Next);
                    }
                }

                None
            };

            if ctx.itrace {
                instr.itrace(stack, signals, ctx.logic_mode);
            }

            if let Some(outcome) = outcome {
                return outcome;
            }
        }
    }
}

#[derive(Clone)]
pub struct SignalInfo {
    pub name: String,
}

pub fn run(
    ctx: &mut Context,
    processes: &[VmProcess],
    regions: &mut Regions,
    signals: &mut [StackRef],
    signal_info: &[SignalInfo],
    listeners: &mut SlotMap<ListenerKey, Event>,
    watches: &mut [Vec<ListenerKey>],
    mut trace: Option<&mut vogls_trace::Trace>,
    stack: &mut Stack,
    max_time: u64,
) -> Result<(), ()> {
    let mut schedule = BTreeMap::<Timestamp, Vec<Event>>::new();
    let mut vcd = None;
    'region_loop: loop {
        while let Some(event) = regions.active.pop() {
            if let Some(trace) = trace.as_deref_mut() {
                trace.events.push(vogls_trace::Event::Evaluation(
                    event.process.0 as u64,
                    trace.driven.len() as u64..trace.driven.len() as u64,
                    vogls_trace::EventStopReason::Halt,
                ));
            }
            if cfg!(vm_profile) {
                ctx.event_count += 1;
            }

            let outcome = event.evaluate(
                ctx,
                processes,
                &mut schedule,
                regions,
                signals,
                listeners,
                watches,
                stack,
                &mut vcd,
                trace.as_deref_mut(),
            );

            if ctx.itrace {
                eprintln!();
            }

            if let Some(trace) = trace.as_deref_mut() {
                match trace.events.last_mut().unwrap() {
                    vogls_trace::Event::Drive(_, drive) => {
                        _ = drive.take_if(|d| *d == trace.driven.len() as u64)
                    }
                    vogls_trace::Event::Evaluation(_, driven, _) => {
                        driven.end = trace.driven.len() as u64
                    }
                    vogls_trace::Event::Time(_) => {}
                }
            }

            match outcome {
                EvalOutcome::Next => continue,
                EvalOutcome::Error => return Err(()),
                EvalOutcome::Exit => break 'region_loop,
            }
        }

        for (i, region) in regions.other.iter_mut().enumerate() {
            if !region.is_empty() {
                regions.other_dispatched[i].clear();
                std::mem::swap(&mut regions.active, region);
                continue 'region_loop;
            }
        }

        // Dump the VCD updates for this simulation time.
        if let Some(vcd) = vcd.as_mut() {
            vcd.dump_time_step(ctx, stack, signals, signal_info, false)
                .unwrap();
        }

        let Some((at, events)) = schedule.pop_first() else {
            break;
        };

        ctx.time = at;
        if let Some(trace) = trace.as_deref_mut() {
            trace.events.push(vogls_trace::Event::Time(ctx.time));
        }
        if ctx.time > max_time {
            break;
        }
        regions.active = events;
    }

    if let Some(vcd) = vcd.as_mut() {
        vcd.dump_time_step(ctx, stack, signals, signal_info, true)
            .unwrap();
        vcd.writer.flush().unwrap();
    }

    if cfg!(vm_profile) {
        writeln!(ctx.stdout, "Stats:",).unwrap();
        writeln!(ctx.stdout, "  # Instructions: {}", ctx.instruction_count).unwrap();
        writeln!(ctx.stdout, "  # Events:       {}", ctx.event_count).unwrap();
        writeln!(ctx.stdout, "  # Stack size:   {}", stack.0.len()).unwrap();
    }

    Ok(())
}

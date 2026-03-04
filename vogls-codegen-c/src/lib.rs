use std::fmt::Write;
use std::{fmt, io};

use vogls_bits::format::{BitsFormatBase, BitsFormatOptions, BitsFormatWidth};
use vogls_codegen::{
    HeapBuilder, HeapOffset, HeapRef, resolve_heap_map, resolve_var_logic_mode_map,
};
use vogls_ir::dyn_format_string::DynFormatString;
use vogls_ir::{
    BasicBlockKey, BasicBlockTerminator, GlobalContext, INTEGER_VSIZE, Instruction, LogicMode,
    Process, ProcessKey, ResizeOp, SCALAR_VSIZE, SignalKey, TIME_VSIZE, UnaryOp, VariableKey,
    VectorSize,
};
use vogls_utils::{IndexMap, IndexSet, VgHashMap, VgHashSet};

pub mod runtime;

mod binary;
mod resize;
mod unary;

#[derive(Clone, Copy)]
struct CIdent(u64);
impl fmt::Display for CIdent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char('t')?;
        self.0.fmt(f)
    }
}

#[derive(Clone, Copy)]
struct CVar {
    ident: CIdent,
    ty: CType,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CType {
    size: VectorSize,
    mode: LogicMode,
}

#[derive(Clone, Copy)]
enum CElementType {
    U8,
    U16,
    U32,
    U64,
}

impl CElementType {
    const fn size(self) -> VectorSize {
        VectorSize::new(match self {
            CElementType::U8 => 8,
            CElementType::U16 => 16,
            CElementType::U32 => 32,
            CElementType::U64 => 64,
        })
        .unwrap()
    }

    const fn signed_ty_str(self) -> &'static str {
        match self {
            CElementType::U8 => "int8_t",
            CElementType::U16 => "int16_t",
            CElementType::U32 => "int32_t",
            CElementType::U64 => "int64_t",
        }
    }
}

impl CType {
    fn is_array(self) -> bool {
        self.array_size().is_some()
    }

    fn array_size(self) -> Option<u32> {
        match self.mode {
            LogicMode::TwoValue => (self.size.get() > 64).then_some(self.size.get().div_ceil(64)),
            LogicMode::FourValue => {
                (self.size.get() > 32).then_some(2 * self.size.get().div_ceil(32))
            }
        }
    }

    fn element_type(self) -> CElementType {
        use LogicMode as M;
        match self.mode {
            M::FourValue if self.size.get() <= 4 => CElementType::U8,
            M::TwoValue if self.size.get() <= 8 => CElementType::U8,

            M::FourValue if self.size.get() <= 8 => CElementType::U16,
            M::TwoValue if self.size.get() <= 16 => CElementType::U16,

            M::FourValue if self.size.get() <= 16 => CElementType::U32,
            M::TwoValue if self.size.get() <= 32 => CElementType::U32,

            M::TwoValue | M::FourValue => CElementType::U64,
        }
    }
}

impl fmt::Display for CElementType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CElementType::U8 => "uint8_t",
            CElementType::U16 => "uint16_t",
            CElementType::U32 => "uint32_t",
            CElementType::U64 => "uint64_t",
        })
    }
}

const INDENT: &str = "    ";

pub fn process_to_procedure_name(process: &Process, idx: usize) -> String {
    format!(
        "vogls_proc_{idx}_{}",
        &process
            .name
            .replace(|c: char| !c.is_ascii_alphanumeric(), "_")
    )
}

pub struct Listener {
    pub offset: usize,
    pub process_idx: usize,
    pub process_key: ProcessKey,
    pub state: u32,
}

#[derive(Default)]
pub struct ListenerBuilder {
    pub map: VgHashMap<SignalKey, Vec<Listener>>,
    pub top: usize,
}

impl ListenerBuilder {
    pub fn insert(
        &mut self,
        signal: SignalKey,
        process_idx: usize,
        process_key: ProcessKey,
        state: u32,
    ) {
        let offset = self.top;
        self.top += 1;
        self.map.entry(signal).or_default().push(Listener {
            offset,
            process_idx,
            process_key,
            state,
        });
    }
}

pub fn lower_process(
    f: &mut impl io::Write,
    process_key: ProcessKey,
    process_idx: usize,
    gl: &GlobalContext,
    heap_builder: &mut HeapBuilder,
    listener_builder: &mut ListenerBuilder,
    io_signals: &IndexMap<SignalKey, HeapRef>,
) -> io::Result<()> {
    use Instruction as I;

    let process = &gl.processes[process_key];

    let mut bb_stack = Vec::new();
    let mut bb_seen = VgHashSet::<BasicBlockKey>::default();
    let mut bb_phis = VgHashMap::<BasicBlockKey, Vec<(VariableKey, VariableKey)>>::default();

    let mut var_mode = VgHashMap::<VariableKey, LogicMode>::default();
    let mut conv_map = VgHashMap::<VariableKey, HeapOffset>::default();
    let mut heap_map = VgHashMap::default();

    resolve_var_logic_mode_map(
        process.entry,
        gl,
        &mut bb_stack,
        &mut bb_seen,
        &mut var_mode,
        &mut conv_map,
    );
    resolve_heap_map(
        process.entry,
        gl,
        &mut bb_stack,
        &mut bb_seen,
        &var_mode,
        &mut conv_map,
        heap_builder,
        &mut heap_map,
        &mut bb_phis,
    );

    // @Performance. Amortize buffer.
    let mut buffer = Vec::<u8>::new();
    let procedure = process_to_procedure_name(process, process_idx);

    writeln!(
        f,
        "void {procedure}(int state, uint64_t *heap, schedule_t *schedule, uint64_t time, uint64_t *is_scheduled, uint64_t *listening, uint64_t *last_active_time, cold_context_t *cldctx) {{",
    )?;

    let mut bb_ident = IndexSet::<BasicBlockKey>::new();
    let mut states_set = IndexSet::<BasicBlockKey>::new();

    bb_stack.push(process.entry);
    bb_ident.insert(process.entry);
    states_set.insert(process.entry);
    while let Some(bb_key) = bb_stack.pop() {
        let bb = &gl.bbs[bb_key];
        bb.terminator.for_each_bb(|bb| {
            if bb_ident.insert(bb) {
                bb_stack.push(bb);
            }
        });

        match bb.terminator {
            BasicBlockTerminator::Wait(target, _)
            | BasicBlockTerminator::VariableWait(target, _)
            | BasicBlockTerminator::WaitRegion(target, _)
            | BasicBlockTerminator::Watch(target, _) => {
                states_set.insert(target);
            }
            BasicBlockTerminator::Jump(..)
            | BasicBlockTerminator::Branch(..)
            | BasicBlockTerminator::Halt => {}
        }
    }

    use std::io::Write;
    if states_set.len() > 1 {
        writeln!(buffer, "{INDENT}switch (state) {{")?;
        for (i, state) in states_set.iter().enumerate() {
            let bb_ident = bb_ident.get_index(state).unwrap();
            writeln!(buffer, "{INDENT}{INDENT}case {i}: goto L{bb_ident};")?;
        }
        writeln!(buffer, "{INDENT}}}")?;
        writeln!(buffer)?;
    }

    let mut tmp_counter = 0u64;
    macro_rules! claim_tmp {
        ($size:expr, $mode:expr) => {{
            let t = CVar {
                ident: CIdent(tmp_counter),
                ty: CType {
                    size: $size,
                    mode: $mode,
                },
            };
            tmp_counter += 1;

            write!(f, "{INDENT}{} {}", t.ty.element_type(), t.ident)?;
            if let Some(array_size) = t.ty.array_size() {
                write!(f, "[{array_size}]")?;
            }
            writeln!(f, ";")?;
            t
        }};
    }

    bb_seen.clear();
    bb_seen.insert(process.entry);
    bb_stack.push(process.entry);
    while let Some(bb_key) = bb_stack.pop() {
        let bb = &gl.bbs[bb_key];
        bb.terminator.for_each_bb(|bb| {
            if bb_seen.insert(bb) {
                bb_stack.push(bb);
            }
        });

        let ident = bb_ident.get_index(&bb_key).unwrap();

        writeln!(buffer, "L{ident}:")?;

        for i in &bb.instrs {
            writeln!(buffer, "{INDENT}// {i:?}")?;
            match i {
                I::Constant(dst, bits) => {
                    let mode = var_mode[dst];
                    let t = claim_tmp!(bits.size(), mode);

                    match (t.ty.array_size(), t.ty.mode) {
                        (None, _) => {
                            writeln!(
                                buffer,
                                "{INDENT}{} = 0x{};",
                                t.ident,
                                bits.display(&BitsFormatOptions {
                                    prefix: false,
                                    base: BitsFormatBase::UpperHex,
                                    separator: None,
                                    align: None,
                                    fill: '0',
                                    width: BitsFormatWidth::Shrink,
                                })
                            )?;
                        }
                        (Some(arr_size), LogicMode::TwoValue) => {
                            for i in 0..arr_size {
                                writeln!(
                                    f,
                                    "{INDENT}{}[{i}] = 0x{};",
                                    t.ident,
                                    bits.slice(i * arr_size, VectorSize::new(64).unwrap())
                                        .display(&BitsFormatOptions {
                                            prefix: false,
                                            base: BitsFormatBase::UpperHex,
                                            separator: None,
                                            align: None,
                                            fill: '0',
                                            width: BitsFormatWidth::Shrink,
                                        })
                                )?;
                            }
                        }
                        (Some(_), LogicMode::FourValue) => {
                            for (i, v) in bits.as_u64_slice().iter().enumerate() {
                                writeln!(f, "{INDENT}{}[{i}] = 0x{v:x};", t.ident)?;
                            }
                        }
                    }
                    store(&mut buffer, heap_map[dst], t)?;
                }
                I::Unary(dst, op, src) => {
                    let src_size = gl.vars[*src].size;
                    let dst_size = gl.vars[*dst].size;
                    let msrc = var_mode[src];
                    let mdst = var_mode[dst];

                    let mut t = claim_tmp!(src_size, msrc);
                    load(&mut buffer, heap_map[src], t)?;
                    if msrc != mdst {
                        let unconverted_t = t;
                        t = claim_tmp!(src_size, mdst);
                        convert(f, src_size, msrc, mdst, unconverted_t.ident, t.ident)?;
                    }

                    let dst_t = claim_tmp!(dst_size, mdst);
                    use UnaryOp as O;
                    match op {
                        O::Neg => unary::cgc_negate(&mut buffer, dst_t, t)?,
                        O::ReduceOr => unary::cgc_reduce_or(&mut buffer, dst_t, t)?,
                        O::ReduceAnd => unary::cgc_reduce_and(&mut buffer, dst_t, t)?,
                        O::ReduceXor => unary::cgc_reduce_xor(&mut buffer, dst_t, t)?,
                        O::ContainsX => todo!(),
                    }
                    store(&mut buffer, heap_map[dst], dst_t)?;
                }
                I::Resize(dst, op, src) => {
                    let src_size = gl.vars[*src].size;
                    let dst_size = gl.vars[*dst].size;
                    let msrc = var_mode[src];
                    let mdst = var_mode[dst];

                    let mut t = claim_tmp!(src_size, msrc);
                    load(&mut buffer, heap_map[src], t)?;
                    if msrc != mdst {
                        let unconverted_t = t;
                        t = claim_tmp!(src_size, mdst);
                        convert(f, src_size, msrc, mdst, unconverted_t.ident, t.ident)?;
                    }

                    let dst_t = claim_tmp!(dst_size, mdst);
                    use ResizeOp as O;
                    match op {
                        O::Truncate => resize::cgc_truncate(&mut buffer, dst_t, t)?,
                        O::ZeroExtend => resize::cgc_zero_extend(&mut buffer, dst_t, t)?,
                        O::SignExtend => resize::cgc_sign_extend(&mut buffer, dst_t, t)?,
                    }
                    store(&mut buffer, heap_map[dst], dst_t)?;
                }
                I::Binary(dst, op, lhs, rhs) => {
                    let lhs_size = gl.vars[*lhs].size;
                    let rhs_size = gl.vars[*rhs].size;
                    let dst_size = gl.vars[*dst].size;
                    let (mlhs, mrhs, mdst) = (var_mode[lhs], var_mode[rhs], var_mode[dst]);

                    let (mut lhs_t, mut rhs_t) =
                        (claim_tmp!(lhs_size, mlhs), claim_tmp!(rhs_size, mrhs));
                    load(&mut buffer, heap_map[lhs], lhs_t)?;
                    if mlhs != mdst {
                        let unconverted_t = lhs_t;
                        lhs_t = claim_tmp!(lhs_size, mdst);
                        convert(f, lhs_size, mlhs, mdst, unconverted_t.ident, lhs_t.ident)?;
                    }
                    load(&mut buffer, heap_map[rhs], rhs_t)?;
                    if mrhs != mdst {
                        let unconverted_t = rhs_t;
                        rhs_t = claim_tmp!(rhs_size, mdst);
                        convert(f, rhs_size, mrhs, mdst, unconverted_t.ident, rhs_t.ident)?;
                    }

                    let dst_t = claim_tmp!(dst_size, mdst);

                    use vogls_ir::BinaryOp as O;
                    match op {
                        O::And => {
                            binary::cgc_bin_and(&mut buffer, dst_t, lhs_t.ident, rhs_t.ident)?
                        }
                        O::Or => binary::cgc_bin_or(&mut buffer, dst_t, lhs_t.ident, rhs_t.ident)?,
                        O::Xor => {
                            binary::cgc_bin_xor(&mut buffer, dst_t, lhs_t.ident, rhs_t.ident)?
                        }
                        O::Add => {
                            binary::cgc_bin_add(&mut buffer, dst_t, lhs_t.ident, rhs_t.ident)?
                        }
                        O::Sub => {
                            binary::cgc_bin_sub(&mut buffer, dst_t, lhs_t.ident, rhs_t.ident)?
                        }
                        O::Power => {
                            binary::cgc_bin_pow(&mut buffer, dst_t, lhs_t.ident, rhs_t.ident)?
                        }
                        O::Multiply => {
                            binary::cgc_bin_mul(&mut buffer, dst_t, lhs_t.ident, rhs_t.ident)?
                        }
                        O::Divide => {
                            binary::cgc_bin_div(&mut buffer, dst_t, lhs_t.ident, rhs_t.ident)?
                        }
                        O::Modulus => {
                            binary::cgc_bin_mod(&mut buffer, dst_t, lhs_t.ident, rhs_t.ident)?
                        }
                        O::UnsignedLessEqual => {
                            binary::cgc_bin_ule(&mut buffer, dst_t.ident, lhs_t, rhs_t.ident)?
                        }

                        O::SelectBit => {
                            binary::cgc_select_bit(&mut buffer, dst_t.ident, lhs_t, rhs_t)?
                        }
                        O::LogicalShiftLeft => binary::cgc_lsl(&mut buffer, dst_t, lhs_t, rhs_t)?,
                        O::LogicalShiftRight => binary::cgc_lsr(&mut buffer, dst_t, lhs_t, rhs_t)?,
                        O::ArithmeticShiftRight => {
                            binary::cgc_asr(&mut buffer, dst_t, lhs_t, rhs_t)?
                        }
                        O::Concat => binary::cgc_concat(&mut buffer, dst_t, lhs_t, rhs_t)?,
                        O::CopyX => binary::cgc_copy_x(&mut buffer, dst_t, lhs_t, rhs_t)?,
                        O::CopyZ => binary::cgc_copy_y(&mut buffer, dst_t, lhs_t, rhs_t)?,
                        O::Min => binary::cgc_bin_min(&mut buffer, dst_t, lhs_t, rhs_t)?,
                        O::Max => binary::cgc_bin_max(&mut buffer, dst_t, lhs_t, rhs_t)?,
                        O::CaseEquality => {
                            binary::cgc_case_eq(&mut buffer, dst_t.ident, lhs_t, rhs_t)?
                        }
                    }
                    store(&mut buffer, heap_map[dst], dst_t)?;
                }
                I::Intrinsic(dst, op, items) => match op.as_ref() {
                    vogls_ir::IntrinsicOp::Time => {
                        let heap_idx = heap_map[dst].bit_offset / 64;
                        writeln!(buffer, "{INDENT}heap[{heap_idx}] = time;")?;
                    }
                    vogls_ir::IntrinsicOp::Finish => {
                        writeln!(buffer, "{INDENT}printf(\"[FINISH]\\n\"); cldctx->exit = 1;")?;
                    }
                    vogls_ir::IntrinsicOp::Random => todo!(),
                    vogls_ir::IntrinsicOp::Display(dyn_format_string) => {
                        // @Performance: scratchpad this.
                        let args = items
                            .iter()
                            .map(|i| {
                                let t = claim_tmp!(gl.vars[*i].size, var_mode[i]);
                                load(&mut buffer, heap_map[i], t)?;
                                Ok(t)
                            })
                            .collect::<io::Result<Vec<CVar>>>()?;
                        lower_dyn_format_str(&mut buffer, &dyn_format_string, args)?;
                    }
                    vogls_ir::IntrinsicOp::Assert(dyn_format_string) => {
                        // @TODO: Format
                        let fst = items[0];
                        let t = claim_tmp!(SCALAR_VSIZE, LogicMode::TwoValue);
                        load(&mut buffer, heap_map[&fst], t)?;
                        writeln!(
                            buffer,
                            r#"{INDENT}if ({t} == 0) {{
"#,
                            t = t.ident
                        )?;
                        // @Performance: scratchpad this.
                        let args = items
                            .iter()
                            .skip(1)
                            .map(|i| {
                                let t = claim_tmp!(gl.vars[*i].size, var_mode[i]);
                                load(&mut buffer, heap_map[i], t)?;
                                Ok(t)
                            })
                            .collect::<io::Result<Vec<CVar>>>()?;
                        lower_dyn_format_str(&mut buffer, &dyn_format_string, args)?;
                        writeln!(
                            buffer,
                            r#"{INDENT}{INDENT}cldctx->exit = 2; return;
{INDENT}}}"#
                        )?;
                    }
                    vogls_ir::IntrinsicOp::VcdOpenFile(_) => todo!(),
                    vogls_ir::IntrinsicOp::VcdAppendModule(vcd_scope) => todo!(),
                    vogls_ir::IntrinsicOp::VcdPause => todo!(),
                    vogls_ir::IntrinsicOp::VcdResume => todo!(),
                },
                I::LastUpdateTime(dst, signal) => {
                    let heap_idx = heap_map[dst].bit_offset / 64;
                    let signal_idx = io_signals.get_index(signal).unwrap();
                    writeln!(
                        buffer,
                        "{INDENT}heap[{heap_idx}] = last_active_time[{signal_idx}];"
                    )?;
                }
                I::Probe(dst, signal) => {
                    let signal = io_signals[signal];
                    let size = gl.vars[*dst].size;
                    assert_eq!(var_mode[dst], gl.logic_mode);

                    let t = claim_tmp!(size, gl.logic_mode);
                    load(&mut buffer, signal.offset, t)?;
                    store(&mut buffer, heap_map[dst], t)?;
                }
                I::Drive(signal, src, partial) => {
                    let size = gl.vars[*src].size;
                    let msrc = var_mode[src];
                    let mut t = claim_tmp!(size, msrc);
                    load(&mut buffer, heap_map[src], t)?;
                    if msrc != gl.logic_mode {
                        let unconverted_t = t;
                        t = claim_tmp!(size, gl.logic_mode);
                        convert(f, size, msrc, gl.logic_mode, unconverted_t.ident, t.ident)?;
                    }

                    if let Some((offset, partial_size)) = partial {
                        // @TODO: offset > size
                        // @TODO: offset contains special
                        let offset_t = claim_tmp!(INTEGER_VSIZE, LogicMode::TwoValue);
                        load(&mut buffer, heap_map[offset], offset_t)?;

                        let current_t = claim_tmp!(*partial_size, gl.logic_mode);
                        load_slice(&mut buffer, current_t, offset_t, io_signals[signal])?;

                        match current_t.ty.array_size() {
                            None => {
                                writeln!(
                                    buffer,
                                    "{INDENT}if ({t} != {current_t}) {{",
                                    t = t.ident,
                                    current_t = current_t.ident
                                )?;
                            }
                            Some(_) => {
                                let condition = claim_tmp!(SCALAR_VSIZE, LogicMode::TwoValue);
                                binary::cgc_case_eq(&mut buffer, condition.ident, t, current_t)?;
                                writeln!(buffer, "{INDENT}if (!{}) {{", condition.ident)?;
                            }
                        }
                        writeln!(
                            buffer,
                            "{INDENT}{INDENT}drive_signal_{idx}(schedule, time, is_scheduled, listening, last_active_time);",
                            idx = io_signals.get_index(signal).unwrap()
                        )?;
                        store_slice(&mut buffer, io_signals[signal], offset_t, t)?;
                        writeln!(buffer, "{INDENT}}}")?;
                    } else {
                        let current_t = claim_tmp!(size, gl.logic_mode);
                        load(&mut buffer, io_signals[signal].offset, current_t)?;
                        match current_t.ty.array_size() {
                            None => {
                                writeln!(
                                    buffer,
                                    "{INDENT}if ({t} != {current_t}) {{",
                                    t = t.ident,
                                    current_t = current_t.ident
                                )?;
                            }
                            Some(_) => {
                                let condition = claim_tmp!(SCALAR_VSIZE, LogicMode::TwoValue);
                                binary::cgc_case_eq(&mut buffer, condition.ident, t, current_t)?;
                                writeln!(buffer, "{INDENT}if (!{}) {{", condition.ident)?;
                            }
                        }
                        writeln!(
                            buffer,
                            "{INDENT}{INDENT}drive_signal_{idx}(schedule, time, is_scheduled, listening, last_active_time);",
                            idx = io_signals.get_index(signal).unwrap()
                        )?;
                        store(&mut buffer, io_signals[signal].offset, t)?;
                        writeln!(buffer, "{INDENT}}}")?;
                    }
                }
                I::Phi(_, _) => continue,
            }
        }

        if let Some(phis) = bb_phis.get(&bb_key) {
            for (dst, src) in phis {
                let src_size = gl.vars[*src].size;
                let dst_size = gl.vars[*dst].size;
                assert_eq!(src_size, dst_size);
                let size = src_size;
                let src_mode = var_mode[src];
                let dst_mode = var_mode[dst];
                let (dst, src) = (heap_map[dst], heap_map[src]);
                use LogicMode as M;
                writeln!(&mut buffer, "{INDENT}// Phi({dst:?}, {src:?});")?;
                match (dst_mode, src_mode) {
                    (M::TwoValue, M::TwoValue) if size.get() > 64 => writeln!(
                        &mut buffer,
                        "{INDENT}memmove(heap+{}, heap+{}, {}*sizeof(uint64_t));",
                        dst.bit_offset / 64,
                        src.bit_offset / 64,
                        size.get().div_ceil(64)
                    )?,
                    (M::TwoValue, M::TwoValue) => {
                        // @Performance. Better lowering.
                        let t = claim_tmp!(size, src_mode);
                        load(&mut buffer, src, t)?;
                        store(&mut buffer, dst, t)?;
                    }
                    _ => todo!(),
                };
            }
        }

        match &bb.terminator {
            BasicBlockTerminator::Wait(bb_key, time) => {
                let time = time.0;
                let state = states_set.get_index(bb_key).unwrap();
                writeln!(
                    buffer,
                    "{INDENT}schedule_future_event(schedule, time + {time}, (event_t){{.ptr=&{procedure}, .state={state}}});"
                )?;
                writeln!(buffer, "{INDENT}return;",)?;
            }
            BasicBlockTerminator::VariableWait(bb_key, time) => {
                let t = claim_tmp!(TIME_VSIZE, LogicMode::TwoValue);
                load(&mut buffer, heap_map[time], t)?;
                let state = states_set.get_index(bb_key).unwrap();
                writeln!(
                    buffer,
                    "{INDENT}schedule_future_event(schedule, time + {t}, (event_t){{.ptr=&{procedure}, .state={state}}});",
                    t = t.ident,
                )?;
                writeln!(buffer, "{INDENT}return;",)?;
            }
            BasicBlockTerminator::WaitRegion(bb_key, region) => {
                let state = states_set.get_index(bb_key).unwrap();
                writeln!(
                    buffer,
                    "{INDENT}event_vec_push(&schedule->regions[{region}], (event_t){{.ptr=&{procedure}, .state={state}}});",
                )?;
                writeln!(buffer, "{INDENT}return;",)?;
            }
            BasicBlockTerminator::Watch(bb_key, items) => {
                let state = states_set.get_index(bb_key).unwrap();
                for item in items {
                    let offset = listener_builder.top;
                    writeln!(
                        buffer,
                        "{INDENT}listening[{}] |= 0x{:x};",
                        offset / 64,
                        1u64 << (offset % 64)
                    )?;
                    listener_builder.insert(*item, process_idx, process_key, state as u32);
                }
                writeln!(
                    buffer,
                    "{INDENT}is_scheduled[{}] &= 0x{:x};",
                    process_idx / 64,
                    !(1u64 << (process_idx % 64)),
                )?;
                writeln!(buffer, "{INDENT}return;",)?;
            }
            BasicBlockTerminator::Jump(bb_key) => {
                writeln!(
                    buffer,
                    "{INDENT}goto L{};",
                    bb_ident.get_index(bb_key).unwrap()
                )?;
            }
            BasicBlockTerminator::Branch(condition, truthy, falsy) => {
                let truthy = bb_ident.get_index(truthy).unwrap();
                let falsy = bb_ident.get_index(falsy).unwrap();

                let size = gl.vars[*condition].size;
                let mcondition = var_mode[condition];
                let t = claim_tmp!(size, mcondition);
                load(&mut buffer, heap_map[condition], t)?;

                writeln!(
                    buffer,
                    "{INDENT}if ({t} != 0) {{ goto L{truthy}; }} else {{ goto L{falsy}; }}",
                    t = t.ident
                )?;
            }
            BasicBlockTerminator::Halt => {
                writeln!(buffer, "{INDENT}return;")?;
            }
        }
    }

    f.write_all(&buffer)?;
    writeln!(f)?;
    writeln!(f, "}}")?;

    Ok(())
}

fn lower_dyn_format_str(
    f: &mut impl io::Write,
    dyn_format_string: &DynFormatString,
    args: Vec<CVar>,
) -> io::Result<()> {
    write!(f, r#"{INDENT}{{ "#)?;
    for (i, arg) in args.iter().enumerate() {
        if !arg.ty.is_array() {
            write!(f, "uint64_t arg{i} = (uint64_t){}; ", arg.ident)?;
        }
    }
    write!(
        f,
        r#"bits_ref_t args[{num_args}] = {{ "#,
        num_args = args.len()
    )?;
    for (i, arg) in args.iter().enumerate() {
        if arg.ty.is_array() {
            write!(
                f,
                r#"(bits_ref_t){{ .size={}, .ptr={} }}, "#,
                arg.ty.size, arg.ident
            )?;
        } else {
            write!(
                f,
                r#"(bits_ref_t){{ .size={}, .ptr=&arg{i} }}, "#,
                arg.ty.size
            )?;
        }
    }
    write!(
        f,
        r#"}}; (cldctx->fmt)(cldctx->stdout, (void*){dyn_fmt_ptr:p}, args); }}"#,
        dyn_fmt_ptr = dyn_format_string as *const DynFormatString,
    )?;
    Ok(())
}

const fn mask(num_bits: u32) -> u64 {
    if num_bits == 64 {
        u64::MAX
    } else {
        (1u64 << num_bits) - 1
    }
}

fn load(b: &mut impl io::Write, heap_offset: HeapOffset, t: CVar) -> io::Result<()> {
    if t.ty.mode == LogicMode::FourValue {
        todo!()
    }
    if let Some(arr_size) = t.ty.array_size() {
        let offset = heap_offset;
        let word = offset.bit_offset / 64;
        writeln!(
            b,
            "{INDENT}memmove(&{t}, heap + {word}, {arr_size} * sizeof(uint64_t));",
            t = t.ident
        )?;
        return Ok(());
    }

    let mut num_bits = t.ty.size.get();
    if t.ty.mode == LogicMode::FourValue {
        num_bits *= 2;
    }

    let offset = heap_offset;
    let word = offset.bit_offset / 64;
    let shift = offset.bit_offset % 64;
    let mask = mask(num_bits);

    writeln!(
        b,
        "{INDENT}{t} = (heap[{word}] >> {shift}) & 0x{mask:x};",
        t = t.ident
    )
}

fn store(f: &mut impl io::Write, heap_offset: HeapOffset, t: CVar) -> io::Result<()> {
    if t.ty.mode == LogicMode::FourValue {
        todo!()
    }
    if let Some(arr_size) = t.ty.array_size() {
        let offset = heap_offset;
        let word = offset.bit_offset / 64;
        writeln!(
            f,
            "{INDENT}memmove(heap + {word}, &{t}, {arr_size} * sizeof(uint64_t));",
            t = t.ident
        )?;
        return Ok(());
    }

    let mut num_bits = t.ty.size.get();
    if t.ty.mode == LogicMode::FourValue {
        num_bits *= 2;
    }

    let offset = heap_offset;
    let word = offset.bit_offset / 64;
    let shift = offset.bit_offset % 64;
    let mask = if num_bits == 64 {
        assert_eq!(shift, 0);
        u64::MAX
    } else {
        !(((1u64 << num_bits) - 1) << shift)
    };

    writeln!(
        f,
        "{INDENT}heap[{word}] = (heap[{word}] & 0x{mask:x}) | ((uint64_t){t} << {shift});",
        t = t.ident
    )
}

fn load_slice(b: &mut impl io::Write, dst: CVar, offset: CVar, src: HeapRef) -> io::Result<()> {
    if dst.ty.mode == LogicMode::FourValue {
        todo!()
    }
    let src_ty = CType {
        size: src.size,
        mode: dst.ty.mode,
    };
    if let Some(_) = src_ty.array_size() {
        match dst.ty.array_size() {
            None => {
                write!(
                    b,
                    r#"
{INDENT}if (({offset}%64)+{dst_size} <= 64) {dst} = (heap[{word}+({offset}/64)] >> ({offset}%64)) & 0x{mask:x};
{INDENT}else {dst} = (heap[{word}+({offset}/64)] >> ({offset}%64)) | (heap[{word}+({offset}/64) + 1] >> (64 - {offset}%64)) & 0x{mask:x};
"#,
                    offset = offset.ident,
                    dst = dst.ident,
                    dst_size = dst.ty.size,
                    word = src.offset.bit_offset / 64,
                    mask = mask(dst.ty.size.get())
                )?;
            }
            _ => todo!(),
        }

        return Ok(());
    }

    let mut num_bits = dst.ty.size.get();
    if dst.ty.mode == LogicMode::FourValue {
        num_bits *= 2;
    }

    let word = src.offset.bit_offset / 64;
    let shift = src.offset.bit_offset % 64;
    let mask = mask(num_bits);

    writeln!(
        b,
        "{INDENT}{t} = (heap[{word}] >> ({shift} + {offset})) & 0x{mask:x};",
        t = dst.ident,
        offset = offset.ident,
    )
}

fn store_slice(f: &mut impl io::Write, dst: HeapRef, offset: CVar, src: CVar) -> io::Result<()> {
    if src.ty.mode == LogicMode::FourValue {
        todo!()
    }
    let dst_ty = CType {
        size: dst.size,
        mode: src.ty.mode,
    };
    if let Some(_) = dst_ty.array_size() {
        match src.ty.array_size() {
            None => {
                write!(
                    f,
                    r#"
{INDENT}if (({offset}%64)+{src_size} <= 64) heap[{word}+({offset}/64)] = (heap[{word}+({offset}/64)] & ~(((uint64_t)0x{mask:x}) << ({offset}%64))) | (((uint64_t){src}) << ({offset}%64));
{INDENT}else {{ printf("NYI [STORE SLICE]\n"); cldctx->exit = 2; return; }};
"#,
                    src_size = src.ty.size,
                    offset = offset.ident,
                    src = src.ident,
                    word = dst.offset.bit_offset / 64,
                    mask = mask(src.ty.size.get()),
                )?;
            }
            _ => todo!(),
        }
        return Ok(());
    }

    let mut num_bits = src.ty.size.get();
    if src.ty.mode == LogicMode::FourValue {
        num_bits *= 2;
    }

    let word = dst.offset.bit_offset / 64;
    let shift = dst.offset.bit_offset % 64;
    let mask = mask(num_bits);

    writeln!(
        f,
        "{INDENT}heap[{word}] = (heap[{word}] & ~(((uint64_t)0x{mask:x}) << ({shift}+{offset}))) | ((uint64_t){t} << ({shift} + {offset}));",
        t = src.ident,
        offset = offset.ident,
    )
}

fn convert(
    f: &mut impl io::Write,
    size: std::num::NonZero<u32>,
    msrc: LogicMode,
    mdst: LogicMode,
    unconverted_t: CIdent,
    t: CIdent,
) -> io::Result<()> {
    todo!()
}

pub fn lower_signal_drive_header(
    f: &mut impl io::Write,
    signal: SignalKey,
    io_signals: &IndexMap<SignalKey, HeapRef>,
) -> io::Result<()> {
    let idx = io_signals.get_index(&signal).unwrap();
    writeln!(
        f,
        "void drive_signal_{idx}(schedule_t *schedule, uint64_t time, uint64_t *is_scheduled, uint64_t *listening, uint64_t *last_active_time);"
    )
}

pub fn lower_signal_drive_fn(
    f: &mut impl io::Write,
    gl: &GlobalContext,
    signal: SignalKey,
    listener_builder: &ListenerBuilder,
    io_signals: &IndexMap<SignalKey, HeapRef>,
) -> io::Result<()> {
    let idx = io_signals.get_index(&signal).unwrap();
    writeln!(
        f,
        "void drive_signal_{idx}(schedule_t *schedule, uint64_t time, uint64_t *is_scheduled, uint64_t *listening, uint64_t *last_active_time) {{",
    )?;

    writeln!(f, "{INDENT}last_active_time[{idx}] = time;")?;
    if let Some(listeners) = listener_builder.map.get(&signal) {
        for listener in listeners {
            writeln!(
                f,
                "{INDENT}if (((listening[{}] >> {}) & 1) != 0 && ((is_scheduled[{}] >> {}) & 1) == 0) {{",
                listener.offset / 64,
                listener.offset % 64,
                listener.process_idx / 64,
                listener.process_idx % 64,
            )?;
            writeln!(
                f,
                "{INDENT}{INDENT}is_scheduled[{}] |= 0x{:x};",
                listener.process_idx / 64,
                1u64 << (listener.process_idx % 64),
            )?;
            writeln!(
                f,
                "{INDENT}{INDENT}event_vec_push(&schedule->active_region, (event_t){{.ptr=&{}, .state={}}});",
                process_to_procedure_name(
                    &gl.processes[listener.process_key],
                    listener.process_idx
                ),
                listener.state
            )?;
            writeln!(f, "{INDENT}}}",)?;
        }
    }

    writeln!(f, "}}")?;
    Ok(())
}

pub fn lower_startup_function(f: &mut impl io::Write, gl: &GlobalContext) -> io::Result<()> {
    f.write_all(
        b"void startup(
    uint64_t *heap,
    schedule_t *schedule,
    uint64_t time,
    uint64_t *is_scheduled,
    uint64_t *listening,
    uint64_t *last_active_time,
    cold_context_t *cldctx
) {\n",
    )?;
    writeln!(f, "{INDENT}(void)heap;")?;
    writeln!(f, "{INDENT}(void)schedule;")?;
    writeln!(f, "{INDENT}(void)time;")?;
    writeln!(f, "{INDENT}(void)is_scheduled;")?;
    writeln!(f, "{INDENT}(void)listening;")?;
    writeln!(f, "{INDENT}(void)last_active_time;")?;
    writeln!(f)?;
    for (i, process) in gl.processes.values().enumerate() {
        writeln!(
            f,
            "{INDENT}{}(0, heap, schedule, time, is_scheduled, listening, last_active_time, cldctx); if (cldctx->exit != 0) return;",
            // int state, uint64_t *heap, schedule_t *schedule, uint64_t *time, uint64_t *is_scheduled, uint64_t *listening, uint64_t *last_active_time
            process_to_procedure_name(process, i)
        )?;
    }
    writeln!(f, "}}")?;
    Ok(())
}

pub fn prologue(f: &mut impl io::Write) -> io::Result<()> {
    f.write_all(
        br#"#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct bits_ref {
    uint32_t size;
    uint64_t* ptr;
} bits_ref_t;

struct schedule;
typedef struct cold_context {
    uint8_t exit;
    void (*fmt)(void*, void*, bits_ref_t*);
    void *stdout;
    void *stderr;
} cold_context_t;

typedef struct event {
  void (*ptr)(int, uint64_t*, struct schedule*, uint64_t, uint64_t*, uint64_t*, uint64_t*, cold_context_t*);
  int state;
} event_t;
typedef struct timed_event {
  event_t event;
  uint64_t time;
} timed_event_t;
typedef struct event_vec {
  event_t *ptr;
  size_t length;
  size_t capacity;
  void (*grow)(void*);
} event_vec_t;
typedef struct timed_event_vec {
  timed_event_t *ptr;
  size_t length;
  size_t capacity;
  void (*grow)(void*);
} timed_event_vec_t;

typedef struct schedule {
  event_vec_t active_region;
  event_vec_t *regions;
  timed_event_vec_t future;
  uint64_t next_time;
} schedule_t;

static inline void schedule_future_event(schedule_t *schedule, uint64_t time, event_t event);
static inline void event_vec_push(event_vec_t *v, event_t event);
static inline bool event_vec_pop(event_vec_t *v, event_t *event);

static inline uint32_t popcount64(uint64_t n) {
#if defined(__GNUC__) || defined(__clang__)
    return __builtin_popcountll(n);
#else
#error "No popcount built-in"
#endif
}
static inline uint32_t popcount32(uint32_t n) {
#if defined(__GNUC__) || defined(__clang__)
    return __builtin_popcount(n);
#elif defined(_MSC_VER)
    #include <intrin.h>
    return __popcnt(n);
#else
#error "No popcount built-in"
#endif
}
static inline uint32_t popcount16(uint16_t n) {
	return popcount32((uint32_t)n);
}
static inline uint32_t popcount8(uint8_t n) {
	return popcount32((uint32_t)n);
}
"#,
    )
}

pub fn epilogue(f: &mut impl io::Write) -> io::Result<()> {
    f.write_all(
        br#"

static inline void schedule_future_event(schedule_t *schedule, uint64_t time, event_t event) {
  schedule->next_time = (schedule->future.length == 0 || time < schedule->next_time) ? time : schedule->next_time;
  if (schedule->future.length >= schedule->future.capacity) {
    (schedule->future.grow)((void*)&schedule->future);
  }

  timed_event_t te = {
      .event = event,
      .time = time,
  };
  schedule->future.ptr[schedule->future.length] = te;
  schedule->future.length += 1;
}

static inline void event_vec_push(event_vec_t *v, event_t event) {
  if (v->length >= v->capacity) {
    (v->grow)((void*)v);
  }
  v->ptr[v->length] = event;
  v->length += 1;
}
static inline bool event_vec_pop(event_vec_t *v, event_t *event) {
  if (v->length == 0) {
    event->ptr = NULL;
    return false;
  }
  *event = v->ptr[v->length - 1];
  v->length -= 1;
  return true;
}
"#)
}

pub fn add_main(
    f: &mut impl io::Write,
    gl: &GlobalContext,
    heap_builder: &HeapBuilder,
    listener_builder: &ListenerBuilder,
) -> io::Result<()> {
    f.write_all(b"int main() {")?;
    write!(
        f,
        r#"
  uint64_t time = 0;
  schedule_t schedule = {{
      .active_region = {{}},
      .regions = NULL,
      .future = {{}},
  }};
  uint64_t heap[{heap_size}] = {{}};
  uint64_t is_scheduled[{is_scheduled_size}] = {{}};
  uint64_t listening[{listening_size}] = {{}};
  uint64_t last_active_time[{last_active_time_size}] = {{}};
"#,
        heap_size = heap_builder.top().div_ceil(64),
        is_scheduled_size = gl.processes.len().div_ceil(64),
        listening_size = listener_builder.top.div_ceil(64),
        last_active_time_size = gl.signals.len(),
    )?;
    f.write_all(
        b"
  size_t j;
  startup(heap, &schedule, time, is_scheduled, listening, last_active_time);

  while (schedule.active_region.length > 0) {{
    event_t e;
    while (event_vec_pop(&schedule.active_region, &e)) {{
      (e.ptr)(e.state, heap, &schedule, time, is_scheduled, listening, last_active_time);
    }}

    // @TODO: Regions

    uint64_t next_time;
    timed_event_t te;
    j = 0;
    for (size_t i = 0; i < schedule.future.length; ++i) {
      te = (timed_event_t)schedule.future.ptr[i];
      if (te.time == schedule.next_time) {
        event_vec_push(&schedule.active_region, te.event);
      } else {
        next_time = (te.time < next_time) ? time : next_time;
        schedule.future.ptr[j] = te;
        j += 1;
      }
    }}
    schedule.future.length = j;
  }
}",
    )
}

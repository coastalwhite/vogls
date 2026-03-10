use std::fmt::Write;
use std::{fmt, io};

use vogls_bits::BitsDataRef;
use vogls_bits::format::{BitsFormatBase, BitsFormatOptions, BitsFormatWidth};
use vogls_codegen::{
    HeapBuilder, HeapOffset, HeapRef, resolve_heap_map, resolve_var_logic_mode_map,
};
use vogls_ir::dyn_format_string::DynFormatString;
use vogls_ir::{
    BasicBlockKey, BasicBlockTerminator, BinaryOp, GlobalContext, Instruction, LogicMode, Process,
    ProcessKey, ResizeOp, SCALAR_VSIZE, SignalKey, UnaryOp, VariableKey, VectorSize,
};
use vogls_ir_properties::get_temporal_variables;
use vogls_runtime::RtSignalKey;
use vogls_utils::{IndexSet, VgHashMap, VgHashSet};

pub mod runtime;

mod binary;
mod resize;
mod unary;

// Variables are represented as u8, u16, u32, u64 or an array of u64s depending or their size and
// their logic mode.
//
// |  size | mode |                         type | bit layout              |
// |-------|------|------------------------------|-------------------------|
// |   1-8 |   TV |                           u8 | `size` least sign. bits |
// |  9-16 |   TV |                          u16 | `size` least sign. bits |
// | 17-32 |   TV |                          u32 | `size` least sign. bits |
// | 33-64 |   TV |                          u64 | `size` least sign. bits |
// |   65+ |   TV |     [u64; size.div_ceil(64)] | little endian word order. Last word same as u64 |
// |       |      |                              |                         |
// |   1-4 |   FV |                           u8 |                         |
// |   5-8 |   FV |                          u16 |                         |
// |  9-16 |   FV |                          u32 |                         |
// | 17-32 |   FV |                          u64 |                         |
// |   33+ |   FV | [u64; 2 * size.div_ceil(64)] |                         |
//
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
                (self.size.get() > 32).then_some(2 * self.size.get().div_ceil(64))
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

    fn num_bits(&self) -> u32 {
        self.size.get() << u32::from(self.mode == LogicMode::FourValue)
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

fn write_cvar(f: &mut impl io::Write, var: CVar) -> io::Result<()> {
    write!(f, "{INDENT}{} {}", var.ty.element_type(), var.ident)?;
    if let Some(array_size) = var.ty.array_size() {
        write!(f, "[{array_size}]")?;
    }
    writeln!(f, ";")
}

pub fn lower_process(
    f: &mut impl io::Write,
    process_key: ProcessKey,
    process_idx: usize,
    gl: &GlobalContext,
    heap_builder: &mut HeapBuilder,
    listener_builder: &mut ListenerBuilder,
    io_signals: &VgHashMap<SignalKey, RtSignalKey>,
    signals: &[HeapRef],
) -> io::Result<()> {
    use Instruction as I;

    let process = &gl.processes[process_key];

    let mut bb_stack = Vec::new();
    let mut bb_seen = VgHashSet::<BasicBlockKey>::default();
    let mut bb_phis = VgHashMap::<BasicBlockKey, Vec<(VariableKey, VariableKey)>>::default();

    let mut var_mode = VgHashMap::<VariableKey, LogicMode>::default();
    let mut conv_map = VgHashMap::<VariableKey, HeapOffset>::default();
    let mut heap_map = VgHashMap::default();
    let mut temporal_variables = Default::default();
    {
        let mut temporal_roots = Default::default();
        let mut temporal = Default::default();
        let mut variable_to_tmr_map = Default::default();
        get_temporal_variables(
            process.entry,
            &gl.bbs,
            &mut bb_stack,
            &mut bb_seen,
            &mut temporal_roots,
            &mut temporal,
            &mut variable_to_tmr_map,
            &mut temporal_variables,
        );
    }
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
        Some(&temporal_variables),
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
    let mut temp_map = VgHashMap::<(VariableKey, LogicMode), CVar>::default();

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

        bb.try_for_each_dst_var(|v| {
            let mode = var_mode[&v];
            let t = CVar {
                ident: CIdent(temp_map.len() as u64),
                ty: CType {
                    size: gl.vars[v].size,
                    mode,
                },
            };

            write_cvar(f, t)?;
            temp_map.insert((v, t.ty.mode), t);

            if conv_map.contains_key(&v) {
                let mut t = t;
                t.ident = CIdent(temp_map.len() as u64);
                t.ty.mode = t.ty.mode.other();
                write_cvar(f, t)?;
                temp_map.insert((v, t.ty.mode), t);
            }
            io::Result::Ok(())
        })?;

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

    let mut temp_counter = temp_map.len() as u64;

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
                    let t = temp_map[&(*dst, mode)];

                    match (t.ty.array_size(), t.ty.mode) {
                        (None, LogicMode::TwoValue) => {
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
                        (None, LogicMode::FourValue) => {
                            let BitsDataRef::InlineFv(spc, val) = bits.as_data_ref() else {
                                unreachable!();
                            };

                            writeln!(
                                buffer,
                                "{INDENT}{} = 0x{:x};",
                                t.ident,
                                (val << bits.size().get()) | spc
                            )?;
                        }
                        (Some(arr_size), LogicMode::TwoValue) => {
                            for i in 0..arr_size {
                                writeln!(
                                    buffer,
                                    "{INDENT}{}[{i}] = 0x{};",
                                    t.ident,
                                    bits.slice(i * 64, VectorSize::new(64).unwrap()).display(
                                        &BitsFormatOptions {
                                            prefix: false,
                                            base: BitsFormatBase::UpperHex,
                                            separator: None,
                                            align: None,
                                            fill: '0',
                                            width: BitsFormatWidth::Shrink,
                                        }
                                    )
                                )?;
                            }
                        }
                        (Some(_), LogicMode::FourValue) => {
                            for (i, v) in bits.as_u64_slice().iter().enumerate() {
                                writeln!(buffer, "{INDENT}{}[{i}] = 0x{v:x};", t.ident)?;
                            }
                        }
                    }
                    if temporal_variables.contains(dst) {
                        store(&mut buffer, heap_map[dst], t)?;
                    }
                }
                I::Unary(dst, op, src) => {
                    let src_size = gl.vars[*src].size;
                    let msrc = var_mode[src];
                    let mdst = var_mode[dst];

                    let mut t = temp_map[&(*src, msrc)];
                    if temporal_variables.contains(src) {
                        load(&mut buffer, heap_map[src], t)?;
                    }
                    if msrc != mdst {
                        let unconverted_t = t;
                        t = temp_map[&(*src, mdst)];
                        convert(
                            &mut buffer,
                            src_size,
                            mdst,
                            msrc,
                            t.ident,
                            unconverted_t.ident,
                        )?;
                    }

                    let dst_t = temp_map[&(*dst, mdst)];
                    use UnaryOp as O;
                    match op {
                        O::Neg => unary::cgc_negate(&mut buffer, dst_t, t)?,
                        O::ReduceOr => unary::cgc_reduce_or(&mut buffer, dst_t, t)?,
                        O::ReduceAnd => unary::cgc_reduce_and(&mut buffer, dst_t, t)?,
                        O::ReduceXor => unary::cgc_reduce_xor(&mut buffer, dst_t, t)?,
                    }
                    if temporal_variables.contains(dst) {
                        store(&mut buffer, heap_map[dst], dst_t)?;
                    }
                }
                I::Resize(dst, op, src) => {
                    let src_size = gl.vars[*src].size;
                    let msrc = var_mode[src];
                    let mdst = var_mode[dst];

                    let mut t = temp_map[&(*src, msrc)];
                    if temporal_variables.contains(src) {
                        load(&mut buffer, heap_map[src], t)?;
                    }
                    if msrc != mdst {
                        let unconverted_t = t;
                        t = temp_map[&(*src, mdst)];
                        convert(
                            &mut buffer,
                            src_size,
                            mdst,
                            msrc,
                            t.ident,
                            unconverted_t.ident,
                        )?;
                    }

                    let dst_t = temp_map[&(*dst, mdst)];
                    use ResizeOp as O;
                    match op {
                        O::Truncate => resize::cgc_truncate(&mut buffer, dst_t, t)?,
                        O::ZeroExtend => resize::cgc_zero_extend(&mut buffer, dst_t, t)?,
                        O::SignExtend => resize::cgc_sign_extend(&mut buffer, dst_t, t)?,
                    }
                    if temporal_variables.contains(dst) {
                        store(&mut buffer, heap_map[dst], dst_t)?;
                    }
                }
                I::Binary(dst, op, lhs, rhs) => {
                    let lhs_size = gl.vars[*lhs].size;
                    let rhs_size = gl.vars[*rhs].size;
                    let (mlhs, mrhs, mdst) = (var_mode[lhs], var_mode[rhs], var_mode[dst]);

                    let (mut lhs_t, mut rhs_t) = (temp_map[&(*lhs, mlhs)], temp_map[&(*rhs, mrhs)]);
                    if temporal_variables.contains(lhs) {
                        load(&mut buffer, heap_map[lhs], lhs_t)?;
                    }
                    let (mtgt, conv_lhs, conv_rhs) =
                        bin_args_need_conversion(*op, mdst, mlhs, mrhs);
                    if conv_lhs {
                        let unconverted_t = lhs_t;
                        lhs_t = temp_map[&(*lhs, mtgt)];
                        convert(
                            &mut buffer,
                            lhs_size,
                            mtgt,
                            mlhs,
                            lhs_t.ident,
                            unconverted_t.ident,
                        )?;
                    }
                    if temporal_variables.contains(rhs) {
                        load(&mut buffer, heap_map[rhs], rhs_t)?;
                    }
                    if conv_rhs {
                        let unconverted_t = rhs_t;
                        rhs_t = temp_map[&(*rhs, mtgt)];
                        convert(
                            &mut buffer,
                            rhs_size,
                            mtgt,
                            mrhs,
                            rhs_t.ident,
                            unconverted_t.ident,
                        )?;
                    }

                    let dst_t = temp_map[&(*dst, mdst)];

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
                        O::Posedge => binary::cgc_posedge(&mut buffer, dst_t.ident, lhs_t, rhs_t)?,
                        O::Negedge => binary::cgc_negedge(&mut buffer, dst_t.ident, lhs_t, rhs_t)?,
                    }
                    if temporal_variables.contains(dst) {
                        store(&mut buffer, heap_map[dst], dst_t)?;
                    }
                }
                I::Intrinsic(dst, op, items) => match op.as_ref() {
                    vogls_ir::IntrinsicOp::Time => {
                        let heap_idx = heap_map[dst].bit_offset / 64;
                        writeln!(buffer, "{INDENT}heap[{heap_idx}] = time;")?;
                    }
                    vogls_ir::IntrinsicOp::Finish => {
                        writeln!(buffer, "{INDENT}cldctx->exit = 1; return;")?;
                    }
                    vogls_ir::IntrinsicOp::Random => todo!(),
                    vogls_ir::IntrinsicOp::Display(dyn_format_string) => {
                        // @Performance: scratchpad this.
                        let args = items
                            .iter()
                            .map(|i| {
                                let t = temp_map[&(*i, var_mode[i])];
                                if temporal_variables.contains(i) {
                                    load(&mut buffer, heap_map[i], t)?;
                                }
                                Ok(t)
                            })
                            .collect::<io::Result<Vec<CVar>>>()?;
                        lower_dyn_format_str(&mut buffer, &dyn_format_string, args)?;
                    }
                    vogls_ir::IntrinsicOp::Assert(dyn_format_string) => {
                        // @TODO: Format
                        let fst = items[0];
                        let t = temp_map[&(fst, LogicMode::TwoValue)];
                        if temporal_variables.contains(&fst) {
                            load(&mut buffer, heap_map[&fst], t)?;
                        }
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
                                let t = temp_map[&(*i, var_mode[i])];
                                if temporal_variables.contains(i) {
                                    load(&mut buffer, heap_map[i], t)?;
                                }
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
                    let t = temp_map[&(*dst, LogicMode::TwoValue)];
                    let signal_idx = io_signals[signal].as_u64();
                    writeln!(
                        buffer,
                        "{INDENT}{} = last_active_time[{signal_idx}];",
                        t.ident
                    )?;
                    if temporal_variables.contains(dst) {
                        store(&mut buffer, heap_map[dst], t)?;
                    }
                }
                I::Probe(dst, signal) => {
                    let signal = signals[io_signals[signal].as_usize()];
                    assert_eq!(var_mode[dst], gl.logic_mode);

                    let t = temp_map[&(*dst, gl.logic_mode)];
                    load(&mut buffer, signal.offset, t)?;
                    if temporal_variables.contains(dst) {
                        store(&mut buffer, heap_map[dst], t)?;
                    }
                }
                I::Drive(signal, src, partial) => {
                    let size = gl.vars[*src].size;
                    let msrc = var_mode[src];
                    let mut t = temp_map[&(*src, msrc)];
                    if temporal_variables.contains(src) {
                        load(&mut buffer, heap_map[src], t)?;
                    }
                    if msrc != gl.logic_mode {
                        let unconverted_t = t;
                        t = temp_map[&(*src, gl.logic_mode)];
                        convert(
                            &mut buffer,
                            size,
                            gl.logic_mode,
                            msrc,
                            t.ident,
                            unconverted_t.ident,
                        )?;
                    }

                    writeln!(buffer, "{INDENT}{{")?;

                    if let Some((offset, partial_size)) = partial {
                        // @TODO: offset > size
                        // @TODO: offset contains special
                        let offset_t = temp_map[&(*offset, LogicMode::TwoValue)];
                        if temporal_variables.contains(offset) {
                            load(&mut buffer, heap_map[offset], offset_t)?;
                        }

                        let current_t = CVar {
                            ident: CIdent(temp_counter),
                            ty: CType {
                                size: *partial_size,
                                mode: gl.logic_mode,
                            },
                        };
                        temp_counter += 1;
                        write_cvar(&mut buffer, current_t)?;
                        load_slice(
                            &mut buffer,
                            current_t,
                            offset_t,
                            signals[io_signals[signal].as_usize()],
                        )?;

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
                                let condition = CVar {
                                    ident: CIdent(temp_counter),
                                    ty: CType {
                                        size: SCALAR_VSIZE,
                                        mode: LogicMode::TwoValue,
                                    },
                                };
                                temp_counter += 1;
                                write_cvar(&mut buffer, condition)?;
                                binary::cgc_case_eq(&mut buffer, condition.ident, t, current_t)?;
                                writeln!(buffer, "{INDENT}if (!{}) {{", condition.ident)?;
                            }
                        }
                        writeln!(
                            buffer,
                            "{INDENT}{INDENT}drive_signal_{idx}(schedule, time, is_scheduled, listening, last_active_time);",
                            idx = io_signals[signal].as_u64()
                        )?;
                        store_slice(
                            &mut buffer,
                            signals[io_signals[signal].as_usize()],
                            offset_t,
                            t,
                        )?;
                        writeln!(buffer, "{INDENT}}}")?;
                    } else {
                        let current_t = CVar {
                            ident: CIdent(temp_counter),
                            ty: CType {
                                size,
                                mode: gl.logic_mode,
                            },
                        };
                        temp_counter += 1;
                        write_cvar(&mut buffer, current_t)?;
                        load(
                            &mut buffer,
                            signals[io_signals[signal].as_usize()].offset,
                            current_t,
                        )?;
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
                                let condition = CVar {
                                    ident: CIdent(temp_counter),
                                    ty: CType {
                                        size: SCALAR_VSIZE,
                                        mode: LogicMode::TwoValue,
                                    },
                                };
                                temp_counter += 1;
                                write_cvar(&mut buffer, condition)?;
                                binary::cgc_case_eq(&mut buffer, condition.ident, t, current_t)?;
                                writeln!(buffer, "{INDENT}if (!{}) {{", condition.ident)?;
                            }
                        }
                        writeln!(
                            buffer,
                            "{INDENT}{INDENT}drive_signal_{idx}(schedule, time, is_scheduled, listening, last_active_time);",
                            idx = io_signals[signal].as_u64()
                        )?;
                        store(
                            &mut buffer,
                            signals[io_signals[signal].as_usize()].offset,
                            t,
                        )?;
                        writeln!(buffer, "{INDENT}}}")?;
                    }

                    writeln!(buffer, "{INDENT}}}")?;
                }
                I::Phi(_, _) => continue,
            }
        }

        if let Some(phis) = bb_phis.get(&bb_key) {
            for (dst, src) in phis {
                let src_size = gl.vars[*src].size;
                let dst_size = gl.vars[*dst].size;
                assert_eq!(src_size, dst_size);
                let src_mode = var_mode[src];
                let dst_mode = var_mode[dst];

                let mut src_t = temp_map[&(*src, src_mode)];
                let dst_t = temp_map[&(*dst, dst_mode)];

                if temporal_variables.contains(src) {
                    load(&mut buffer, heap_map[src], src_t)?;
                }

                if src_mode != dst_mode {
                    let unconverted_t = src_t;
                    src_t = temp_map[&(*src, dst_mode)];
                    convert(
                        &mut buffer,
                        src_size,
                        dst_mode,
                        src_mode,
                        src_t.ident,
                        unconverted_t.ident,
                    )?;
                }

                writeln!(&mut buffer, "{INDENT}// Phi({dst:?}, {src:?});")?;
                let d = dst_t.ident;
                let s = src_t.ident;
                match dst_t.ty.array_size() {
                    None => writeln!(f, "{INDENT}{d} = {s};")?,
                    Some(num_words) => writeln!(
                        f,
                        "{INDENT}memmove({d}, {s}, {num_words}*sizeof(uint64_t));"
                    )?,
                }
                if temporal_variables.contains(dst) {
                    store(&mut buffer, heap_map[dst], dst_t)?;
                }
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
                let t = temp_map[&(*time, LogicMode::TwoValue)];
                if temporal_variables.contains(time) {
                    load(&mut buffer, heap_map[time], t)?;
                }
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

                let mcondition = var_mode[condition];
                let t = temp_map[&(*condition, mcondition)];
                if temporal_variables.contains(condition) {
                    load(&mut buffer, heap_map[condition], t)?;
                }

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

fn bin_args_need_conversion(
    op: BinaryOp,
    mdst: LogicMode,
    mlhs: LogicMode,
    mrhs: LogicMode,
) -> (LogicMode, bool, bool) {
    use LogicMode as M;
    if op.always_outputs_bool() {
        (
            M::FourValue,
            mlhs == M::TwoValue && mrhs == M::FourValue,
            mlhs == M::FourValue && mrhs == M::TwoValue,
        )
    } else {
        (mdst, mdst != mlhs, mdst != mrhs)
    }
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
                r#"(bits_ref_t){{ .size={}, .mode={}, .ptr={} }}, "#,
                arg.ty.size,
                u8::from(arg.ty.mode == LogicMode::FourValue),
                arg.ident
            )?;
        } else {
            write!(
                f,
                r#"(bits_ref_t){{ .size={}, .mode={}, .ptr=&arg{i} }}, "#,
                arg.ty.size,
                u8::from(arg.ty.mode == LogicMode::FourValue)
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
    match t.ty.array_size() {
        None => {
            let word = heap_offset.bit_offset / 64;
            let shift = heap_offset.bit_offset % 64;
            let num_bits = t.ty.num_bits();
            let mask = mask(num_bits);

            writeln!(
                b,
                "{INDENT}{t} = (heap[{word}] >> {shift}) & 0x{mask:x};",
                t = t.ident
            )
        }
        Some(arr_size) => {
            let word = heap_offset.bit_offset / 64;
            writeln!(
                b,
                "{INDENT}memmove(&{t}, heap + {word}, {arr_size} * sizeof(uint64_t));",
                t = t.ident
            )
        }
    }
}

fn store(f: &mut impl io::Write, heap_offset: HeapOffset, t: CVar) -> io::Result<()> {
    match t.ty.array_size() {
        None => {
            let word = heap_offset.bit_offset / 64;
            let shift = heap_offset.bit_offset % 64;
            let num_bits = t.ty.num_bits();
            if num_bits == 64 {
                writeln!(f, "{INDENT}heap[{word}] = {t};", t = t.ident)
            } else {
                let mask = !(((1u64 << num_bits) - 1) << shift);
                writeln!(
                    f,
                    "{INDENT}heap[{word}] = (heap[{word}] & 0x{mask:x}) | ((uint64_t){t} << {shift});",
                    t = t.ident
                )
            }
        }
        Some(arr_size) => {
            let word = heap_offset.bit_offset / 64;
            writeln!(
                f,
                "{INDENT}memmove(heap + {word}, &{t}, {arr_size} * sizeof(uint64_t));",
                t = t.ident
            )
        }
    }
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
    size: VectorSize,
    mdst: LogicMode,
    msrc: LogicMode,
    dst: CIdent,
    src: CIdent,
) -> io::Result<()> {
    use LogicMode as M;
    let dst_ty = CType { size, mode: mdst };
    let src_ty = CType { size, mode: msrc };
    let msbw_mask = if size.get() % 64 == 0 {
        u64::MAX
    } else {
        mask(size.get() % 64)
    };
    match (mdst, msrc, dst_ty.array_size(), src_ty.array_size()) {
        (M::FourValue, M::TwoValue, None, None) => {
            writeln!(
                f,
                "{INDENT}{dst} = ((({dst_elem_ty}){src}) << {size}) | 0x{mask:x};",
                dst_elem_ty = dst_ty.element_type(),
                mask = msbw_mask
            )
        }
        (M::FourValue, M::TwoValue, Some(_), None) => {
            writeln!(
                f,
                "{INDENT}{dst}[0] = 0x{msbw_mask:x}; {dst}[1] = (uint64_t){src};"
            )
        }
        (M::FourValue, M::TwoValue, Some(_), Some(arr_size)) => {
            let main_loop_size = if size.get() % 64 == 0 {
                arr_size
            } else {
                arr_size - 1
            };
            if main_loop_size > 0 {
                writeln!(
                    f,
                    "{INDENT}memset({dst}, 0xFF, {main_loop_size}*sizeof(uint64_t));"
                )?;
                writeln!(
                    f,
                    "{INDENT}memmove({dst}+{arr_size}, {src}, {main_loop_size}*sizeof(uint64_t));"
                )?;
            }
            if size.get() % 64 != 0 {
                let last_i = 2 * arr_size - 1;
                writeln!(f, "{INDENT}{dst}[{main_loop_size}] = 0x{msbw_mask:x};")?;
                writeln!(f, "{INDENT}{dst}[{last_i}] = {src}[{main_loop_size}];")?;
            }
            Ok(())
        }

        (M::TwoValue, M::FourValue, None, None) => {
            writeln!(
                f,
                "{INDENT}{dst} = ({dst_elem_ty})({src} >> {size});",
                dst_elem_ty = dst_ty.element_type(),
            )
        }
        (M::TwoValue, M::FourValue, None, Some(_)) => {
            writeln!(
                f,
                "{INDENT}{dst} = ({dst_elem_ty}){src}[1];",
                dst_elem_ty = dst_ty.element_type(),
            )
        }
        (M::TwoValue, M::FourValue, Some(arr_size), Some(_)) => {
            writeln!(
                f,
                "{INDENT}memmove({dst}, {src} + {arr_size}, {arr_size} * sizeof(uint64_t));"
            )
        }

        (M::TwoValue, M::FourValue, Some(_), None) | (M::FourValue, M::TwoValue, None, Some(_)) => {
            unreachable!()
        }
        (M::FourValue, M::FourValue, _, _) | (M::TwoValue, M::TwoValue, _, _) => unreachable!(),
    }
}

pub fn lower_signal_drive_header(
    f: &mut impl io::Write,
    signal: SignalKey,
    io_signals: &VgHashMap<SignalKey, RtSignalKey>,
) -> io::Result<()> {
    let idx = io_signals[&signal].as_u64();
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
    io_signals: &VgHashMap<SignalKey, RtSignalKey>,
) -> io::Result<()> {
    let idx = io_signals[&signal].as_u64();
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
    uint8_t mode;
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

static inline bool contains_special(uint64_t *src, uint32_t size) {
    size_t num_full_words = size / 64;
    bool contains_special = false;
    for (int i = 0; i < num_full_words; ++i) contains_special |= ~src[i] != 0;
    if (size % 64 != 0) {
        uint64_t mask = (1ULL << (size % 64)) - 1;
        contains_special |= (src[num_full_words] & mask) != mask;
    }
    return contains_special;
}

static inline void set_no_special(uint64_t *dst, uint32_t size) {
    size_t num_full_words = size / 64;
    for (int i = 0; i < num_full_words; ++i) dst[i] = ~0;;
    if (size % 64 != 0) {
        dst[num_full_words] = (1ULL << (size % 64)) - 1;
    }
}

static inline void tv_bigint_add_sub(uint64_t *dst, uint64_t *lhs, uint64_t *rhs, uint32_t size, bool subtract) {
    size_t num_words = (size + 63) / 64;
    if (num_words == 1) {
        if (subtract) {
            dst[0] = lhs[0] - rhs[0];
        } else {
            dst[0] = lhs[0] + rhs[0];
        }
        if (size % 64 != 0) {
            dst[0] &= (1ULL << (size % 64)) - 1;
        }
        return;
    }

    uint64_t mask = 0;
    if (subtract) {
        mask = ~(0ULL);
    }
    uint64_t carry_in = subtract;
    for (int i = 0; i < num_words; ++i) {
        dst[i] = __builtin_addcl(lhs[i], rhs[i] ^ mask, carry_in, &carry_in);
    }
    if (size % 64 != 0) {
        dst[num_words-1] &= (1ULL << (size % 64)) - 1;
    }
}

static inline void tv_bigint_mul(uint64_t *dst, uint64_t *lhs, uint64_t *rhs, uint32_t size) {
    size_t num_words = (size + 63) / 64;
    if (num_words == 1) {
        dst[0] = lhs[0] * rhs[0];
        if (size % 64 != 0) {
            dst[0] &= (1ULL << (size % 64)) - 1;
        }
        return;
    }

    memset(dst, 0, num_words*sizeof(uint64_t));

    // @Performance. This can probably be written in a better way.
    uint64_t carry_in, hi, lo;
    unsigned __int128 result;
    int i, j, k;
    for (j = 0; j < num_words; ++j) {
        for (k = 0; k < num_words; ++k) {
            result = ((unsigned __int128)lhs[j]) * ((unsigned __int128)rhs[k]);

            hi = (uint64_t)(result >> 64);
            lo = (uint64_t)(result & 0xFFFFFFFFFFFFFFFF);

            carry_in = 0;
            dst[j + k] = __builtin_addcl(dst[j + k], lo, false, &carry_in);
            if (j + k + 1 < num_words) {
                dst[j + k + 1] = __builtin_addcl(hi, dst[j + k + 1], carry_in, &carry_in);
                i = 2;
                while (carry_in && i + j + k < num_words) {
                    dst[i + j + k] = __builtin_addcl(dst[i + j + k], 0, carry_in, &carry_in);
                    i += 1;
                }
            }
        }
    }
    if (size % 64 != 0) {
        dst[num_words-1] &= (1ULL << (size % 64)) - 1;
    }
}
static inline void tv_l_concat(
    uint64_t *dst,
    uint64_t *lhs,
    uint64_t *rhs,
    uint32_t lhs_size,
    uint32_t rhs_size
) {
    uint32_t lwords = (lhs_size + 63) / 64;
    uint32_t rwords = (rhs_size + 63) / 64;
    uint32_t dwords = (lhs_size + rhs_size + 63) / 64;

    for (int i = 0; i < rwords; ++i) {
        dst[i] = rhs[i];
    }

    uint32_t roff = rhs_size % 64;

    // Fast path: left side is empty or right side is aligned.
    if (roff == 0) {
        for (int i = 0; i < lwords; ++i) {
            dst[rwords + i] = lhs[i];
        }
        return;
    }

    dst[rwords - 1] |= lhs[0] << roff;
    uint64_t s;
    if (lhs_size < (64 - roff)) s = 0;
    else                        s = lhs_size - (64 - roff);
    int i = 0;
    while (s > roff) {
        dst[rwords + i] = (lhs[i] >> (64 - roff)) | (lhs[i + 1] << roff);
        if (s < 64) s = 0;
        else        s -= 64;
        i += 1;
    }
    if (s > 0) {
        dst[dwords - 1] = lhs[lwords - 1] >> (64 - roff);
    }
}
static inline void tv_l_lsl_with(
    uint64_t *dst,
    uint64_t *src,
    uint32_t amount,
    uint32_t size,
    bool shiftin_value
) {
    uint32_t nwords = (size + 63) / 64;
    if (amount == 0) {
        memmove(dst, src, nwords*sizeof(uint64_t));
        return;
    }
    if (amount >= size) {
        memset(dst, ((uint8_t)(!shiftin_value)) - 1, nwords*sizeof(uint64_t));
        if (size % 64 != 0) dst[nwords - 1] &= (((uint64_t)1) << (size % 64)) - 1;
        return;
    }

    uint32_t swords = (amount + 63) / 64;
    uint32_t soff = amount % 64;
    uint64_t shiftin_mask = ((uint64_t)(!shiftin_value)) - 1;
    if (soff == 0) {
        memset(dst, ((uint8_t)(!shiftin_value)) - 1, swords*sizeof(uint64_t));
        memmove(dst+swords, src, (nwords - swords)*sizeof(uint64_t));
    } else {
        memset(dst, ((uint8_t)(!shiftin_value)) - 1, (swords-1)*sizeof(uint64_t));
        dst[swords - 1] = (src[0] << soff) | (shiftin_mask >> (64 - soff));
        for (int i = 0; i < nwords - swords; ++i)
            dst[i + swords] = (src[i + 1] << soff) | (src[i] >> (64 - soff));
    }

    if (size % 64 != 0) {
        dst[nwords - 1] &= (((uint64_t)1) << (size % 64)) - 1;
    }
}
static inline void tv_l_lsr_with(
    uint64_t *dst,
    uint64_t *src,
    uint32_t amount,
    uint32_t size,
    bool shiftin_value
) {
    uint32_t nwords = (size + 63) / 64;
    if (amount == 0) {
        memmove(dst, src, nwords*sizeof(uint64_t));
        return;
    }
    if (amount >= size) {
        memset(dst, ((uint8_t)(!shiftin_value)) - 1, nwords*sizeof(uint64_t));
        if (size % 64 != 0) dst[nwords - 1] &= (((uint64_t)1) << (size % 64)) - 1;
        return;
    }

    uint32_t swords = (amount + 63) / 64;
    uint32_t soff = amount % 64;
    uint64_t shiftin_mask = ((uint64_t)(!shiftin_value)) - 1;
    if (soff == 0) {
        memmove(dst, src + swords, (nwords - swords)*sizeof(uint64_t));
        memset(dst+(nwords-swords), ((uint8_t)(!shiftin_value)) - 1, swords*sizeof(uint64_t));
    } else {
        for (int i = 0; i < nwords - swords; ++i)
            dst[i] = (src[i + swords] << (64 - soff)) | (src[i + swords - 1] >> soff);
        dst[nwords - swords] = (shiftin_mask << (64 - soff)) | (src[nwords - 1] >> soff);
        memset(dst+(nwords-swords+1), ((uint8_t)(!shiftin_value)) - 1, (swords - 1)*sizeof(uint64_t));
    }

    if (size % 64 != 0) {
        uint64_t mask = shiftin_mask << (size % 64);
        if (shiftin_value) {
            dst[nwords - amount / 64 - 1] |= mask >> soff;
            if (nwords >= amount / 64 + 2) {
                dst[nwords - amount / 64 - 2] |= mask << (64 - soff);
            }
        }
        dst[nwords - 1] &= (((uint64_t)1) << (size % 64)) - 1;
    }
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

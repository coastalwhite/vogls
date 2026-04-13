use std::fmt::Write;
use std::{fmt, io};

use vogls_bits::BitsDataRef;
use vogls_codegen::{
    HeapBuilder, HeapOffset, HeapRef, bin_args_need_conversion, bin_imm_args_need_conversion,
    insert_bb_phis, resolve_heap_map, resolve_var_logic_mode_map,
};
use vogls_ir::dyn_format_string::{DynFormatArgument, DynFormatString};
use vogls_ir::{
    BasicBlockKey, BasicBlockTerminator, BinaryImmOp, Bits, ContextFormat, DisplayContext,
    GlobalContext, INTEGER_VSIZE, Instruction, LogicMode, Mode, Process, ProcessKey, ReadMem,
    ResizeOp, ShiftImmOp, SignalKey, TIME_VSIZE, UnaryOp, VariableKey, VectorSize,
};
use vogls_ir_properties::get_temporal_variables;
use vogls_runtime::RtSignalKey;
use vogls_runtime::plugins::RuntimePluginState;
use vogls_utils::{IndexSet, VgHashMap, VgHashSet};

pub mod runtime;

mod binary;
mod drive;
mod resize;
mod slice;
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
enum CIdent {
    Numbered(u64),
    Scoped(u64),
    HeapWords(u64),
}
impl fmt::Display for CIdent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Numbered(t) => {
                f.write_char('t')?;
                t.fmt(f)
            }
            Self::Scoped(t) => {
                f.write_char('s')?;
                t.fmt(f)
            }
            Self::HeapWords(t) => {
                write!(f, "(heap+{t})")
            }
        }
    }
}

#[derive(Clone, Copy)]
enum CExpr<'a> {
    Ident(CVar),
    HeapRef(HeapRef, LogicMode),
    Bits(&'a Bits, LogicMode),
}
impl CExpr<'_> {
    fn ty(self) -> CType {
        match self {
            CExpr::Ident(var) => var.ty,
            CExpr::HeapRef(heap_ref, mode) => CType {
                size: heap_ref.size,
                mode,
            },
            CExpr::Bits(bits, mode) => CType {
                size: bits.size(),
                mode,
            },
        }
    }
}

impl<'a> From<CVar> for CExpr<'a> {
    fn from(value: CVar) -> Self {
        Self::Ident(value)
    }
}

impl fmt::Display for CExpr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ident(var) => var.ident.fmt(f),
            Self::HeapRef(heap_ref, mode) => {
                let ty = CType {
                    size: heap_ref.size,
                    mode: *mode,
                };
                match ty.array_size() {
                    None => {
                        let word = heap_ref.offset.bit_offset / 64;
                        let shift = heap_ref.offset.bit_offset % 64;
                        let num_bits = ty.num_bits();
                        let mask = mask(num_bits);
                        write!(f, "((heap[{word}] >> {shift}) & 0x{mask:x})")
                    }
                    Some(_) => {
                        let word = heap_ref.offset.bit_offset / 64;
                        write!(f, "(heap+{word})",)
                    }
                }
            }
            Self::Bits(bits, mode) => match mode {
                LogicMode::TwoValue => match bits.as_data_ref() {
                    BitsDataRef::InlineTv(v) => write!(f, "((uint64_t)0x{v:x})"),
                    BitsDataRef::SeparateTv(v) => {
                        write!(f, "(uint64_t[{}]){{0x{:x}", v.len(), v[0])?;
                        for v in &v[1..] {
                            write!(f, ",0x{v:x}")?;
                        }
                        f.write_char('}')
                    }
                    BitsDataRef::InlineFv(..) | BitsDataRef::SeparateFv(..)
                        if bits.contains_special() =>
                    {
                        unreachable!()
                    }
                    BitsDataRef::InlineFv(_, v) => write!(f, "((uint64_t)0x{v:x})"),
                    BitsDataRef::SeparateFv(v) if bits.size().get() <= 64 => {
                        write!(f, "((uint64_t)0x{:x})", v[1])
                    }
                    BitsDataRef::SeparateFv(v) => {
                        let v = &v[v.len() / 2..];
                        write!(f, "(uint64_t[{}]){{0x{:x}", v.len(), v[0])?;
                        for v in &v[1..] {
                            write!(f, ",0x{v:x}")?;
                        }
                        f.write_char('}')
                    }
                },
                LogicMode::FourValue => match bits.as_data_ref() {
                    BitsDataRef::InlineTv(v) => {
                        let mask = mask(bits.size().get());
                        if bits.size() <= Mode::FourValue.max_inline_size() {
                            write!(f, "((uint64_t)0x{:x})", (v << bits.size().get()) | mask)
                        } else {
                            write!(f, "(uint64_t[2]){{0x{:x}, 0x{:x}}}", mask, v)
                        }
                    }
                    BitsDataRef::SeparateTv(v) => {
                        write!(f, "(uint64_t[{}]){{0x{:x}", v.len(), v[0])?;
                        for v in &v[1..] {
                            write!(f, ",0x{v:x}")?;
                        }
                        f.write_char('}')
                    }
                    BitsDataRef::InlineFv(spc, val) => {
                        write!(f, "((uint64_t)0x{:x})", (val << bits.size().get()) | spc)
                    }
                    BitsDataRef::SeparateFv(v) => {
                        write!(f, "(uint64_t[{}]){{0x{:x}", v.len(), v[0])?;
                        for v in &v[1..] {
                            write!(f, ",0x{v:x}")?;
                        }
                        f.write_char('}')
                    }
                },
            },
        }
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
    pub fn insert_signals(
        &mut self,
        signals: &[SignalKey],
        process_idx: usize,
        process_key: ProcessKey,
        state: u32,
    ) {
        for &signal in signals {
            self.map.entry(signal).or_default().push(Listener {
                offset: self.top,
                process_idx,
                process_key,
                state,
            });
        }
        self.top += 1;
    }
}

fn write_var(f: &mut impl io::Write, name: impl fmt::Display, ty: CType) -> io::Result<()> {
    write!(f, "{INDENT}{} {}", ty.element_type(), name)?;
    if let Some(array_size) = ty.array_size() {
        write!(f, "[{array_size}]")?;
    }
    writeln!(f, ";")
}
fn write_cvar(f: &mut impl io::Write, var: CVar) -> io::Result<()> {
    write_var(f, var.ident, var.ty)
}

pub struct CLowerOptions {
    pub itrace: bool,
    pub stats: bool,
    pub num_plugins: usize,
}

#[derive(Default)]
pub struct StateBuilder {
    pub dyn_fmt_strs: IndexSet<DynFormatString>,
    pub read_mems: Vec<(HeapRef, ReadMem)>,
}

pub fn lower_process(
    f: &mut impl io::Write,
    process_key: ProcessKey,
    process_idx: usize,
    gl: &GlobalContext,
    heap_builder: &mut HeapBuilder,
    listener_builder: &mut ListenerBuilder,
    state_builder: &mut StateBuilder,
    io_signals: &VgHashMap<SignalKey, RtSignalKey>,
    lupdt_indexes: &VgHashMap<RtSignalKey, u64>,
    signals: &[HeapRef],
    lower_options: &CLowerOptions,
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
    insert_bb_phis(process.entry, gl, &mut bb_stack, &mut bb_seen, &mut bb_phis);
    resolve_heap_map(
        process.entry,
        gl,
        &mut bb_stack,
        &mut bb_seen,
        &var_mode,
        &mut conv_map,
        heap_builder,
        &mut heap_map,
        Some(&temporal_variables),
        None,
    );

    // @Performance. Amortize buffer.
    let mut buffer = Vec::<u8>::new();
    let procedure = process_to_procedure_name(process, process_idx);

    writeln!(
        f,
        "NOINLINE __attribute__((preserve_none)) int {procedure}(int state, uint64_t *restrict heap, schedule_t *restrict schedule, uint64_t time, uint64_t *restrict listening, uint64_t *restrict last_active_time, cold_context_t *restrict cldctx) {{",
    )?;
    if lower_options.itrace {
        lower_dyn_format_str(
            f,
            &mut state_builder.dyn_fmt_strs,
            &DynFormatString::new(format!("\n* PROC {procedure}\n").into(), [].into()),
            [].into(),
        )?;
    }

    let mut bb_ident = IndexSet::<BasicBlockKey>::new();
    let mut states_set = IndexSet::<BasicBlockKey>::new();
    let mut goto_target_set = IndexSet::<BasicBlockKey>::new();
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
                ident: CIdent::Numbered(temp_map.len() as u64),
                ty: CType {
                    size: gl.vars[v].size,
                    mode,
                },
            };

            write_cvar(f, t)?;
            temp_map.insert((v, t.ty.mode), t);

            if conv_map.contains_key(&v) {
                let mut t = t;
                t.ident = CIdent::Numbered(temp_map.len() as u64);
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
            BasicBlockTerminator::Jump(target) => _ = goto_target_set.insert(target),
            BasicBlockTerminator::Branch(_, truthy, falsy) => {
                goto_target_set.extend([truthy, falsy])
            }
            BasicBlockTerminator::Halt => {}
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

    let mut display_context = DisplayContext::new(gl);
    if lower_options.itrace {
        display_context.prepare_process(process.entry);
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

        if goto_target_set.contains(&bb_key)
            || (states_set.len() > 1 && states_set.contains(&bb_key))
        {
            writeln!(buffer, "L{ident}:")?;
        }

        for i in &bb.instrs {
            if lower_options.itrace {
                writeln!(buffer, "{INDENT}// {}", i.display(&display_context))?;
            }
            match i {
                I::Constant(dst, bits) => {
                    let mode = var_mode[dst];
                    let t = temp_map[&(*dst, mode)];
                    match t.ty.array_size() {
                        None => {
                            writeln!(buffer, "{INDENT}{} = {};", t.ident, CExpr::Bits(bits, mode))?
                        }
                        Some(n) => writeln!(
                            buffer,
                            "{INDENT}memcpy({}, {}, {n}*sizeof(uint64_t));",
                            t.ident,
                            CExpr::Bits(bits, mode)
                        )?,
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
                        O::Truncate => resize::cgc_truncate(&mut buffer, dst_t, t.into())?,
                        O::ZeroExtend => resize::cgc_zero_extend(&mut buffer, dst_t, t)?,
                        O::SignExtend => resize::cgc_sign_extend(&mut buffer, dst_t, t)?,
                    }
                    if temporal_variables.contains(dst) {
                        store(&mut buffer, heap_map[dst], dst_t)?;
                    }
                }
                I::BinaryImm(dst, op, src, imm) => {
                    let src_size = gl.vars[*src].size;
                    let mimm = if imm.contains_special() {
                        LogicMode::FourValue
                    } else {
                        LogicMode::TwoValue
                    };
                    let (msrc, mdst) = (var_mode[src], var_mode[dst]);

                    let (mtgt, conv_src, conv_imm) =
                        bin_imm_args_need_conversion(*op, mdst, msrc, mimm);

                    let mut src_t = temp_map[&(*src, msrc)];
                    if temporal_variables.contains(src) {
                        load(&mut buffer, heap_map[src], src_t)?;
                    }
                    if conv_src {
                        let unconverted_t = src_t;
                        src_t = temp_map[&(*src, mtgt)];
                        convert(
                            &mut buffer,
                            src_size,
                            mtgt,
                            msrc,
                            src_t.ident,
                            unconverted_t.ident,
                        )?;
                    }
                    let imm_tgt_mode = if conv_imm { mtgt } else { mimm };
                    let imm = CExpr::Bits(imm, imm_tgt_mode);

                    let dst_t = temp_map[&(*dst, mdst)];

                    use BinaryImmOp as O;
                    match op {
                        O::And => binary::cgc_bin_and(&mut buffer, dst_t, src_t.into(), imm)?,
                        O::Or => binary::cgc_bin_or(&mut buffer, dst_t, src_t.into(), imm)?,
                        O::Xor => binary::cgc_bin_xor(&mut buffer, dst_t, src_t.into(), imm)?,
                        O::Add => binary::cgc_bin_add(&mut buffer, dst_t, src_t.into(), imm)?,
                        O::Sub => binary::cgc_bin_sub(&mut buffer, dst_t, src_t.into(), imm)?,
                        O::Power => binary::cgc_bin_pow(&mut buffer, dst_t, src_t.into(), imm)?,
                        O::Multiply => binary::cgc_bin_mul(&mut buffer, dst_t, src_t.into(), imm)?,
                        O::Divide => binary::cgc_bin_div(&mut buffer, dst_t, src_t.into(), imm)?,
                        O::Modulus => binary::cgc_bin_mod(&mut buffer, dst_t, src_t.into(), imm)?,
                        O::RevSub => binary::cgc_bin_sub(&mut buffer, dst_t, imm, src_t.into())?,
                        O::RevPower => binary::cgc_bin_pow(&mut buffer, dst_t, imm, src_t.into())?,
                        O::RevDivide => binary::cgc_bin_div(&mut buffer, dst_t, imm, src_t.into())?,
                        O::RevModulus => {
                            binary::cgc_bin_mod(&mut buffer, dst_t, imm, src_t.into())?
                        }
                        O::UnsignedLessEqual => {
                            binary::cgc_bin_ule(&mut buffer, dst_t.ident, src_t.into(), imm)?
                        }
                        O::UnsignedGreaterEqual => {
                            binary::cgc_bin_ule(&mut buffer, dst_t.ident, imm, src_t.into())?
                        }
                        O::ConcatRight => {
                            binary::cgc_concat(&mut buffer, dst_t, src_t.into(), imm)?
                        }
                        O::ConcatLeft => binary::cgc_concat(&mut buffer, dst_t, imm, src_t.into())?,
                        O::Min => binary::cgc_bin_min(&mut buffer, dst_t, src_t.into(), imm)?,
                        O::Max => binary::cgc_bin_max(&mut buffer, dst_t, src_t.into(), imm)?,
                        O::CaseEquality => {
                            binary::cgc_case_eq(&mut buffer, dst_t.ident, src_t.into(), imm)?
                        }
                    }

                    if temporal_variables.contains(dst) {
                        store(&mut buffer, heap_map[dst], dst_t)?;
                    }
                }
                I::SliceImm(dst, src, offset) => {
                    let src_size = gl.vars[*src].size;
                    let (msrc, mdst) = (var_mode[src], var_mode[dst]);

                    let (conv_src, conv_imm) = (msrc != mdst, mdst == LogicMode::FourValue);

                    let mut src_t = temp_map[&(*src, msrc)];
                    if temporal_variables.contains(src) {
                        load(&mut buffer, heap_map[src], src_t)?;
                    }
                    if conv_src {
                        let unconverted_t = src_t;
                        src_t = temp_map[&(*src, mdst)];
                        convert(
                            &mut buffer,
                            src_size,
                            mdst,
                            msrc,
                            src_t.ident,
                            unconverted_t.ident,
                        )?;
                    }
                    let imm_tgt_mode = if conv_imm { mdst } else { LogicMode::TwoValue };
                    let imm =
                        CExpr::Bits(&Bits::from_u64(INTEGER_VSIZE, *offset as u64), imm_tgt_mode);

                    let dst_t = temp_map[&(*dst, mdst)];
                    slice::slice_with(&mut buffer, dst_t, src_t.into(), imm, false)?;
                    if temporal_variables.contains(dst) {
                        store(&mut buffer, heap_map[dst], dst_t)?;
                    }
                }
                I::ShiftImm(dst, op, src, offset) => {
                    let src_size = gl.vars[*src].size;
                    let (msrc, mdst) = (var_mode[src], var_mode[dst]);

                    let (conv_src, conv_imm) = (msrc != mdst, mdst == LogicMode::FourValue);

                    let mut src_t = temp_map[&(*src, msrc)];
                    if temporal_variables.contains(src) {
                        load(&mut buffer, heap_map[src], src_t)?;
                    }
                    if conv_src {
                        let unconverted_t = src_t;
                        src_t = temp_map[&(*src, mdst)];
                        convert(
                            &mut buffer,
                            src_size,
                            mdst,
                            msrc,
                            src_t.ident,
                            unconverted_t.ident,
                        )?;
                    }
                    let imm_tgt_mode = if conv_imm { mdst } else { LogicMode::TwoValue };
                    let imm =
                        CExpr::Bits(&Bits::from_u64(INTEGER_VSIZE, *offset as u64), imm_tgt_mode);

                    let dst_t = temp_map[&(*dst, mdst)];
                    use ShiftImmOp as O;
                    match op {
                        O::LogicalShiftLeft => {
                            binary::cgc_lsl(&mut buffer, dst_t, src_t.into(), imm)?
                        }
                        O::LogicalShiftRight => {
                            binary::cgc_lsr(&mut buffer, dst_t, src_t.into(), imm)?
                        }
                        O::ArithmeticShiftRight => {
                            binary::cgc_asr(&mut buffer, dst_t, src_t.into(), imm)?
                        }
                    }

                    if temporal_variables.contains(dst) {
                        store(&mut buffer, heap_map[dst], dst_t)?;
                    }
                }
                I::Slice(dst, lhs, rhs) => {
                    let lhs_size = gl.vars[*lhs].size;
                    let rhs_size = gl.vars[*rhs].size;
                    let (mlhs, mrhs, mdst) = (var_mode[lhs], var_mode[rhs], var_mode[dst]);

                    use LogicMode as M;
                    let (mtgt, conv_lhs, conv_rhs) = (
                        M::FourValue,
                        mlhs == M::TwoValue && mrhs == M::FourValue,
                        mlhs == M::FourValue && mrhs == M::TwoValue,
                    );

                    let (mut lhs_t, mut rhs_t) = (temp_map[&(*lhs, mlhs)], temp_map[&(*rhs, mrhs)]);
                    if temporal_variables.contains(lhs) {
                        load(&mut buffer, heap_map[lhs], lhs_t)?;
                    }
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
                    slice::slice(&mut buffer, dst_t, lhs_t.into(), rhs_t.into())?;
                    if temporal_variables.contains(dst) {
                        store(&mut buffer, heap_map[dst], dst_t)?;
                    }
                }
                I::Binary(dst, op, lhs, rhs) => {
                    let lhs_size = gl.vars[*lhs].size;
                    let rhs_size = gl.vars[*rhs].size;
                    let (mlhs, mrhs, mdst) = (var_mode[lhs], var_mode[rhs], var_mode[dst]);

                    let (mtgt, conv_lhs, conv_rhs) =
                        bin_args_need_conversion(*op, mdst, mlhs, mrhs);

                    let (mut lhs_t, mut rhs_t) = (temp_map[&(*lhs, mlhs)], temp_map[&(*rhs, mrhs)]);
                    if temporal_variables.contains(lhs) {
                        load(&mut buffer, heap_map[lhs], lhs_t)?;
                    }
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
                            binary::cgc_bin_and(&mut buffer, dst_t, lhs_t.into(), rhs_t.into())?
                        }
                        O::Or => {
                            binary::cgc_bin_or(&mut buffer, dst_t, lhs_t.into(), rhs_t.into())?
                        }
                        O::Xor => {
                            binary::cgc_bin_xor(&mut buffer, dst_t, lhs_t.into(), rhs_t.into())?
                        }
                        O::Add => {
                            binary::cgc_bin_add(&mut buffer, dst_t, lhs_t.into(), rhs_t.into())?
                        }
                        O::Sub => {
                            binary::cgc_bin_sub(&mut buffer, dst_t, lhs_t.into(), rhs_t.into())?
                        }
                        O::Power => {
                            binary::cgc_bin_pow(&mut buffer, dst_t, lhs_t.into(), rhs_t.into())?
                        }
                        O::Multiply => {
                            binary::cgc_bin_mul(&mut buffer, dst_t, lhs_t.into(), rhs_t.into())?
                        }
                        O::Divide => {
                            binary::cgc_bin_div(&mut buffer, dst_t, lhs_t.into(), rhs_t.into())?
                        }
                        O::Modulus => {
                            binary::cgc_bin_mod(&mut buffer, dst_t, lhs_t.into(), rhs_t.into())?
                        }
                        O::UnsignedLessEqual => binary::cgc_bin_ule(
                            &mut buffer,
                            dst_t.ident,
                            lhs_t.into(),
                            rhs_t.into(),
                        )?,

                        O::LogicalShiftLeft => {
                            binary::cgc_lsl(&mut buffer, dst_t, lhs_t.into(), rhs_t.into())?
                        }
                        O::LogicalShiftRight => {
                            binary::cgc_lsr(&mut buffer, dst_t, lhs_t.into(), rhs_t.into())?
                        }
                        O::ArithmeticShiftRight => {
                            binary::cgc_asr(&mut buffer, dst_t, lhs_t.into(), rhs_t.into())?
                        }
                        O::Concat => {
                            binary::cgc_concat(&mut buffer, dst_t, lhs_t.into(), rhs_t.into())?
                        }
                        O::CopyX => {
                            binary::cgc_copy_x(&mut buffer, dst_t, lhs_t.into(), rhs_t.into())?
                        }
                        O::CopyZ => {
                            binary::cgc_copy_y(&mut buffer, dst_t, lhs_t.into(), rhs_t.into())?
                        }
                        O::Min => {
                            binary::cgc_bin_min(&mut buffer, dst_t, lhs_t.into(), rhs_t.into())?
                        }
                        O::Max => {
                            binary::cgc_bin_max(&mut buffer, dst_t, lhs_t.into(), rhs_t.into())?
                        }
                        O::CaseEquality => binary::cgc_case_eq(
                            &mut buffer,
                            dst_t.ident,
                            lhs_t.into(),
                            rhs_t.into(),
                        )?,
                        O::Posedge => binary::cgc_posedge(
                            &mut buffer,
                            dst_t.ident,
                            lhs_t.into(),
                            rhs_t.into(),
                        )?,
                        O::Negedge => binary::cgc_negedge(
                            &mut buffer,
                            dst_t.ident,
                            lhs_t.into(),
                            rhs_t.into(),
                        )?,
                    }
                    if temporal_variables.contains(dst) {
                        store(&mut buffer, heap_map[dst], dst_t)?;
                    }
                }
                I::Intrinsic(dst, op, items) => match op.as_ref() {
                    vogls_ir::IntrinsicOp::Time => {
                        let t = temp_map[&(*dst, LogicMode::TwoValue)];
                        writeln!(buffer, "{INDENT}{} = time;", t.ident)?;
                        if temporal_variables.contains(dst) {
                            store(&mut buffer, heap_map[dst], t)?;
                        }
                    }
                    vogls_ir::IntrinsicOp::Finish => {
                        writeln!(buffer, "{INDENT}return 1;")?;
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
                        lower_dyn_format_str(
                            &mut buffer,
                            &mut state_builder.dyn_fmt_strs,
                            &dyn_format_string,
                            args,
                        )?;
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
                        lower_dyn_format_str(
                            &mut buffer,
                            &mut state_builder.dyn_fmt_strs,
                            &dyn_format_string,
                            args,
                        )?;
                        writeln!(
                            buffer,
                            r#"{INDENT}{INDENT}return 2;
{INDENT}}}"#
                        )?;
                    }
                    vogls_ir::IntrinsicOp::VcdOpenFile(_) => todo!(),
                    vogls_ir::IntrinsicOp::VcdAppendModule(_) => todo!(),
                    vogls_ir::IntrinsicOp::VcdPause => todo!(),
                    vogls_ir::IntrinsicOp::VcdResume => todo!(),
                    vogls_ir::IntrinsicOp::ReadMem(readmem) => {
                        let i = state_builder.read_mems.len();
                        state_builder.read_mems.push((
                            signals[io_signals[&readmem.signal].as_usize()],
                            readmem.as_ref().clone(),
                        ));
                        writeln!(
                            buffer,
                            "{INDENT}cldctx->readmem(heap, cldctx->heap_len, {}, cldctx->readmems+{});",
                            match gl.logic_mode {
                                LogicMode::TwoValue => 0,
                                LogicMode::FourValue => 0,
                            },
                            i * size_of::<(HeapRef, ReadMem)>(),
                        )?;
                    }
                },
                I::LastUpdateTime(dst, signal) => {
                    let t = temp_map[&(*dst, LogicMode::TwoValue)];
                    let rt_key = io_signals[signal];
                    let idx = lupdt_indexes[&rt_key];
                    writeln!(buffer, "{INDENT}{} = last_active_time[{idx}];", t.ident)?;
                    if temporal_variables.contains(dst) {
                        store(&mut buffer, heap_map[dst], t)?;
                    }
                }
                I::Probe(dst, signal, offset) => {
                    let signal_ref = signals[io_signals[signal].as_usize()];
                    assert_eq!(var_mode[dst], gl.logic_mode);

                    let t = temp_map[&(*dst, gl.logic_mode)];
                    let dst_size = gl.vars[*dst].size;
                    let src_size = gl.signals[*signal].size;

                    if dst_size == src_size && *offset == 0 {
                        load(&mut buffer, signal_ref.offset, t)?;
                    } else {
                        let signal = CExpr::HeapRef(signal_ref, gl.logic_mode);
                        let offset_c = CExpr::Bits(&Bits::new_u32(*offset), LogicMode::TwoValue);
                        if *offset > 0 {
                            slice::slice_with(&mut buffer, t, signal, offset_c, false)?;
                        } else {
                            resize::cgc_truncate(&mut buffer, t, signal)?;
                        }
                    }
                    if temporal_variables.contains(dst) {
                        store(&mut buffer, heap_map[dst], t)?;
                    }
                }
                I::ProbeSlice(dst, signal, offset) => {
                    let signal_ref = signals[io_signals[signal].as_usize()];
                    assert_eq!(var_mode[dst], LogicMode::FourValue);

                    let t = temp_map[&(*dst, LogicMode::FourValue)];

                    let moffset = var_mode[offset];
                    let offset_t = temp_map[&(*offset, moffset)];
                    if temporal_variables.contains(offset) {
                        load(&mut buffer, heap_map[offset], offset_t)?;
                    }

                    slice::slice(
                        &mut buffer,
                        t,
                        CExpr::HeapRef(signal_ref, gl.logic_mode),
                        offset_t.into(),
                    )?;
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
                    let partial = match partial {
                        None => None,
                        Some((offset, partial_size)) => {
                            let moffset = var_mode[offset];
                            let mut offset_t = temp_map[&(*offset, moffset)];
                            if temporal_variables.contains(offset) {
                                load(&mut buffer, heap_map[offset], offset_t)?;
                            }
                            if moffset != gl.logic_mode {
                                let unconverted_t = offset_t;
                                offset_t = temp_map[&(*offset, gl.logic_mode)];
                                convert(
                                    &mut buffer,
                                    INTEGER_VSIZE,
                                    gl.logic_mode,
                                    moffset,
                                    offset_t.ident,
                                    unconverted_t.ident,
                                )?;
                            }
                            Some((offset_t, *partial_size))
                        }
                    };
                    let dst = io_signals[signal];
                    drive::drive(&mut buffer, signals, dst, t, partial)?;
                }
                I::Phi(_, _) => continue,
            }
            if lower_options.stats {
                writeln!(&mut buffer, "{INDENT}cldctx->icount++;")?;
            }
            if lower_options.itrace && !matches!(i, I::Phi(_, _)) {
                lower_dyn_format_str(
                    &mut buffer,
                    &mut state_builder.dyn_fmt_strs,
                    &DynFormatString::new(
                        format!("* {}\n", i.display(&display_context)).into(),
                        [].into(),
                    ),
                    [].into(),
                )?;
                writeln!(&mut buffer)?;
                let mut content = String::from("*   : ");
                let mut arg_offsets = Vec::new();
                let mut args = Vec::new();
                if let Some(dst) = i.get_destination_variable() {
                    let mode = var_mode[&dst];
                    let var = temp_map[&(dst, mode)];
                    content.push_str(&dst.display(&display_context).to_string());
                    content.push_str(" = ");
                    arg_offsets.push((content.len(), DynFormatArgument::default()));
                    content.push_str("; ");
                    args.push(var);
                }
                i.for_each_src(|src| {
                    let mode = var_mode[&src];
                    let var = temp_map[&(src, mode)];
                    content.push_str(&src.display(&display_context).to_string());
                    content.push_str(" = ");
                    arg_offsets.push((content.len(), DynFormatArgument::default()));
                    content.push_str("; ");
                    args.push(var);
                });
                content.push('\n');
                lower_dyn_format_str(
                    &mut buffer,
                    &mut state_builder.dyn_fmt_strs,
                    &DynFormatString::new(content.into(), arg_offsets.into()),
                    args,
                )?;
                writeln!(&mut buffer)?;
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

                if lower_options.itrace {
                    writeln!(&mut buffer, "{INDENT}// Phi({dst:?}, {src:?});")?;
                }
                let d = dst_t.ident;
                let s = src_t.ident;
                match dst_t.ty.array_size() {
                    None => writeln!(buffer, "{INDENT}{d} = {s};")?,
                    Some(num_words) => writeln!(
                        buffer,
                        "{INDENT}memcpy({d}, {s}, {num_words}*sizeof(uint64_t));"
                    )?,
                }
                if temporal_variables.contains(dst) {
                    store(&mut buffer, heap_map[dst], dst_t)?;
                }
                if lower_options.itrace {
                    lower_dyn_format_str(
                        &mut buffer,
                        &mut state_builder.dyn_fmt_strs,
                        &DynFormatString::new(
                            format!(
                                "* {} = phi {}\n",
                                dst.display(&display_context),
                                src.display(&display_context)
                            )
                            .into(),
                            [].into(),
                        ),
                        [].into(),
                    )?;
                    writeln!(&mut buffer)?;
                    let mut content = String::from("*   : ");
                    let mut arg_offsets = Vec::new();
                    let mut args = Vec::new();
                    content.push_str(&dst.display(&display_context).to_string());
                    content.push_str(" = ");
                    arg_offsets.push((content.len(), DynFormatArgument::default()));
                    content.push_str("; ");
                    args.push(dst_t);
                    content.push_str(&src.display(&display_context).to_string());
                    content.push_str(" = ");
                    arg_offsets.push((content.len(), DynFormatArgument::default()));
                    content.push_str("; ");
                    args.push(src_t);
                    content.push('\n');
                    lower_dyn_format_str(
                        &mut buffer,
                        &mut state_builder.dyn_fmt_strs,
                        &DynFormatString::new(content.into(), arg_offsets.into()),
                        args,
                    )?;
                    writeln!(&mut buffer)?;
                }
            }
        }

        fn next_event_or_return_0(f: &mut impl io::Write) -> io::Result<()> {
            writeln!(
                f,
                r#"{INDENT}{{
{INDENT}event_t e;
{INDENT}if (!event_vec_pop(&schedule->active_region, &e)) {{
{INDENT}{INDENT}return 0;
{INDENT}}}
{INDENT}[[clang::musttail]] return (e.ptr)(e.state, heap, schedule, time, listening, last_active_time, cldctx);
{INDENT}}}"#
            )
        }

        match &bb.terminator {
            BasicBlockTerminator::Wait(bb_key, time) => {
                let time = time.0;
                let state = states_set.get_index(bb_key).unwrap();
                writeln!(
                    buffer,
                    "{INDENT}schedule_future_event(schedule, time + {time}, (event_t){{.ptr=&{procedure}, .state={state}}});"
                )?;
                next_event_or_return_0(&mut buffer)?;
            }
            BasicBlockTerminator::VariableWait(bb_key, time) => {
                let mtime = var_mode[time];
                let t = temp_map[&(*time, mtime)];
                if temporal_variables.contains(time) {
                    load(&mut buffer, heap_map[time], t)?;
                }
                let state = states_set.get_index(bb_key).unwrap();
                writeln!(buffer, "{INDENT}{{")?;
                let s0 = CVar {
                    ident: CIdent::Scoped(0),
                    ty: CType {
                        size: TIME_VSIZE,
                        mode: LogicMode::TwoValue,
                    },
                };
                write_cvar(&mut buffer, s0)?;
                let t = t.ident;
                let s0 = s0.ident;
                match mtime {
                    LogicMode::TwoValue => writeln!(buffer, "{INDENT}{s0} = {t};")?,
                    LogicMode::FourValue => {
                        writeln!(buffer, "{INDENT}{s0} = (~{t}[0] == 0) ? 0 : {t}[1];")?;
                    }
                }
                writeln!(
                    buffer,
                    "{INDENT}schedule_future_event(schedule, time + {s0}, (event_t){{.ptr=&{procedure}, .state={state}}});",
                )?;
                next_event_or_return_0(&mut buffer)?;
                writeln!(buffer, "{INDENT}}}")?;
            }
            BasicBlockTerminator::WaitRegion(bb_key, region) => {
                let state = states_set.get_index(bb_key).unwrap();
                writeln!(
                    buffer,
                    "{INDENT}event_vec_push(&schedule->regions[{region}], (event_t){{.ptr=&{procedure}, .state={state}}});",
                )?;
                next_event_or_return_0(&mut buffer)?;
            }
            BasicBlockTerminator::Watch(bb_key, items) => {
                let state = states_set.get_index(bb_key).unwrap();
                let offset = listener_builder.top;
                writeln!(
                    buffer,
                    "{INDENT}listening[{}] |= 0x{:x};",
                    offset / 64,
                    1u64 << (offset % 64)
                )?;
                listener_builder.insert_signals(items, process_idx, process_key, state as u32);
                next_event_or_return_0(&mut buffer)?;
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

                let t = t.ident;
                match mcondition {
                    LogicMode::TwoValue => write!(buffer, "{INDENT}if ({t} == 1)")?,
                    LogicMode::FourValue => write!(buffer, "{INDENT}if ({t} == 3)")?,
                }
                writeln!(buffer, " {{ goto L{truthy}; }} else {{ goto L{falsy}; }}")?;
            }
            BasicBlockTerminator::Halt => {
                next_event_or_return_0(&mut buffer)?;
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
    dyn_fmt_strs: &mut IndexSet<DynFormatString>,
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
    let dyn_fmt_ptr = dyn_fmt_strs.insert_index(dyn_format_string.clone());
    write!(
        f,
        r#"}}; (cldctx->fmt)(cldctx->stdout, (cldctx->fmt_strs+{size_of_dyn_str}*{dyn_fmt_ptr}), args); }}"#,
        size_of_dyn_str = size_of::<DynFormatString>(),
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
                "{INDENT}memcpy(&{t}, heap + {word}, {arr_size} * sizeof(uint64_t));",
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
                "{INDENT}memcpy(heap + {word}, &{t}, {arr_size} * sizeof(uint64_t));",
                t = t.ident
            )
        }
    }
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
                    "{INDENT}memcpy({dst}+{arr_size}, {src}, {main_loop_size}*sizeof(uint64_t));"
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
                "{INDENT}memcpy({dst}, {src} + {arr_size}, {arr_size} * sizeof(uint64_t));"
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
    use vogls_utils::TableKey;
    let idx = io_signals[&signal].get();
    writeln!(
        f,
        "void drive_signal_{idx}(schedule_t *schedule, uint64_t time, uint64_t *listening, uint64_t *last_active_time, cold_context_t *cldctx);"
    )
}

pub fn lower_signal_drive_fn(
    f: &mut impl io::Write,
    gl: &GlobalContext,
    signal: SignalKey,
    listener_builder: &ListenerBuilder,
    io_signals: &VgHashMap<SignalKey, RtSignalKey>,
    lupdt_indexes: &VgHashMap<RtSignalKey, u64>,
    state_builder: &mut StateBuilder,
    lower_options: &CLowerOptions,
) -> io::Result<()> {
    use vogls_utils::TableKey;
    let rt_key = io_signals[&signal];
    let idx = rt_key.get();
    writeln!(
        f,
        "void drive_signal_{idx}(schedule_t *schedule, uint64_t time, uint64_t *listening, uint64_t *last_active_time, cold_context_t *cldctx) {{",
    )?;

    if lower_options.itrace {
        let content = format!("* poke {}\n", gl.signals[signal].name).into();
        lower_dyn_format_str(
            f,
            &mut state_builder.dyn_fmt_strs,
            &DynFormatString::new(content, [].into()),
            [].into(),
        )?;
    }

    for i in 0..lower_options.num_plugins {
        writeln!(
            f,
            r#"{INDENT}cldctx->plugin_poke_signal(cldctx->plugins+{offset}, {idx});"#,
            offset = i * size_of::<RuntimePluginState>(),
        )?;
    }

    if let Some(lupdt_idx) = lupdt_indexes.get(&rt_key) {
        writeln!(f, "{INDENT}last_active_time[{lupdt_idx}] = time;")?;
    }
    if let Some(listeners) = listener_builder.map.get(&signal) {
        for listener in listeners {
            writeln!(
                f,
                "{INDENT}if ((listening[{}] >> {}) & 1) {{",
                listener.offset / 64,
                listener.offset % 64,
            )?;
            writeln!(
                f,
                "{INDENT}{INDENT}listening[{}] ^= ((uint64_t)1) << {};",
                listener.offset / 64,
                listener.offset % 64,
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
        b"int startup(
    uint64_t *heap,
    schedule_t *schedule,
    uint64_t time,
    uint64_t *listening,
    uint64_t *last_active_time,
    cold_context_t *cldctx
) {\n",
    )?;
    writeln!(f, "{INDENT}(void)heap;")?;
    writeln!(f, "{INDENT}(void)schedule;")?;
    writeln!(f, "{INDENT}(void)time;")?;
    writeln!(f, "{INDENT}(void)listening;")?;
    writeln!(f, "{INDENT}(void)last_active_time;")?;
    writeln!(f)?;
    writeln!(f, "{INDENT}int exit = 0;")?;
    for (i, process) in gl.processes.values().enumerate() {
        writeln!(
            f,
            "{INDENT}if ((exit = {}(0, heap, schedule, time, listening, last_active_time, cldctx)) != 0) return exit;",
            process_to_procedure_name(process, i)
        )?;
    }
    writeln!(f, "{INDENT}return 0;")?;
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

#if defined(_MSC_VER)
    #define NOINLINE __declspec(noinline)
#elif defined(__GNUC__) || defined(__clang__)
    #define NOINLINE __attribute__((noinline))
#else
    #define NOINLINE
#endif

typedef struct bits_ref {
    uint32_t size;
    uint8_t mode;
    uint64_t* ptr;
} bits_ref_t;

struct schedule;
typedef struct cold_context {
    void (*fmt)(void*, void*, bits_ref_t*);
    void *fmt_strs;

    void **plugins;
    void (*plugin_poke_signal)(void*, size_t);

    size_t heap_len;
    void *readmems;
    void (*readmem)(uint64_t*, size_t, uint8_t, void*);

    uint64_t icount;

    void *stdout;
    void *stderr;
} cold_context_t;

typedef struct event {
  int __attribute__((preserve_none)) (*ptr)(int, uint64_t*, struct schedule*, uint64_t, uint64_t*, uint64_t*, cold_context_t*);
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
static inline uint64_t min(uint64_t l, uint64_t r) {
    return (l < r) ? l : r;
}
static inline uint64_t max(uint64_t l, uint64_t r) {
    return (l < r) ? l : r;
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
        memcpy(dst, src, nwords*sizeof(uint64_t));
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
        memcpy(dst+swords, src, (nwords - swords)*sizeof(uint64_t));
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
        memcpy(dst, src, nwords*sizeof(uint64_t));
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
        memcpy(dst, src + swords, (nwords - swords)*sizeof(uint64_t));
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
static inline void tv_part_ll_slice(
    uint64_t *dst,
    uint64_t *src,
    uint32_t offset,
    uint32_t dst_size,
    uint32_t src_size,
    bool shiftin_value
) {
    uint32_t dst_words = (dst_size + 63)/64;
    uint32_t src_words = (src_size + 63)/64;
    if (offset == 0) {
        memcpy(dst, src, dst_words*sizeof(uint64_t));
        if (dst_size % 64 != 0) {
            dst[dst_words-1] &= (((uint64_t)1) << (dst_size%64))-1;
        }
        return;
    }
    uint64_t shiftin_mask = ((uint64_t)(!shiftin_value)) - 1;
    if (offset >= src_size) {
        memset(dst, shiftin_mask, dst_words*sizeof(uint64_t));
        return;
    }

    uint32_t swords = (offset+63)/64;
    uint32_t soff = offset % 64;
    uint32_t num_copy_words = min(src_words-swords, dst_words);
    if (soff == 0) {
        memcpy(dst, src+swords, num_copy_words*sizeof(uint64_t));
        memset(dst+num_copy_words, shiftin_mask, (dst_words-num_copy_words)*sizeof(uint64_t));
    } else {
        for (int i = 0; i < num_copy_words; ++i) {
            dst[i] = (src[i + swords] << (64 - soff)) | (src[i + swords - 1] >> soff);
        }
        if (num_copy_words < dst_words) {
            dst[num_copy_words] = (shiftin_mask << (64 - soff)) | (src[src_words - 1] >> soff);
            memset(dst+num_copy_words+1, shiftin_mask, (dst_words-num_copy_words-1)*sizeof(uint64_t));
        }
    }
    if (dst_size % 64 != 0) {
        dst[dst_words-1] &= (((uint64_t)1) << (dst_size%64))-1;
    }
}
static inline void tv_ll_slice(
    uint64_t *dst,
    uint64_t *src,
    uint32_t offset,
    uint32_t dst_size,
    uint32_t src_size,
    bool fill_with_null
) {
    uint32_t dst_words = (dst_size + 63)/64;
    uint32_t src_words = (src_size + 63)/64;
    if (offset == 0) {
        if (fill_with_null) {
            set_no_special(dst, dst_size);
            memcpy(dst+dst_words, src, dst_words*sizeof(uint64_t));
        } else {
            memcpy(dst, src, dst_words*sizeof(uint64_t));
        }
        if (dst_size % 64 != 0) {
            dst[dst_words*2-1] &= (((uint64_t)1) << (dst_size%64))-1;
        }
        return;
    }
    if (offset >= src_size) {
        if (fill_with_null) {
            memset(dst, 0, 2*dst_words*sizeof(uint64_t));
        } else {
            memset(dst, 0, dst_words*sizeof(uint64_t));
        }
        return;
    }

    // Fill valid bits.
    if (fill_with_null) {
        uint32_t num_x_bits = ((src_size-dst_size)>=offset) ? 0 : (offset-(src_size-dst_size));
        if (num_x_bits == 0) {
            set_no_special(dst, dst_size);
        } else {
            uint32_t num_valid_bits = dst_size - num_x_bits;
            memset(dst, 0xFF, (num_valid_bits / 64)*sizeof(uint64_t));
            if (num_valid_bits % 64 != 0) {
                dst[num_valid_bits / 64] = (((uint64_t)1) << (num_valid_bits%64))-1;
            }
            uint32_t o = (num_valid_bits / 64)+((uint32_t)(num_valid_bits % 64 != 0));
            memset(dst+o, 0, (dst_words-o)*sizeof(uint64_t));
        }
        tv_part_ll_slice(dst+dst_words, src, offset, dst_size, src_size, false);
    } else {
        tv_part_ll_slice(dst, src, offset, dst_size, src_size, false);
    }
}
static inline uint64_t tv_s_set(
    uint64_t dst,
    uint64_t src,
    uint32_t dst_size,
    uint32_t offset,
    uint32_t src_size
) {
    if (dst_size == src_size && offset == 0) {
        return src;
    }
    if (offset >= dst_size) {
        return dst;
    }

    uint32_t update_size = min(src_size, dst_size - offset);
    uint64_t mask = (((uint64_t)1) << update_size) - 1;
    src = src & mask;
    mask = mask << offset;
    uint64_t res = (src << offset) | (dst & ~mask);
    return res;
}

static inline void tv_l_set(
    uint64_t *dst,
    uint64_t *src,
    uint32_t dst_size,
    uint32_t offset,
    uint32_t src_size
) {
    uint32_t dst_words = (dst_size+63)/64;
    uint32_t src_words = (src_size+63)/64;

    if (dst_size == src_size && offset == 0) {
        memcpy(dst, src, dst_words*sizeof(uint64_t));
        return;
    }
    if (offset >= dst_size) {
        return;
    }

    // Truncate `src << offset` to fit in `dst`.
    src_size = min(src_size, dst_size-offset);

    uint32_t sh_words = offset / 64;
    uint32_t sh_offset = offset % 64;
    int i = 0;
    uint32_t bits_consumed = 0;

    // Least-Significant Word
    if (sh_offset > 0) {
        uint64_t mask = (src_size>=64) ? ~((uint64_t)0) : ((((uint64_t)1) << src_size) - 1);
        dst[sh_words] = ((src[0] & mask) << sh_offset) | (dst[sh_words] & ~(mask << sh_offset));
        i += 1;
        bits_consumed += 64 - sh_offset;
        while (bits_consumed + 64 <= src_size) {
            dst[i + sh_words] = (src[(bits_consumed / 64)] >> (64 - sh_offset))
                | src[(bits_consumed / 64) + 1] << sh_offset;
            bits_consumed += 64;
            i += 1;
        }
    } else {
        while (bits_consumed + 64 <= src_size) {
            dst[i + sh_words] = src[bits_consumed/64];
            bits_consumed += 64;
            i += 1;
        }
    }

    // Most-Significant Word
    if (bits_consumed < src_size) {
        uint32_t num_rem_bits = src_size - bits_consumed;
        uint64_t mask = (((uint64_t)1) << num_rem_bits) - 1;
        uint64_t new_src = src[bits_consumed/64] >> ((64 - sh_offset) % 64);
        bits_consumed += sh_offset;
        if (bits_consumed < src_size) {
            new_src |= src[bits_consumed/64] << sh_offset;
        }
        dst[i + sh_words] = (new_src & mask) | (dst[i + sh_words] & ~mask);
    }
}

NOINLINE int empty_active_event_queue(uint64_t *restrict heap, schedule_t *restrict schedule, uint64_t time, uint64_t *restrict listening, uint64_t *restrict last_active_time, cold_context_t *restrict cldctx) {
    event_t e;
    if (!event_vec_pop(&schedule->active_region, &e)) {
        return 0;
    }
    return (e.ptr)(e.state, heap, schedule, time, listening, last_active_time, cldctx);
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

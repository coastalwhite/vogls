use std::fmt::Write;
use std::{fmt, io};

use vogls_bits::BitsDataRef;
use vogls_codegen::{
    HeapBuilder, HeapOffset, HeapRef, bin_imm_args_need_conversion, insert_bb_phis,
};
use vogls_ir::dyn_format_string::{DynFormatArgument, DynFormatString};
use vogls_ir::{
    BasicBlockKey, BasicBlockTerminator, BinaryImmOp, Bits, ContextFormat, DisplayContext,
    GlobalContext, INTEGER_VSIZE, Instruction, LogicMode, Process, ProcessKey, ReadMem, ResizeOp,
    ShiftImmOp, SignalKey, TIME_VSIZE, UnaryOp, VariableKey, VectorSize,
};
use vogls_runtime::RtSignalKey;
use vogls_runtime::plugins::RuntimePluginState;
use vogls_utils::{IndexSet, VgHashMap, VgHashSet, saturating_rem};

pub mod runtime;

mod binary;
mod drive;
mod resize;
mod select;
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
        CExpr::Ident(value)
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
                        if bits.size() <= vogls_ir::Mode::FourValue.max_inline_size() {
                            write!(f, "((uint64_t)0x{:x})", (v << bits.size().get()) | mask)
                        } else {
                            write!(f, "(uint64_t[2]){{0x{:x}, 0x{:x}}}", mask, v)
                        }
                    }
                    BitsDataRef::SeparateTv(v) => {
                        let mask = mask(saturating_rem(bits.size().get(), 64));
                        let nwords = bits.size().get().saturating_sub(64).div_ceil(64);
                        write!(f, "(uint64_t[{}]){{", v.len() * 2)?;
                        for _ in 0..nwords {
                            write!(f, "0xffffffffffffffff,")?;
                        }
                        write!(f, "0x{mask:x}")?;
                        for v in v.iter() {
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

pub fn process_to_procedure_name(process: &Process, idx: usize, tri: usize) -> String {
    format!(
        "vogls_proc_{idx}_{}_TR{tri}",
        process.kind.into_static_str()
    )
}

pub struct Listener {
    pub offset: usize,
    pub process_idx: usize,
    pub process_key: ProcessKey,
    pub tri: usize,
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
        tri: usize,
    ) {
        for &signal in signals {
            self.map.entry(signal).or_default().push(Listener {
                offset: self.top,
                process_idx,
                process_key,
                tri,
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
    _heap_builder: &mut HeapBuilder,
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

    insert_bb_phis(
        &process.regions,
        gl,
        &mut bb_stack,
        &mut bb_seen,
        &mut bb_phis,
    );

    // @Performance. Amortize buffer.
    let mut buffer = Vec::<u8>::new();
    let mut temp_map = VgHashMap::<VariableKey, CVar>::default();

    let tr_indices = IndexSet::from_iter(process.regions.iter().copied());

    for (tri, _) in process.regions.iter().enumerate() {
        let procedure = process_to_procedure_name(process, process_idx, tri);
        writeln!(
            f,
            "NOINLINE __attribute__((preserve_none)) int {procedure}(uint64_t *restrict heap, schedule_t *restrict schedule, uint64_t time, uint64_t *restrict listening, uint64_t *restrict last_active_time, cold_context_t *restrict cldctx);",
        )?;
    }

    for (tri, &tr) in process.regions.iter().enumerate() {
        let procedure = process_to_procedure_name(process, process_idx, tri);

        buffer.clear();
        temp_map.clear();

        writeln!(
            f,
            "NOINLINE __attribute__((preserve_none)) int {procedure}(uint64_t *restrict heap, schedule_t *restrict schedule, uint64_t time, uint64_t *restrict listening, uint64_t *restrict last_active_time, cold_context_t *restrict cldctx) {{",
        )?;
        if lower_options.itrace {
            lower_dyn_format_str(
                f,
                &mut state_builder.dyn_fmt_strs,
                &DynFormatString::new(
                    format!("\n* PROC {procedure}, TR {tri}\n").into(),
                    [].into(),
                ),
                [].into(),
            )?;
        }

        let mut bb_ident = IndexSet::<BasicBlockKey>::new();
        let mut goto_target_set = IndexSet::<BasicBlockKey>::new();

        bb_stack.push(tr.entry());
        bb_ident.insert(tr.entry());
        while let Some(bb_key) = bb_stack.pop() {
            let bb = &gl.bbs[bb_key];
            bb.terminator.for_each_non_temporal_bb(|bb| {
                if bb_ident.insert(bb) {
                    bb_stack.push(bb);
                }
            });

            bb.try_for_each_dst_var(|v| {
                let t = CVar {
                    ident: CIdent::Numbered(temp_map.len() as u64),
                    ty: CType {
                        size: gl.vars.size(v),
                        mode: v.mode(),
                    },
                };

                write_cvar(f, t)?;
                temp_map.insert(v, t);
                io::Result::Ok(())
            })?;

            match bb.terminator {
                BasicBlockTerminator::Wait(..)
                | BasicBlockTerminator::VariableWait(..)
                | BasicBlockTerminator::WaitRegion(..)
                | BasicBlockTerminator::Watch(..) => {}
                BasicBlockTerminator::Jump(target) => _ = goto_target_set.insert(target),
                BasicBlockTerminator::Branch(_, truthy, falsy) => {
                    goto_target_set.extend([truthy, falsy])
                }
                BasicBlockTerminator::Halt => {}
            }
        }

        use std::io::Write;
        let mut display_context = DisplayContext::new(gl);
        if lower_options.itrace {
            for tr in &process.regions {
                display_context.prepare_process(tr.entry());
            }
        }

        bb_seen.clear();
        bb_seen.insert(tr.entry());
        bb_stack.push(tr.entry());
        while let Some(bb_key) = bb_stack.pop() {
            let bb = &gl.bbs[bb_key];
            bb.terminator.for_each_non_temporal_bb(|bb| {
                if bb_seen.insert(bb) {
                    bb_stack.push(bb);
                }
            });

            let ident = bb_ident.get_index(&bb_key).unwrap();

            if goto_target_set.contains(&bb_key) {
                writeln!(buffer, "L{ident}:")?;
            }

            for i in &bb.instrs {
                if lower_options.itrace {
                    writeln!(buffer, "{INDENT}// {}", i.display(&display_context))?;
                }
                match i {
                    I::Constant(dst, bits) => {
                        let t = temp_map[dst];
                        match t.ty.array_size() {
                            None => writeln!(
                                buffer,
                                "{INDENT}{} = {};",
                                t.ident,
                                CExpr::Bits(bits, dst.mode()),
                            )?,
                            Some(n) => writeln!(
                                buffer,
                                "{INDENT}memcpy({}, {}, {n}*sizeof(uint64_t));",
                                t.ident,
                                CExpr::Bits(bits, dst.mode()),
                            )?,
                        }
                    }
                    I::Unary(dst, op, src) => {
                        let t: CExpr = temp_map[src].into();
                        let dst_t = temp_map[dst];
                        use UnaryOp as O;
                        match op {
                            O::Neg => unary::cgc_negate(&mut buffer, dst_t.into(), t)?,
                            O::ReduceOr => unary::cgc_reduce_or(&mut buffer, dst_t.into(), t)?,
                            O::ReduceAnd => unary::cgc_reduce_and(&mut buffer, dst_t.into(), t)?,
                            O::ReduceXor => unary::cgc_reduce_xor(&mut buffer, dst_t.into(), t)?,
                            O::LeadingZeros => todo!(),
                            O::TvToFv => unary::cgc_tv_to_fv(&mut buffer, dst_t, t)?,
                            O::FvToTv => unary::cgc_fv_to_tv(&mut buffer, dst_t, t)?,
                        }
                    }
                    I::Resize(dst, op, src) => {
                        let t: CExpr = temp_map[src].into();
                        let dst_t: CExpr = temp_map[dst].into();
                        use ResizeOp as O;
                        match op {
                            O::Truncate => resize::cgc_truncate(&mut buffer, dst_t, t.into())?,
                            O::ZeroExtend => resize::cgc_zero_extend(&mut buffer, dst_t, t)?,
                            O::SignExtend => resize::cgc_sign_extend(&mut buffer, dst_t, t)?,
                        }
                    }
                    I::BinaryImm(dst, op, src, imm) => {
                        let mimm = if imm.contains_special() {
                            LogicMode::FourValue
                        } else {
                            LogicMode::TwoValue
                        };

                        let (mtgt, _, conv_imm) =
                            bin_imm_args_need_conversion(*op, dst.mode(), src.mode(), mimm);

                        let src_t: CExpr = temp_map[src].into();
                        let imm_tgt_mode = if conv_imm { mtgt } else { mimm };
                        let imm = CExpr::Bits(imm, imm_tgt_mode);
                        let dst_t = temp_map[dst];

                        use BinaryImmOp as O;
                        match op {
                            O::And => binary::cgc_bin_and(&mut buffer, dst_t, src_t.into(), imm)?,
                            O::Or => binary::cgc_bin_or(&mut buffer, dst_t, src_t.into(), imm)?,
                            O::Xor => binary::cgc_bin_xor(&mut buffer, dst_t, src_t.into(), imm)?,
                            O::Add => binary::cgc_bin_add(&mut buffer, dst_t, src_t.into(), imm)?,
                            O::Sub => binary::cgc_bin_sub(&mut buffer, dst_t, src_t.into(), imm)?,
                            O::Power => binary::cgc_bin_pow(&mut buffer, dst_t, src_t.into(), imm)?,
                            O::Multiply => {
                                binary::cgc_bin_mul(&mut buffer, dst_t, src_t.into(), imm)?
                            }
                            O::Divide => {
                                binary::cgc_bin_div(&mut buffer, dst_t, src_t.into(), imm)?
                            }
                            O::Modulus => {
                                binary::cgc_bin_mod(&mut buffer, dst_t, src_t.into(), imm)?
                            }
                            O::RevSub => {
                                binary::cgc_bin_sub(&mut buffer, dst_t, imm, src_t.into())?
                            }
                            O::RevPower => {
                                binary::cgc_bin_pow(&mut buffer, dst_t, imm, src_t.into())?
                            }
                            O::RevDivideX => {
                                binary::cgc_bin_div(&mut buffer, dst_t, imm, src_t.into())?
                            }
                            O::RevDivide0 => todo!(),
                            O::RevModulusX => {
                                binary::cgc_bin_mod(&mut buffer, dst_t, imm, src_t.into())?
                            }
                            O::RevModulus0 => todo!(),
                            O::UnsignedLessEqual => {
                                binary::cgc_bin_ule(&mut buffer, dst_t.ident, src_t.into(), imm)?
                            }
                            O::UnsignedGreaterEqual => {
                                binary::cgc_bin_ule(&mut buffer, dst_t.ident, imm, src_t.into())?
                            }
                            O::ConcatRight => {
                                binary::cgc_concat(&mut buffer, dst_t, src_t.into(), imm)?
                            }
                            O::ConcatLeft => {
                                binary::cgc_concat(&mut buffer, dst_t, imm, src_t.into())?
                            }
                            O::Min => binary::cgc_bin_min(&mut buffer, dst_t, src_t.into(), imm)?,
                            O::Max => binary::cgc_bin_max(&mut buffer, dst_t, src_t.into(), imm)?,
                            O::CaseEquality => {
                                binary::cgc_case_eq(&mut buffer, dst_t.ident, src_t.into(), imm)?
                            }
                            O::BitwiseCaseEquality => todo!(),
                        }
                    }
                    I::SliceImm(dst, src, offset) => {
                        let src_t: CExpr = temp_map[src].into();
                        let imm =
                            CExpr::Bits(&Bits::from_u64(INTEGER_VSIZE, *offset as u64), dst.mode());
                        let dst_t = temp_map[dst];
                        slice::slice_with(&mut buffer, dst_t, src_t.into(), imm, false)?;
                    }
                    I::ShiftImm(dst, op, src, offset) => {
                        let src_t: CExpr = temp_map[src].into();
                        let imm =
                            CExpr::Bits(&Bits::from_u64(INTEGER_VSIZE, *offset as u64), dst.mode());

                        let dst_t = temp_map[dst];
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
                    }
                    I::Select(dst, cond, truthy, falsy) => {
                        let cond_t: CExpr = temp_map[cond].into();
                        let truthy_t: CExpr = temp_map[truthy].into();
                        let falsy_t: CExpr = temp_map[falsy].into();

                        let dst_t = temp_map[dst];
                        select::cgc_select(&mut buffer, dst_t, cond_t, truthy_t, falsy_t)?;
                    }
                    I::Slice(dst, lhs, rhs) => {
                        let lhs_t: CExpr = temp_map[lhs].into();
                        let rhs_t: CExpr = temp_map[rhs].into();
                        let dst_t = temp_map[dst];
                        slice::slice(&mut buffer, dst_t, lhs_t.into(), rhs_t.into())?;
                    }
                    I::Binary(dst, op, lhs, rhs) => {
                        let lhs_t: CExpr = temp_map[lhs].into();
                        let rhs_t: CExpr = temp_map[rhs].into();
                        let dst_t = temp_map[dst];

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
                            O::DivideX => {
                                binary::cgc_bin_div(&mut buffer, dst_t, lhs_t.into(), rhs_t.into())?
                            }
                            O::Divide0 => todo!(),
                            O::ModulusX => {
                                binary::cgc_bin_mod(&mut buffer, dst_t, lhs_t.into(), rhs_t.into())?
                            }
                            O::Modulus0 => todo!(),
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
                            O::CopyZ => binary::cgc_copy_y(&mut buffer, dst_t, lhs_t, rhs_t)?,
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
                            O::Negedge => {
                                binary::cgc_negedge(&mut buffer, dst_t.ident, lhs_t, rhs_t)?
                            }
                        }
                    }
                    I::Intrinsic(dst, op, items) => match op.as_ref() {
                        vogls_ir::IntrinsicOp::Time => {
                            let t = temp_map[dst];
                            writeln!(buffer, "{INDENT}{} = time;", t.ident)?;
                        }
                        vogls_ir::IntrinsicOp::Finish => {
                            writeln!(buffer, "{INDENT}return 1;")?;
                        }
                        vogls_ir::IntrinsicOp::Random => todo!(),
                        vogls_ir::IntrinsicOp::Display(dyn_format_string) => {
                            // @Performance: scratchpad this.
                            let args = items
                                .iter()
                                .map(|i| Ok(temp_map[i].into()))
                                .collect::<io::Result<Vec<CExpr>>>()?;
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
                            let t = temp_map[&fst];
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
                                .map(|i| Ok(temp_map[i].into()))
                                .collect::<io::Result<Vec<CExpr>>>()?;
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
                        let t = temp_map[dst];
                        let rt_key = io_signals[signal];
                        let idx = lupdt_indexes[&rt_key];
                        writeln!(buffer, "{INDENT}{} = last_active_time[{idx}];", t.ident)?;
                    }
                    I::Probe(dst, signal, offset) => {
                        let signal_mode = gl.signals[*signal].mode;
                        let signal_ref = signals[io_signals[signal].as_usize()];

                        let t = temp_map[dst];
                        let dst_size = gl.vars.size(*dst);
                        let src_size = gl.signals[*signal].size;

                        if dst_size == src_size && *offset == 0 {
                            load(&mut buffer, signal_ref.offset, t)?;
                        } else {
                            let signal = CExpr::HeapRef(signal_ref, signal_mode);
                            let offset_c =
                                CExpr::Bits(&Bits::new_u32(*offset), LogicMode::TwoValue);
                            if *offset > 0 {
                                slice::slice_with(&mut buffer, t, signal, offset_c, false)?;
                            } else {
                                resize::cgc_truncate(&mut buffer, t.into(), signal)?;
                            }
                        }
                    }
                    I::ProbeSlice(dst, signal, offset) => {
                        let signal_mode = gl.signals[*signal].mode;
                        let signal_ref = signals[io_signals[signal].as_usize()];
                        assert_eq!(dst.mode(), LogicMode::FourValue);

                        let t = temp_map[dst];
                        let offset_t = temp_map[offset];

                        slice::slice(
                            &mut buffer,
                            t,
                            CExpr::HeapRef(signal_ref, signal_mode),
                            offset_t.into(),
                        )?;
                    }
                    I::Drive(signal, src, partial) => {
                        let t: CExpr = temp_map[src].into();
                        let zero = Bits::new_u32(0);
                        let partial = match partial {
                            None if gl.signals[*signal].size != t.ty().size => {
                                Some(CExpr::Bits(&zero, LogicMode::TwoValue))
                            }
                            None => None,
                            Some((offset, _)) => {
                                let offset_t = temp_map[offset];
                                Some(offset_t.into())
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
                        let var = temp_map[&dst];
                        content.push_str(&dst.display(&display_context).to_string());
                        content.push_str(" = ");
                        arg_offsets.push((content.len(), DynFormatArgument::default()));
                        content.push_str("; ");
                        args.push(CExpr::from(var));
                    }
                    i.for_each_src(|src| {
                        let var = temp_map[&src];
                        content.push_str(&src.display(&display_context).to_string());
                        content.push_str(" = ");
                        arg_offsets.push((content.len(), DynFormatArgument::default()));
                        content.push_str("; ");
                        args.push(CExpr::from(var));
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
                    let src_size = gl.vars.size(*src);
                    let dst_size = gl.vars.size(*dst);
                    assert_eq!(src_size, dst_size);
                    let src_t: CExpr = temp_map[src].into();
                    let dst_t: CExpr = temp_map[dst].into();

                    if lower_options.itrace {
                        writeln!(&mut buffer, "{INDENT}// Phi({dst:?}, {src:?});")?;
                    }
                    let d = dst_t;
                    let s = src_t;
                    match dst_t.ty().array_size() {
                        None => writeln!(buffer, "{INDENT}{d} = {s};")?,
                        Some(num_words) => writeln!(
                            buffer,
                            "{INDENT}memcpy({d}, {s}, {num_words}*sizeof(uint64_t));"
                        )?,
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
{INDENT}[[clang::musttail]] return (e)(heap, schedule, time, listening, last_active_time, cldctx);
{INDENT}}}"#
                )
            }

            match &bb.terminator {
                BasicBlockTerminator::Wait(tr, time) => {
                    let time = time.0;
                    let tgt_tri = tr_indices.get_index(tr).unwrap();
                    let tgt = process_to_procedure_name(process, process_idx, tgt_tri);
                    if time == 0 {
                        writeln!(
                            buffer,
                            r#"{INDENT}[[clang::musttail]] return {tgt}(heap, schedule, time, listening, last_active_time, cldctx);"#,
                        )?;
                    } else {
                        writeln!(
                            buffer,
                            "{INDENT}schedule_future_event(schedule, time + {time}, &{tgt});"
                        )?;
                        next_event_or_return_0(&mut buffer)?;
                    }
                }
                BasicBlockTerminator::VariableWait(tr, time) => {
                    let t = temp_map[time];
                    let tgt_tri = tr_indices.get_index(tr).unwrap();
                    let tgt = process_to_procedure_name(process, process_idx, tgt_tri);
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
                    match time.mode() {
                        LogicMode::TwoValue => writeln!(buffer, "{INDENT}{s0} = {t};")?,
                        LogicMode::FourValue => {
                            writeln!(buffer, "{INDENT}{s0} = (~{t}[0] != 0) ? 0 : {t}[1];")?;
                        }
                    }
                    writeln!(
                        buffer,
                        r#"{INDENT}if ({s0} == 0) {{
{INDENT}{INDENT}[[clang::musttail]] return {tgt}(heap, schedule, time, listening, last_active_time, cldctx);
{INDENT}}} else {{
{INDENT}{INDENT}schedule_future_event(schedule, time + {s0}, &{tgt});"#,
                    )?;
                    next_event_or_return_0(&mut buffer)?;
                    writeln!(buffer, "{INDENT}}}}}")?;
                }
                BasicBlockTerminator::WaitRegion(tr, region) => {
                    let tgt_tri = tr_indices.get_index(tr).unwrap();
                    let tgt = process_to_procedure_name(process, process_idx, tgt_tri);
                    writeln!(
                        buffer,
                        "{INDENT}event_vec_push(&schedule->regions[{region}], &{tgt});",
                    )?;
                    next_event_or_return_0(&mut buffer)?;
                }
                BasicBlockTerminator::Watch(tr, items) => {
                    let tgt_tri = tr_indices.get_index(tr).unwrap();
                    let offset = listener_builder.top;
                    writeln!(
                        buffer,
                        "{INDENT}listening[{}] |= 0x{:x};",
                        offset / 64,
                        1u64 << (offset % 64)
                    )?;
                    listener_builder.insert_signals(items, process_idx, process_key, tgt_tri);
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

                    let t = temp_map[condition];

                    let t = t.ident;
                    match condition.mode() {
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
    }

    Ok(())
}

fn lower_dyn_format_str(
    f: &mut impl io::Write,
    dyn_fmt_strs: &mut IndexSet<DynFormatString>,
    dyn_format_string: &DynFormatString,
    args: Vec<CExpr>,
) -> io::Result<()> {
    write!(f, r#"{INDENT}{{ "#)?;
    for (i, arg) in args.iter().enumerate() {
        if !arg.ty().is_array() {
            write!(f, "uint64_t arg{i} = (uint64_t){}; ", arg)?;
        }
    }
    write!(
        f,
        r#"bits_ref_t args[{num_args}] = {{ "#,
        num_args = args.len()
    )?;
    for (i, arg) in args.iter().enumerate() {
        if arg.ty().is_array() {
            write!(
                f,
                r#"(bits_ref_t){{ .size={}, .mode={}, .ptr={} }}, "#,
                arg.ty().size,
                u8::from(arg.ty().mode == LogicMode::FourValue),
                arg
            )?;
        } else {
            write!(
                f,
                r#"(bits_ref_t){{ .size={}, .mode={}, .ptr=&arg{i} }}, "#,
                arg.ty().size,
                u8::from(arg.ty().mode == LogicMode::FourValue)
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

fn store(f: &mut impl io::Write, heap_offset: HeapOffset, t: CExpr) -> io::Result<()> {
    match t.ty().array_size() {
        None => {
            let word = heap_offset.bit_offset / 64;
            let shift = heap_offset.bit_offset % 64;
            let num_bits = t.ty().num_bits();
            if num_bits == 64 {
                writeln!(f, "{INDENT}heap[{word}] = {t};")
            } else {
                let mask = !(((1u64 << num_bits) - 1) << shift);
                writeln!(
                    f,
                    "{INDENT}heap[{word}] = (heap[{word}] & 0x{mask:x}) | ((uint64_t){t} << {shift});",
                )
            }
        }
        Some(arr_size) => {
            let word = heap_offset.bit_offset / 64;
            writeln!(
                f,
                "{INDENT}memcpy(heap + {word}, &{t}, {arr_size} * sizeof(uint64_t));",
            )
        }
    }
}

pub fn lower_process_array(f: &mut impl io::Write, gl: &GlobalContext) -> io::Result<()> {
    write!(f, "event_t PROCS[{}] = {{", gl.processes.len(),)?;
    for (i, process) in gl.processes.values().enumerate() {
        writeln!(f, "{}, ", process_to_procedure_name(process, i, 0))?;
    }
    writeln!(f, "}};")
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

    if matches!(gl.logic_mode, LogicMode::TwoValue) {
        writeln!(
            f,
            "{INDENT}cldctx->fst_poke[{}] |= ((uint64_t)1) << {};",
            rt_key.as_u64() / 64,
            rt_key.as_u64() % 64,
        )?;
    }

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
                "{INDENT}{INDENT}event_vec_push(&schedule->active_region, &{});",
                process_to_procedure_name(
                    &gl.processes[listener.process_key],
                    listener.process_idx,
                    listener.tri,
                ),
            )?;
            writeln!(f, "{INDENT}}}",)?;
        }
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

    uint64_t *fst_poke;

    uint64_t icount;

    void *stdout;
    void *stderr;
} cold_context_t;

typedef
    int __attribute__((preserve_none))
        (*event_t)(uint64_t*, struct schedule*, uint64_t, uint64_t*, uint64_t*, cold_context_t*);

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
    return (e)(heap, schedule, time, listening, last_active_time, cldctx);
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
    event = NULL;
    return false;
  }
  *event = v->ptr[v->length - 1];
  v->length -= 1;
  return true;
}
"#)
}

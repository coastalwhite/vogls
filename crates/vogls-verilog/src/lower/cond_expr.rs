//! Describes expressions used for conditional delays. These expressions are optimizes to:
//! - Be especially effective for scalar comparisons.
//! - Have a relatively small memory footprint.
//! - Be mostly flat, i.e. less pointer chasing
//!
//! The main structure is [`CondExpr`].

use std::cmp::Ordering;
use std::hash::BuildHasher;

use foldhash::fast::RandomState;
use hashbrown::HashTable;
use vogls_ir::bits::arena::{BitsArena, BitsArenaPtr};
use vogls_ir::{
    BasicBlockBuilder, Bits, GlobalContext, SCALAR_VSIZE, SignalKey, SignalSlice, VariableKey,
    VectorSize,
};
use vogls_utils::{NonMaxU64, Table, TableKey, VgHashMap, new_table_key};

new_table_key! { pub struct TerminalKey; }

macro_rules! kinds {
    (
        $(
            $(#[$attr:meta])*
            $kind:ident
        ),+ $(,)?
    ) => {
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        #[repr(u8)]
        pub enum CondExprKind {
            $(
            $(#[$attr])*
            $kind,
            )+
        }

        impl CondExprKind {
            #[allow(path_statements)]
            const LAST_KIND: Self = {
                Self::ScalarConstant
                $( ; Self::$kind )+
            };
        }

        impl TryFrom<u8> for CondExprKind {
            type Error = ();
            #[inline(always)]
            fn try_from(value: u8) -> Result<Self, Self::Error> {
                if value > Self::LAST_KIND as u8 {
                    std::hint::cold_path();
                    return Err(());
                }

                // This does not properly optimize above a certain number of enum-variants.
                //
                // SAFETY:
                // - Niche checked before.
                let kind = unsafe { std::mem::transmute::<u8, CondExprKind>(value) };
                Ok(kind)
            }
        }
    };
}

kinds! {
    /// Data encodes 0 / 1.
    ScalarConstant,
    /// Data encodes the raw `BitsArenaPtr` of the constant in `constants`.
    Constant,
    /// Data encodes the `TerminalKey` of a slice of a signal.
    Terminal,

    /// Data encodes `index` into `path_conds`.
    ArithmeticNeg,
    BitwiseNot,
    LogicalNot,
    ToLogical,
    /// `a === 1'b0`
    CaseEq0,
    /// `a === 1'b1`
    CaseEq1,

    /// `a == 1'b1`, which is `a` with `z` as `x`.
    Eq1,

    ReduceAnd,
    ReduceOr,
    ReduceXor,
    ReduceNand,
    ReduceNor,
    ReduceXnor,

    // Data encodes `index`, `index + 1` into `path_conds`.
    And,
    Or,
    Xor,
    Xnor,
    Eq,
    Neq,
    CaseEq,
    CaseNeq,
    BitwiseEquiv,
    Concat,
    /// `index` extended to the width at `index + 1`, which is an entry with the illegal kind
    /// `0xFF` (see [`CondExpr::width_entry`]).
    ZeroExtend,
    SignExtend,

    // Data encodes `index`, `index + 1`, `index + 2` into `path_conds`: `index ? index + 1 : index + 2`.
    Mux,
}

/// An expression of a condition encoded as `(CondExprKind) | (data << 8)`. Its width follows from
/// its terminals and constants (see [`CondExprs::width`]).
///
/// Children are stored in a [`CondExprs`] instance.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CondExpr(NonMaxU64);

impl CondExpr {
    fn new(kind: CondExprKind, data: u64) -> Self {
        assert!(
            data < 1u64 << 56,
            "the data of a path condition fits in 56 bits"
        );
        Self(NonMaxU64::new(kind as u64 | data << 8).expect("the kind is not 0xFF"))
    }

    pub fn kind(self) -> CondExprKind {
        let repr = (self.0.get() & 0xFF) as u8;
        debug_assert!(CondExprKind::try_from(repr).is_ok());

        let repr = repr.min(CondExprKind::LAST_KIND as u8);
        CondExprKind::try_from(repr).unwrap()
    }

    fn data(self) -> u64 {
        self.0.get() >> 8
    }

    fn index(self) -> usize {
        self.data() as usize
    }

    /// The constant of a [`CondExprKind::Constant`] in [`CondExprs`].
    fn as_constant(self) -> BitsArenaPtr {
        debug_assert_eq!(self.kind(), CondExprKind::Constant);
        BitsArenaPtr::from_raw(self.data())
    }

    pub fn constant(value: bool) -> Self {
        Self::new(CondExprKind::ScalarConstant, u64::from(value))
    }

    /// An operand entry that holds `width`, with the illegal kind `0xFF`. It is not an expression,
    /// so [`CondExpr::kind`] must not be called on it.
    fn width_entry(width: VectorSize) -> Self {
        Self(NonMaxU64::new(0xFF | u64::from(width.get()) << 8).expect("a width is not u64::MAX"))
    }
}

/// The storage of the operands, terminals and constants of [`CondExpr`]s.
///
/// Terminals and constants are interned, so that equal expressions are built from equal leaves
/// and can be compared structurally (see [`CondExprs::exprs_cmp`]).
#[derive(Default, Clone)]
pub struct CondExprs {
    path_conds: Vec<CondExpr>,
    terminals: Table<TerminalKey, (SignalKey, SignalSlice)>,
    terminal_lut: VgHashMap<(SignalKey, SignalSlice), TerminalKey>,
    constants: BitsArena,
    /// The constants in `constants` by the hash of their value, to intern them.
    constant_lut: HashTable<BitsArenaPtr>,
    constant_hasher: RandomState,
}

/// The result of narrowing the operands of a comparison (see [`CondExprBuilder::narrow`]).
enum Narrowed {
    /// The operands stay as they are.
    No,
    /// The operands without the extension.
    To(CondExpr, CondExpr),
    /// A bit that the extension adds differs from the constant, so they are never equal.
    Mismatch,
}

pub struct CondExprBuilder<'a> {
    conds: &'a mut CondExprs,
    scratch: Vec<(CondExpr, CondExpr)>,
}

macro_rules! symmetric_bitwise_op {
    ($f:ident, $op:ident) => {
        pub fn $f(&mut self, a: CondExpr, b: CondExpr) -> CondExpr {
            let children = if self.conds.exprs_cmp(&mut self.scratch, a, b) == Ordering::Greater {
                [b, a]
            } else {
                [a, b]
            };
            self.push(CondExprKind::$op, &children)
        }
    };
}

impl<'a> CondExprBuilder<'a> {
    #[inline(always)]
    fn push(&mut self, kind: CondExprKind, operands: &[CondExpr]) -> CondExpr {
        let index = self.conds.path_conds.len() as u64;
        self.conds.path_conds.extend_from_slice(operands);
        CondExpr::new(kind, index)
    }

    pub fn arithmetic_neg(&mut self, child: CondExpr) -> CondExpr {
        self.push(CondExprKind::ArithmeticNeg, &[child])
    }
    pub fn bitwise_not(&mut self, child: CondExpr) -> CondExpr {
        match child.kind() {
            CondExprKind::ScalarConstant => CondExpr::constant(child.data() == 0),
            CondExprKind::BitwiseNot => self.conds.path_conds[child.index()],
            _ => self.push(CondExprKind::BitwiseNot, &[child]),
        }
    }
    /// `!child`, which is `~child` for a single bit.
    pub fn logical_not(&mut self, child: CondExpr) -> CondExpr {
        if self.conds.width(child) == 1 {
            return self.bitwise_not(child);
        }
        self.push(CondExprKind::LogicalNot, &[child])
    }
    /// Whether `child` is non-zero, which is `child == 1'b1` for a single bit.
    pub fn to_logical(&mut self, child: CondExpr) -> CondExpr {
        if self.conds.width(child) == 1 {
            return self.eq1(child);
        }
        self.push(CondExprKind::ToLogical, &[child])
    }
    pub fn case_eq0(&mut self, child: CondExpr) -> CondExpr {
        self.push(CondExprKind::CaseEq0, &[child])
    }
    pub fn case_eq1(&mut self, child: CondExpr) -> CondExpr {
        self.push(CondExprKind::CaseEq1, &[child])
    }
    /// `child == 1'b1`, which is `child` itself if it cannot be `z`.
    pub fn eq1(&mut self, child: CondExpr) -> CondExpr {
        if !child.kind().can_be_z() {
            return child;
        }
        self.push(CondExprKind::Eq1, &[child])
    }
    pub fn reduce_and(&mut self, child: CondExpr) -> CondExpr {
        self.push(CondExprKind::ReduceAnd, &[child])
    }
    pub fn reduce_or(&mut self, child: CondExpr) -> CondExpr {
        self.push(CondExprKind::ReduceOr, &[child])
    }
    pub fn reduce_xor(&mut self, child: CondExpr) -> CondExpr {
        self.push(CondExprKind::ReduceXor, &[child])
    }
    pub fn reduce_nand(&mut self, child: CondExpr) -> CondExpr {
        self.push(CondExprKind::ReduceNand, &[child])
    }
    pub fn reduce_nor(&mut self, child: CondExpr) -> CondExpr {
        self.push(CondExprKind::ReduceNor, &[child])
    }
    pub fn reduce_xnor(&mut self, child: CondExpr) -> CondExpr {
        self.push(CondExprKind::ReduceXnor, &[child])
    }

    symmetric_bitwise_op!(and, And);
    symmetric_bitwise_op!(or, Or);
    symmetric_bitwise_op!(xor, Xor);
    symmetric_bitwise_op!(xnor, Xnor);
    symmetric_bitwise_op!(raw_eq, Eq);
    symmetric_bitwise_op!(raw_neq, Neq);
    symmetric_bitwise_op!(raw_case_eq, CaseEq);
    symmetric_bitwise_op!(raw_case_neq, CaseNeq);
    symmetric_bitwise_op!(bitwise_equiv, BitwiseEquiv);

    /// `a === b`. Against a single-bit constant, this is [`CondExprKind::CaseEq0`] or
    /// [`CondExprKind::CaseEq1`], which are false for `x` and `z`.
    pub fn case_eq(&mut self, a: CondExpr, b: CondExpr) -> CondExpr {
        match self.narrow(a, b) {
            Narrowed::No => {}
            Narrowed::To(a, b) => return self.case_eq(a, b),
            Narrowed::Mismatch => return CondExpr::constant(false),
        }
        let (value, other) = match (a.kind(), b.kind()) {
            (CondExprKind::ScalarConstant, _) => (a.data(), b),
            (_, CondExprKind::ScalarConstant) => (b.data(), a),
            _ => return self.raw_case_eq(a, b),
        };
        if value != 0 {
            self.case_eq1(other)
        } else {
            self.case_eq0(other)
        }
    }
    pub fn eq(&mut self, a: CondExpr, b: CondExpr) -> CondExpr {
        match self.narrow(a, b) {
            Narrowed::No => {}
            Narrowed::To(a, b) => return self.eq(a, b),
            Narrowed::Mismatch => return CondExpr::constant(false),
        }
        let (value, other) = match (a.kind(), b.kind()) {
            (CondExprKind::ScalarConstant, _) => (a.data(), b),
            (_, CondExprKind::ScalarConstant) => (b.data(), a),
            _ => return self.raw_eq(a, b),
        };
        if value != 0 {
            self.eq1(other)
        } else {
            self.bitwise_not(other)
        }
    }
    pub fn neq(&mut self, a: CondExpr, b: CondExpr) -> CondExpr {
        match self.narrow(a, b) {
            Narrowed::No => self.raw_neq(a, b),
            Narrowed::To(a, b) => self.neq(a, b),
            Narrowed::Mismatch => CondExpr::constant(true),
        }
    }
    pub fn case_neq(&mut self, a: CondExpr, b: CondExpr) -> CondExpr {
        match self.narrow(a, b) {
            Narrowed::No => self.raw_case_neq(a, b),
            Narrowed::To(a, b) => self.case_neq(a, b),
            Narrowed::Mismatch => CondExpr::constant(true),
        }
    }

    /// The operands of a comparison of `a` and `b` without the extension of one of them, if the
    /// other is a constant that fits in the width before the extension.
    fn narrow(&mut self, a: CondExpr, b: CondExpr) -> Narrowed {
        let (extended, constant) = match (a.kind(), b.kind()) {
            (CondExprKind::ZeroExtend, _) => (a, b),
            (_, CondExprKind::ZeroExtend) => (b, a),
            _ => return Narrowed::No,
        };
        let Some(value) = self.conds.constant_value(constant) else {
            return Narrowed::No;
        };
        let inner = self.conds.path_conds[extended.index()];
        let width = VectorSize::new(self.conds.width(inner)).expect("a width is not zero");
        let upper = VectorSize::new(value.size().get() - width.get()).expect("it is extended");

        // The bits that the extension adds are zero.
        let upper = value.slicez(width.get(), upper);
        if upper.contains_special() {
            return Narrowed::No;
        }
        if !upper.is_equal_to_zero() {
            return Narrowed::Mismatch;
        }
        let value = self.constant(&value.truncate(width));
        Narrowed::To(inner, value)
    }

    /// `child` zero-extended to `width`.
    pub fn zero_extend(&mut self, child: CondExpr, width: VectorSize) -> CondExpr {
        self.extend(CondExprKind::ZeroExtend, child, width)
    }
    /// `child` sign-extended to `width`.
    pub fn sign_extend(&mut self, child: CondExpr, width: VectorSize) -> CondExpr {
        self.extend(CondExprKind::SignExtend, child, width)
    }

    fn extend(&mut self, kind: CondExprKind, child: CondExpr, width: VectorSize) -> CondExpr {
        let child_width = self.conds.width(child);
        debug_assert!(child_width <= width.get());
        if child_width == width.get() {
            return child;
        }
        if let Some(value) = self.conds.constant_value(child) {
            let value = match kind {
                CondExprKind::ZeroExtend => value.zero_extend(width),
                _ => value.sign_extend(width),
            };
            return self.constant(&value);
        }
        self.push(kind, &[child, CondExpr::width_entry(width)])
    }

    pub fn concat(&mut self, a: CondExpr, b: CondExpr) -> CondExpr {
        self.push(CondExprKind::Concat, &[a, b])
    }

    /// `a && b`
    pub fn logical_and(&mut self, a: CondExpr, b: CondExpr) -> CondExpr {
        let (a, b) = (self.to_logical(a), self.to_logical(b));
        self.and(a, b)
    }
    /// `a || b`
    pub fn logical_or(&mut self, a: CondExpr, b: CondExpr) -> CondExpr {
        let (a, b) = (self.to_logical(a), self.to_logical(b));
        self.or(a, b)
    }

    /// `cond ? a : b`
    pub fn mux(&mut self, cond: CondExpr, a: CondExpr, b: CondExpr) -> CondExpr {
        let cond = self.to_logical(cond);
        self.push(CondExprKind::Mux, &[cond, a, b])
    }

    pub fn take_scratch(&mut self) -> Vec<(CondExpr, CondExpr)> {
        std::mem::take(&mut self.scratch)
    }

    /// The constant `value`, which is interned unless it is a single `0` or `1`.
    pub fn constant(&mut self, value: &Bits) -> CondExpr {
        if value.size() == SCALAR_VSIZE && !value.contains_special() {
            return CondExpr::constant(!value.is_equal_to_zero());
        }

        let CondExprs {
            constants,
            constant_lut,
            constant_hasher,
            ..
        } = &mut *self.conds;
        let hash = constant_hasher.hash_one(value);
        let ptr = match constant_lut.find(hash, |&ptr| constants.get(ptr) == *value) {
            Some(&ptr) => ptr,
            None => {
                let ptr = constants.push(value);
                constant_lut.insert_unique(hash, ptr, |&ptr| {
                    constant_hasher.hash_one(constants.get(ptr))
                });
                ptr
            }
        };
        CondExpr::new(CondExprKind::Constant, ptr.into_raw())
    }

    pub fn terminal(&mut self, signal: SignalKey, slice: SignalSlice) -> CondExpr {
        let key = self
            .conds
            .terminal_lut
            .entry((signal, slice))
            .or_insert_with(|| self.conds.terminals.insert((signal, slice)));
        CondExpr::new(CondExprKind::Terminal, key.get() as u64)
    }
}

impl CondExprs {
    /// The width of the value of `expr`.
    pub fn width(&self, expr: CondExpr) -> u32 {
        use CondExprKind as K;
        let child = |i: usize| self.path_conds[expr.index() + i];
        match expr.kind() {
            K::ScalarConstant => 1,
            K::Constant => self.constants.bit_length(expr.as_constant()).get(),
            K::Terminal => {
                let key = TerminalKey::from_usize(expr.index()).expect("a terminal is a valid key");
                self.terminals[key].1.width().get()
            }
            K::ArithmeticNeg | K::BitwiseNot => self.width(child(0)),
            K::LogicalNot
            | K::ToLogical
            | K::CaseEq0
            | K::CaseEq1
            | K::Eq1
            | K::ReduceAnd
            | K::ReduceOr
            | K::ReduceXor
            | K::ReduceNand
            | K::ReduceNor
            | K::ReduceXnor
            | K::Eq
            | K::Neq
            | K::CaseEq
            | K::CaseNeq => 1,
            K::And | K::Or | K::Xor | K::Xnor | K::BitwiseEquiv => {
                self.width(child(0)).max(self.width(child(1)))
            }
            K::Concat => self.width(child(0)) + self.width(child(1)),
            K::ZeroExtend | K::SignExtend => self.extension_width(expr),
            K::Mux => self.width(child(1)).max(self.width(child(2))),
        }
    }

    /// The width that the extension `expr` extends to, from its width entry.
    fn extension_width(&self, expr: CondExpr) -> u32 {
        debug_assert!(matches!(
            expr.kind(),
            CondExprKind::ZeroExtend | CondExprKind::SignExtend
        ));
        self.path_conds[expr.index() + 1].data() as u32
    }

    /// The value of `expr` if it is a constant.
    fn constant_value(&self, expr: CondExpr) -> Option<Bits> {
        match expr.kind() {
            CondExprKind::ScalarConstant => Some(Bits::from(expr.data() != 0)),
            CondExprKind::Constant => Some(self.constants.get(expr.as_constant())),
            _ => None,
        }
    }

    pub fn builder<'a>(&'a mut self) -> CondExprBuilder<'a> {
        CondExprBuilder {
            conds: self,
            scratch: Vec::new(),
        }
    }

    pub fn builder_with_scratch<'a>(
        &'a mut self,
        scratch: Vec<(CondExpr, CondExpr)>,
    ) -> CondExprBuilder<'a> {
        CondExprBuilder {
            conds: self,
            scratch,
        }
    }

    pub fn exprs_cmp(
        &self,
        scratch: &mut Vec<(CondExpr, CondExpr)>,
        lhs: CondExpr,
        rhs: CondExpr,
    ) -> Ordering {
        if lhs == rhs {
            return Ordering::Equal;
        }
        let lhs_kind = lhs.kind();
        let rhs_kind = rhs.kind();
        if lhs_kind != rhs_kind {
            return Ord::cmp(&(lhs_kind as u64), &(rhs_kind as u64));
        }
        if lhs_kind.is_leaf() {
            return Ord::cmp(&lhs.0, &rhs.0);
        }

        scratch.clear();
        scratch.push((lhs, rhs));

        while let Some((x, y)) = scratch.pop() {
            if x == y {
                continue;
            }

            let xkind = x.kind();
            let ykind = y.kind();
            if xkind != ykind {
                return Ord::cmp(&(xkind as u64), &(ykind as u64));
            }
            if xkind.is_leaf() {
                return Ord::cmp(&x.0, &y.0);
            }

            let xidx = x.index();
            let yidx = y.index();
            if matches!(xkind, CondExprKind::ZeroExtend | CondExprKind::SignExtend) {
                let (xw, yw) = (self.extension_width(x), self.extension_width(y));
                if xw != yw {
                    return Ord::cmp(&xw, &yw);
                }
            }
            scratch.extend(
                (0..xkind.num_children())
                    .map(|i| (self.path_conds[xidx + i], self.path_conds[yidx + i])),
            );
        }
        Ordering::Equal
    }
}

/// Scratch buffers for [`CondExprs::lower`].
#[derive(Default)]
pub struct CondExprLowerScratch {
    /// The expressions to lower, and whether their operands are lowered already.
    work: Vec<(CondExpr, bool)>,
    /// The lowered values of the operands.
    values: Vec<VariableKey>,
}

impl CondExprs {
    /// Lowers `expr` to IR in `builder`, with the same operations as `lower_expr`.
    pub fn lower(
        &self,
        gl: &mut GlobalContext,
        builder: &mut BasicBlockBuilder,
        scratch: &mut CondExprLowerScratch,
        expr: CondExpr,
    ) -> VariableKey {
        use CondExprKind as K;

        let CondExprLowerScratch { work, values } = scratch;
        work.clear();
        values.clear();
        work.push((expr, false));

        while let Some((expr, is_expanded)) = work.pop() {
            let kind = expr.kind();
            let num_children = kind.num_children();
            if !is_expanded && num_children != 0 {
                // The operands are lowered in order, so they end on `values` in order.
                work.push((expr, true));
                let index = expr.index();
                work.extend(
                    (0..num_children)
                        .rev()
                        .map(|i| (self.path_conds[index + i], false)),
                );
                continue;
            }

            let operands_start = values.len() - num_children;
            let operand = |i: usize| values[operands_start + i];
            let value = match kind {
                K::ScalarConstant => builder.constant(gl, Bits::from(expr.data() != 0)),
                K::Constant => builder.constant(gl, self.constants.get(expr.as_constant())),
                K::Terminal => {
                    let key =
                        TerminalKey::from_usize(expr.index()).expect("a terminal is a valid key");
                    let (signal, slice) = self.terminals[key];
                    builder.probe_slice_constant(gl, signal, slice.lsb(), slice.width())
                }

                K::ArithmeticNeg => {
                    let width = VectorSize::new(self.width(expr)).expect("a width is not zero");
                    builder.revminus_constant(gl, operand(0), Bits::new_zeroed(width))
                }
                K::BitwiseNot => builder.binary_not(gl, operand(0)),
                K::LogicalNot => builder.logical_neg(gl, operand(0)),
                K::ToLogical => builder.reduce_or(gl, operand(0)),
                K::CaseEq0 => builder.case_equals_constant(gl, operand(0), Bits::from(false)),
                K::CaseEq1 => builder.case_equals_constant(gl, operand(0), Bits::from(true)),
                K::Eq1 => {
                    let one = builder.constant(gl, Bits::from(true));
                    builder.equals(gl, operand(0), one)
                }
                K::ReduceAnd => builder.reduce_and(gl, operand(0)),
                K::ReduceOr => builder.reduce_or(gl, operand(0)),
                K::ReduceXor => builder.reduce_xor(gl, operand(0)),
                K::ReduceNand => builder.reduce_nand(gl, operand(0)),
                K::ReduceNor => builder.reduce_nor(gl, operand(0)),
                K::ReduceXnor => builder.reduce_xnor(gl, operand(0)),
                K::ZeroExtend | K::SignExtend => {
                    let width = VectorSize::new(self.extension_width(expr))
                        .expect("a width is not zero");
                    if kind == K::ZeroExtend {
                        builder.zero_extend(gl, operand(0), width)
                    } else {
                        builder.sign_extend(gl, operand(0), width)
                    }
                }

                K::And => builder.and(gl, operand(0), operand(1)),
                K::Or => builder.or(gl, operand(0), operand(1)),
                K::Xor => builder.xor(gl, operand(0), operand(1)),
                K::Xnor | K::BitwiseEquiv => builder.xnor(gl, operand(0), operand(1)),
                K::Eq => builder.equals(gl, operand(0), operand(1)),
                K::Neq => builder.not_equals(gl, operand(0), operand(1)),
                K::CaseEq => builder.case_equals(gl, operand(0), operand(1)),
                K::CaseNeq => builder.not_case_equals(gl, operand(0), operand(1)),
                // The first operand is the most significant one.
                K::Concat => builder.concat(gl, operand(0), operand(1)),

                // An `x` or `z` condition merges both operands, as in `lower_expr`.
                K::Mux => builder.select_merge(gl, operand(0), operand(1), operand(2)),
            };
            values.truncate(operands_start);
            values.push(value);
        }

        assert_eq!(values.len(), 1, "an expression lowers to one value");
        values.pop().unwrap()
    }
}

impl CondExprKind {
    pub fn is_leaf(self) -> bool {
        self.num_children() == 0
    }

    /// Whether an expression of this kind can have `z` bits. Operators only give `0`, `1` and `x`.
    pub fn can_be_z(self) -> bool {
        matches!(
            self,
            Self::Constant
                | Self::Terminal
                | Self::Concat
                | Self::Mux
                | Self::ZeroExtend
                | Self::SignExtend
        )
    }

    pub fn num_children(self) -> usize {
        match self {
            Self::ScalarConstant | Self::Constant | Self::Terminal => 0,
            Self::ArithmeticNeg
            | Self::BitwiseNot
            | Self::LogicalNot
            | Self::ToLogical
            | Self::ReduceAnd
            | Self::ReduceOr
            | Self::ReduceXor
            | Self::ReduceNand
            | Self::ReduceNor
            | Self::ReduceXnor
            | Self::CaseEq0
            | Self::CaseEq1
            | Self::Eq1 => 1,
            // The width entry at `index + 1` is not a child.
            Self::ZeroExtend | Self::SignExtend => 1,
            Self::And
            | Self::Or
            | Self::Xor
            | Self::Xnor
            | Self::Eq
            | Self::Neq
            | Self::CaseEq
            | Self::CaseNeq
            | Self::BitwiseEquiv
            | Self::Concat => 2,
            Self::Mux => 3,
        }
    }
}

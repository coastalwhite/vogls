//! This module handles bit-select and part-select addressing calculations. Before, this was
//! repeated across several places because each operated with slightly different operands, inputs
//! and outputs. At first I thought this was trivial, it is appearently not, so I wanted to prevent
//! different bugs from popping up at all these sites and generaled the logic over some simpler
//! operations.

use std::marker::PhantomData;
use std::num::NonZeroU32;

use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{
    BasicBlockBuilder, Bits, GlobalContext, INTEGER_VSIZE, SCALAR_VSIZE, VariableKey, VectorSize,
};

use crate::ast::AstId;
use crate::ast::constant_expr::{ConstantBitSlice, ConstantExpr};
use crate::ast::expr::{BitSlice, BitSliceKind, Expr};
use crate::elaborate::{ArrayDim, VSymbolTable, VectorTransform};
use crate::parser::AstArenas;

use super::expression::{lower_expr, truncate_or_extend};
use super::{Diagnostics, LowerContext, MutLowerContext, eval_constant_expr};

pub enum RangeExpr<C: AddressingContext> {
    MsbLsb(C::ConstantExpr, C::ConstantExpr),
    PlusWidth(C::Expr, C::ConstantExpr),
    MinusWidth(C::Expr, C::ConstantExpr),
}

pub struct Address<C: AddressingContext> {
    pub elem_offset: Option<C::Var>,
    pub output_width: VectorSize,
    pub array: Option<(C::Var, C::Bool)>,
    pub is_unsigned: bool,
}

pub trait AddressingContext {
    type ConstantExpr;
    type Expr;
    type Var: Clone;
    type Bool;

    type Error;

    fn too_many_selects(&mut self) -> Self::Error;
    fn stride_overflow(&mut self) -> Self::Error;
    fn not_yet_implemented(&mut self, reason: &'static str) -> Self::Error;

    fn eval_constant(&mut self, operand: Self::ConstantExpr) -> Result<i64, Self::Error>;
    fn eval_var(&mut self, operand: Self::Expr) -> Result<Self::Var, Self::Error>;

    fn or_overflow(&mut self, lhs: Self::Bool, rhs: Self::Bool) -> Self::Bool;

    fn var_from_i64(&mut self, v: i64) -> Result<Self::Var, Self::Error>;
    fn var_geq_nonzerou32(
        &mut self,
        lhs: Self::Var,
        rhs: NonZeroU32,
    ) -> Result<Self::Bool, Self::Error>;
    fn var_mul_nonzerou32(
        &mut self,
        lhs: Self::Var,
        rhs: NonZeroU32,
    ) -> Result<Self::Var, Self::Error>;
    fn var_add(&mut self, lhs: Self::Var, rhs: Self::Var) -> Result<Self::Var, Self::Error>;
    fn var_sub_i64(&mut self, lhs: Self::Var, rhs: i64) -> Result<Self::Var, Self::Error>;
    fn var_revsub_u32(&mut self, lhs: Self::Var, rhs: u32) -> Result<Self::Var, Self::Error>;
}

/// Calculate the actual offset and width from the bit-select and part-selects.
pub fn lower_addressing<C: AddressingContext>(
    ctx: &mut C,
    elem_width: VectorSize,
    array_dims: &[ArrayDim],
    transform: VectorTransform,
    mut indices: impl ExactSizeIterator<Item = C::Expr>,
    range: Option<RangeExpr<C>>,
) -> Result<Address<C>, C::Error> {
    // Fast path: no bit-select, no part-selects.
    if array_dims.is_empty() && indices.len() == 0 && range.is_none() {
        return Ok(Address {
            elem_offset: None,
            output_width: elem_width,
            array: None,
            is_unsigned: false,
        });
    }

    // The specification is pretty clear that:
    // - Performing a range expression on an array is illegal (this is stated in an example in
    //   5.2.2)
    // - "A bit-select or part-select of a scalar, or of a variable or parameter of type real or
    //   realtime, shall be illegal." (5.2.1)
    if indices.len() + usize::from(range.is_some()) - 1 > array_dims.len() {
        return Err(ctx.too_many_selects());
    }

    let mut array: Option<(C::Var, C::Bool)> = None;
    let mut stride = elem_width;

    // Handle the array indexing.
    //
    // @NOTE
    // Even though, the specification does not state it explicitly. Array indices are indexed
    // left-to-right. This does not match sane programming languages, but then again, is Verilog
    // really a sane language?
    for (&dim, idx) in array_dims.iter().zip(indices.by_ref()) {
        let idx = ctx.eval_var(idx)?;
        let idx = ctx.var_sub_i64(idx, dim.offset)?;
        let does_idx_overflow = ctx.var_geq_nonzerou32(idx.clone(), dim.num_elements)?;
        let added_offset = ctx.var_mul_nonzerou32(idx, stride)?;
        array = Some(match array {
            Some((offset, overflow)) => (
                ctx.var_add(offset, added_offset)?,
                ctx.or_overflow(overflow, does_idx_overflow),
            ),
            None => (added_offset, does_idx_overflow),
        });
        stride = stride
            .checked_mul(dim.num_elements)
            .ok_or_else(|| ctx.stride_overflow())?;
    }

    // Bit-select path.
    if let Some(bit_select) = indices.next() {
        let mut elem_offset = C::eval_var(ctx, bit_select)?;
        if transform.lsb != 0 {
            elem_offset = C::var_sub_i64(ctx, elem_offset, transform.lsb)?;
        }
        if transform.bit_reversed {
            elem_offset = C::var_revsub_u32(ctx, elem_offset, elem_width.get() - 1)?;
        }

        return Ok(Address {
            elem_offset: Some(elem_offset),
            output_width: SCALAR_VSIZE,
            array,
            is_unsigned: true,
        });
    }

    // Array selection path.
    let Some(range_expr) = range else {
        return Ok(Address {
            elem_offset: None,
            output_width: elem_width,
            array,
            is_unsigned: false,
        });
    };

    // Part-select path.
    if transform.bit_reversed {
        // @TODO
        return Err(C::not_yet_implemented(ctx, "reverse bit range expression"));
    }
    let (mut range_lsb, range_width) = match range_expr {
        RangeExpr::MsbLsb(msb, lsb) => {
            let msb = C::eval_constant(ctx, msb)?;
            let lsb = C::eval_constant(ctx, lsb)?;
            let range_width = msb.abs_diff(lsb);
            let range_width = range_width + 1;
            let range_width = u32::try_from(range_width).unwrap();
            let range_width = VectorSize::new(range_width).unwrap();
            let lsb = C::var_from_i64(ctx, lsb)?;
            (lsb, range_width)
        }
        RangeExpr::PlusWidth(offset, width) => {
            let offset = C::eval_var(ctx, offset)?;
            let width = C::eval_constant(ctx, width)?;
            let range_width = u32::try_from(width).unwrap();
            let range_width = VectorSize::new(range_width).unwrap();
            (offset, range_width)
        }
        RangeExpr::MinusWidth(offset, width) => {
            let offset = C::eval_var(ctx, offset)?;
            let width = C::eval_constant(ctx, width)?;
            let range_width = u32::try_from(width).unwrap();
            let range_width = VectorSize::new(range_width).unwrap();
            let range_lsb = C::var_sub_i64(ctx, offset, i64::from(range_width.get() - 1))?;
            (range_lsb, range_width)
        }
    };
    if transform.lsb != 0 {
        range_lsb = C::var_sub_i64(ctx, range_lsb, transform.lsb)?;
    }

    Ok(Address {
        elem_offset: Some(range_lsb),
        output_width: range_width,
        array,
        is_unsigned: true,
    })
}

pub struct AddressingConstantExprWidthContext<'a, 'b> {
    pub gl: &'b GlobalContext,
    pub arenas: &'b AstArenas,
    pub table: &'b VSymbolTable,
    pub scope: SymbolId,
    pub diagnostics: &'b mut Diagnostics,
    pub loc: usize,
    pub _pd: PhantomData<&'a ()>,
}

impl<'a, 'b> AddressingContext for AddressingConstantExprWidthContext<'a, 'b> {
    type ConstantExpr = AstId<'a, ConstantExpr<'a>>;

    type Expr = ();
    type Var = ();
    type Bool = ();
    type Error = ();

    fn too_many_selects(&mut self) -> Self::Error {
        let tr = self.arenas.spans[self.loc];
        self.diagnostics
            .not_yet_implemented(tr, "cannot select from array or too many selects");
    }
    fn stride_overflow(&mut self) -> Self::Error {
        let tr = self.arenas.spans[self.loc];
        self.diagnostics.not_yet_implemented(tr, "stride overflow");
    }
    fn not_yet_implemented(&mut self, reason: &'static str) -> Self::Error {
        let tr = self.arenas.spans[self.loc];
        self.diagnostics.not_yet_implemented(tr, reason);
    }

    fn eval_constant(&mut self, operand: Self::ConstantExpr) -> Result<i64, Self::Error> {
        let result = eval_constant_expr(
            self.gl,
            self.arenas,
            self.table,
            self.scope,
            self.diagnostics,
            operand,
            None,
        )?;
        let Some(result) = result.as_integer() else {
            let tr = self.arenas.get_span(operand);
            self.diagnostics
                .not_yet_implemented(tr, "unable to use as operand");
            return Err(());
        };
        Ok(result)
    }

    #[inline(always)]
    fn eval_var(&mut self, _operand: Self::Expr) -> Result<Self::Var, Self::Error> {
        Ok(())
    }
    #[inline(always)]
    fn or_overflow(&mut self, _lhs: Self::Bool, _rhs: Self::Bool) -> Self::Bool {}
    #[inline(always)]
    fn var_from_i64(&mut self, _v: i64) -> Result<Self::Var, Self::Error> {
        Ok(())
    }
    #[inline(always)]
    fn var_geq_nonzerou32(
        &mut self,
        _lhs: Self::Var,
        _rhs: NonZeroU32,
    ) -> Result<Self::Bool, Self::Error> {
        Ok(())
    }
    #[inline(always)]
    fn var_mul_nonzerou32(
        &mut self,
        _lhs: Self::Var,
        _rhs: NonZeroU32,
    ) -> Result<Self::Var, Self::Error> {
        Ok(())
    }
    #[inline(always)]
    fn var_add(&mut self, _lhs: Self::Var, _rhs: Self::Var) -> Result<Self::Var, Self::Error> {
        Ok(())
    }
    #[inline(always)]
    fn var_sub_i64(&mut self, _lhs: Self::Var, _rhs: i64) -> Result<Self::Var, Self::Error> {
        Ok(())
    }
    #[inline(always)]
    fn var_revsub_u32(&mut self, _lhs: Self::Var, _rhs: u32) -> Result<Self::Var, Self::Error> {
        Ok(())
    }
}

pub struct LValueAddressingContext<'a, 'b> {
    pub ctx: &'b LowerContext<'a, 'b>,
    pub mctx: &'b mut MutLowerContext,
    pub builder: &'b mut BasicBlockBuilder,
    pub loc: usize,
    pub scope: SymbolId,
}

impl<'a, 'b> From<BitSlice<'a>> for RangeExpr<LValueAddressingContext<'a, 'b>> {
    fn from(value: BitSlice<'a>) -> Self {
        match value {
            BitSlice::MsbLsb(msb_expr, lsb_expr) => RangeExpr::MsbLsb(msb_expr, lsb_expr),
            BitSlice::PlusWidth(offset, width) => RangeExpr::PlusWidth(offset, width),
            BitSlice::MinusWidth(offset, width) => RangeExpr::MinusWidth(offset, width),
        }
    }
}

impl<'a, 'b> AddressingContext for LValueAddressingContext<'a, 'b> {
    type ConstantExpr = AstId<'a, ConstantExpr<'a>>;
    type Expr = AstId<'a, Expr<'a>>;
    type Var = VariableKey;
    type Bool = VariableKey;

    type Error = ();

    fn too_many_selects(&mut self) -> Self::Error {
        let tr = self.ctx.arenas.spans[self.loc];
        self.mctx
            .diagnostics
            .not_yet_implemented(tr, "cannot select from array or too many selects");
    }

    fn stride_overflow(&mut self) -> Self::Error {
        let tr = self.ctx.arenas.spans[self.loc];
        self.mctx
            .diagnostics
            .not_yet_implemented(tr, "stride overflow");
    }

    fn not_yet_implemented(&mut self, reason: &'static str) -> Self::Error {
        let tr = self.ctx.arenas.spans[self.loc];
        self.mctx.diagnostics.not_yet_implemented(tr, reason);
    }

    fn eval_constant(&mut self, operand: Self::ConstantExpr) -> Result<i64, Self::Error> {
        let value = eval_constant_expr(
            &self.mctx.gl,
            self.ctx.arenas,
            &self.ctx.table,
            self.scope,
            &mut self.mctx.diagnostics,
            operand,
            None,
        )?;
        value.as_integer().ok_or_else(|| {
            self.mctx
                .diagnostics
                .not_yet_implemented(self.ctx.arenas.get_span(operand), "cannot convert to index")
        })
    }

    fn eval_var(&mut self, operand: Self::Expr) -> Result<Self::Var, Self::Error> {
        let (var, ty) = lower_expr(self.ctx, self.mctx, self.scope, self.builder, operand, None)?;
        let var = truncate_or_extend(self.mctx.gl(), self.builder, var, ty, INTEGER_VSIZE);
        Ok(var)
    }

    fn or_overflow(&mut self, lhs: Self::Bool, rhs: Self::Bool) -> Self::Bool {
        self.builder.or(self.mctx.gl(), lhs, rhs)
    }

    fn var_from_i64(&mut self, v: i64) -> Result<Self::Var, Self::Error> {
        Ok(self.builder.constant(
            self.mctx.gl(),
            Bits::new_u64(v as u64).truncate(INTEGER_VSIZE),
        ))
    }

    fn var_geq_nonzerou32(
        &mut self,
        lhs: Self::Var,
        rhs: std::num::NonZeroU32,
    ) -> Result<Self::Bool, Self::Error> {
        let rhs = self
            .builder
            .constant(self.mctx.gl(), Bits::new_u32(rhs.get()));
        Ok(self.builder.unsigned_ge(self.mctx.gl(), lhs, rhs))
    }

    fn var_mul_nonzerou32(
        &mut self,
        lhs: Self::Var,
        rhs: std::num::NonZeroU32,
    ) -> Result<Self::Var, Self::Error> {
        Ok(self
            .builder
            .multiply_constant(self.mctx.gl(), lhs, Bits::new_u32(rhs.get())))
    }

    fn var_add(&mut self, lhs: Self::Var, rhs: Self::Var) -> Result<Self::Var, Self::Error> {
        Ok(self.builder.plus(self.mctx.gl(), lhs, rhs))
    }

    fn var_sub_i64(&mut self, lhs: Self::Var, rhs: i64) -> Result<Self::Var, Self::Error> {
        Ok(self.builder.minus_constant(
            self.mctx.gl(),
            lhs,
            Bits::new_u64(rhs as u64).truncate(INTEGER_VSIZE),
        ))
    }
    fn var_revsub_u32(&mut self, lhs: Self::Var, rhs: u32) -> Result<Self::Var, Self::Error> {
        Ok(self
            .builder
            .revminus_constant(self.mctx.gl(), lhs, Bits::new_u32(rhs)))
    }
}

pub struct ConstantAddressingContext<'a, 'b> {
    pub gl: &'b GlobalContext,
    pub arenas: &'b AstArenas,
    pub table: &'b VSymbolTable,
    pub scope: SymbolId,
    pub diagnostics: &'b mut Diagnostics,
    pub loc: usize,
    pub _pd: PhantomData<&'a ()>,
}

impl<'a, 'b> From<BitSlice<'a>> for RangeExpr<ConstantAddressingContext<'a, 'b>> {
    fn from(value: BitSlice<'a>) -> Self {
        match value {
            BitSlice::MsbLsb(msb_expr, lsb_expr) => RangeExpr::MsbLsb(msb_expr, lsb_expr),
            BitSlice::PlusWidth(offset, width) => {
                RangeExpr::PlusWidth(offset.into_constant(), width)
            }
            BitSlice::MinusWidth(offset, width) => {
                RangeExpr::MinusWidth(offset.into_constant(), width)
            }
        }
    }
}

impl<'a, 'b> From<ConstantBitSlice<'a>> for RangeExpr<ConstantAddressingContext<'a, 'b>> {
    fn from(value: ConstantBitSlice<'a>) -> Self {
        match value.kind {
            BitSliceKind::MsbLsb => RangeExpr::MsbLsb(value.fst, value.snd),
            BitSliceKind::PlusWidth => RangeExpr::PlusWidth(value.fst, value.snd),
            BitSliceKind::MinusWidth => RangeExpr::MinusWidth(value.fst, value.snd),
        }
    }
}

impl<'a, 'b> Address<ConstantAddressingContext<'a, 'b>> {
    pub fn signal_offset_as_u32(&self) -> Option<u32> {
        let offset = match (self.elem_offset, self.array) {
            (Some(elem_offset), Some((array_offset, _array_overflow))) => {
                elem_offset.checked_add(array_offset)?
            }
            (Some(elem_offset), None) => elem_offset,
            (None, Some((array_offset, _array_overflow))) => array_offset,
            (None, None) => 0,
        };
        u32::try_from(offset).ok()
    }
}

impl<'a, 'b> AddressingContext for ConstantAddressingContext<'a, 'b> {
    type ConstantExpr = AstId<'a, ConstantExpr<'a>>;
    type Expr = AstId<'a, ConstantExpr<'a>>;
    type Var = i64;
    type Bool = bool;

    type Error = ();

    fn too_many_selects(&mut self) -> Self::Error {
        let tr = self.arenas.spans[self.loc];
        self.diagnostics
            .not_yet_implemented(tr, "cannot select from array or too many selects");
    }

    fn stride_overflow(&mut self) -> Self::Error {
        let tr = self.arenas.spans[self.loc];
        self.diagnostics.not_yet_implemented(tr, "stride overflow");
    }

    fn not_yet_implemented(&mut self, reason: &'static str) -> Self::Error {
        let tr = self.arenas.spans[self.loc];
        self.diagnostics.not_yet_implemented(tr, reason);
    }

    fn eval_constant(&mut self, operand: Self::ConstantExpr) -> Result<i64, Self::Error> {
        let value = eval_constant_expr(
            self.gl,
            self.arenas,
            self.table,
            self.scope,
            self.diagnostics,
            operand,
            None,
        )?;
        value.as_integer().ok_or_else(|| {
            self.diagnostics
                .not_yet_implemented(self.arenas.get_span(operand), "cannot convert to index")
        })
    }

    fn eval_var(&mut self, operand: Self::Expr) -> Result<Self::Var, Self::Error> {
        let value = eval_constant_expr(
            self.gl,
            self.arenas,
            self.table,
            self.scope,
            self.diagnostics,
            operand,
            None,
        )?;
        value.as_integer().ok_or_else(|| {
            self.diagnostics
                .not_yet_implemented(self.arenas.get_span(operand), "cannot convert to index")
        })
    }

    fn or_overflow(&mut self, lhs: Self::Bool, rhs: Self::Bool) -> Self::Bool {
        lhs | rhs
    }
    fn var_from_i64(&mut self, v: i64) -> Result<Self::Var, Self::Error> {
        Ok(v)
    }
    fn var_geq_nonzerou32(
        &mut self,
        lhs: Self::Var,
        rhs: std::num::NonZeroU32,
    ) -> Result<Self::Bool, Self::Error> {
        Ok(lhs >= i64::from(rhs.get()))
    }
    fn var_mul_nonzerou32(
        &mut self,
        lhs: Self::Var,
        rhs: std::num::NonZeroU32,
    ) -> Result<Self::Var, Self::Error> {
        lhs.checked_mul(i64::from(rhs.get())).ok_or(())
    }

    fn var_add(&mut self, lhs: Self::Var, rhs: Self::Var) -> Result<Self::Var, Self::Error> {
        lhs.checked_add(rhs).ok_or(())
    }

    fn var_sub_i64(&mut self, lhs: Self::Var, rhs: i64) -> Result<Self::Var, Self::Error> {
        lhs.checked_sub(rhs).ok_or(())
    }
    fn var_revsub_u32(&mut self, lhs: Self::Var, rhs: u32) -> Result<Self::Var, Self::Error> {
        i64::from(rhs).checked_sub(lhs).ok_or(())
    }
}

#[cfg(test)]
#[test]
fn test() {
    struct PartSelectI64;
    impl AddressingContext for PartSelectI64 {
        type Expr = i64;
        type Var = i64;
        type ConstantExpr = i64;
        type Bool = bool;

        type Error = ();

        fn too_many_selects(&mut self) -> Self::Error {
            ()
        }
        fn stride_overflow(&mut self) -> Self::Error {
            ()
        }
        fn not_yet_implemented(&mut self, _: &'static str) -> Self::Error {
            ()
        }

        fn eval_constant(&mut self, operand: Self::ConstantExpr) -> Result<i64, Self::Error> {
            Ok(operand)
        }

        fn eval_var(&mut self, operand: Self::Expr) -> Result<Self::Var, Self::Error> {
            Ok(operand)
        }

        fn or_overflow(&mut self, lhs: Self::Bool, rhs: Self::Bool) -> Self::Bool {
            lhs | rhs
        }

        fn var_from_i64(&mut self, v: i64) -> Result<Self::Var, Self::Error> {
            Ok(v)
        }

        fn var_geq_nonzerou32(
            &mut self,
            lhs: Self::Var,
            rhs: NonZeroU32,
        ) -> Result<Self::Bool, Self::Error> {
            Ok(lhs >= i64::from(rhs.get()))
        }

        fn var_mul_nonzerou32(
            &mut self,
            lhs: Self::Var,
            rhs: NonZeroU32,
        ) -> Result<Self::Var, Self::Error> {
            lhs.checked_mul(i64::from(rhs.get())).ok_or(())
        }

        fn var_add(&mut self, lhs: Self::Var, rhs: Self::Var) -> Result<Self::Var, Self::Error> {
            lhs.checked_add(rhs).ok_or(())
        }

        fn var_sub_i64(&mut self, lhs: Self::Var, rhs: i64) -> Result<Self::Var, Self::Error> {
            lhs.checked_sub(rhs).ok_or(())
        }
        fn var_revsub_u32(&mut self, lhs: Self::Var, rhs: u32) -> Result<Self::Var, Self::Error> {
            i64::from(rhs).checked_sub(lhs).ok_or(())
        }
    }

    macro_rules! test_case {
        (@range  ; $msb:expr ; $lsb:expr) => {{ Some(RangeExpr::MsbLsb($msb, $lsb)) }};
        (@range +; $offset:expr ; $width:expr) => {{ Some(RangeExpr::PlusWidth($offset, $width)) }};
        (@range -; $offset:expr ; $width:expr) => {{ Some(RangeExpr::MinusWidth($offset, $width)) }};
        (@range) => {{ None }};
        (
            [$msb:expr ; $lsb:expr] _ $([$arr_length:expr])* {
                $(
                    (
                        $([$idx:expr])*
                        $([  ; $tc_msb:expr ; $tc_lsb:expr])?
                        $([ +; $tc_pw_offset:expr ; $tc_pw_width:expr])?
                        $([ -; $tc_mw_offset:expr ; $tc_mw_width:expr])?
                    ): ($offset:expr, $width:expr, $array:expr)
                ),+ $(,)?
            }
        ) => {
            let msb: i64 = $msb;
            let lsb: i64 = $lsb;
            let transform = VectorTransform::from_msb_lsb(msb, lsb).unwrap();
            let width = VectorSize::new(u32::try_from(msb.abs_diff(lsb)).unwrap() + 1).unwrap();
            let dims = [$(ArrayDim { offset: 0, num_elements: NonZeroU32::new($arr_length).unwrap() }),*];

            $(
            let result = lower_addressing(
                &mut PartSelectI64,
                width,
                &dims,
                transform,
                [$($idx),*].into_iter(),
                test_case!(@range
                    $( ; $tc_msb ; $tc_lsb)?
                    $(+; $tc_pw_offset ; $tc_pw_width)?
                    $(-; $tc_mw_offset ; $tc_mw_width)?
                ),
            )
            .unwrap();
            assert_eq!(result.elem_offset, $offset);
            assert_eq!(result.output_width, VectorSize::new($width).unwrap());
            assert_eq!(result.array, $array);
            )+
        };
    }

    test_case!([7;0] _ {
        ():          (None,    8, None),
        ([; 3 ; 2]): (Some(2), 2, None),
    });

    test_case!([4;0] _ [3] [2] {
        ([0][0]):          (None,    5, Some((0,  false))),
        ([2][1]):          (None,    5, Some((25, false))),
        ([0][2]):          (None,    5, Some((30, true))),
        ([0][2] [; 4; 1]): (Some(1), 4, Some((30, true))),
        ([0][2][2]):       (Some(2), 1, Some((30, true))),
        ([0][1][2]):       (Some(2), 1, Some((15, false))),
    });

    test_case!([2;9] _ {
        ():             (None,    8, None),
        ([2]):          (Some(7), 1, None),
        ([3]):          (Some(6), 1, None),
        ([4]):          (Some(5), 1, None),
        ([5]):          (Some(4), 1, None),
        ([6]):          (Some(3), 1, None),
        ([7]):          (Some(2), 1, None),
        ([8]):          (Some(1), 1, None),
        ([9]):          (Some(0), 1, None),
    });
}

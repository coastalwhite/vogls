use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{BasicBlockBuilder, VariableKey, VectorSize};

use crate::ast::AstId;
use crate::ast::expr::{BitSlice, BitSliceKind};
use crate::ast::statement::{NetLValue, NetLValueFlat, VariableLValue, VariableLValueFlat};
use crate::elaborate::{ArrayDim, VSymbol, VectorTransform};
use crate::lower::addressing::{
    Address, AddressingConstantExprWidthContext, LValueAddressingContext, RangeExpr,
    lower_addressing,
};
use crate::lower::expression::{self, truncate_or_extend};
use crate::lower::try_resolve_hident;

use super::addressing::ConstantAddressingContext;
use super::{LowerContext, try_resolve_net};
use super::{MutLowerContext, VType};

pub fn assign_variable_lvalue<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    builder: &mut BasicBlockBuilder,
    ast_lvalue: AstId<'a, VariableLValue<'a>>,
    variable: VariableKey,
    variable_ty: VType,
    nba: bool,
) -> Result<(), ()> {
    if ast_lvalue.0.len() == 1 {
        return assign_variable_lvalue_flat(
            ctx,
            mctx,
            scope,
            builder,
            ast_lvalue.0.get(0),
            variable,
            variable_ty,
            nba,
        );
    }

    assert!(!ast_lvalue.0.is_empty());
    let mut total_width = 0u32;
    for lvf in ast_lvalue.0.iter() {
        let ty = variable_lvalue_flat_ty(ctx, mctx, scope, lvf)?;
        total_width += ty.bit_length().get();
    }
    let variable = truncate_or_extend(
        mctx.gl(),
        builder,
        variable,
        variable_ty,
        VectorSize::new(total_width).unwrap(),
    );

    let mut offset = 0u32;
    for lvf in ast_lvalue.0.iter().rev() {
        let ty = variable_lvalue_flat_ty(ctx, mctx, scope, lvf)?;
        let width = ty.bit_length();
        let variable = builder.slice_constant(mctx.gl(), variable, offset, width);
        assign_variable_lvalue_flat(ctx, mctx, scope, builder, lvf, variable, ty, nba)?;
        offset += width.get();
    }
    Ok(())
}

pub fn variable_lvalue_size<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    ast_lvalue: AstId<'a, VariableLValue<'a>>,
) -> Result<VectorSize, ()> {
    // @TODO: Overflow checks
    let mut size = 0;
    for lvalue_flat in ast_lvalue.0.iter() {
        size += variable_lvalue_flat_ty(ctx, mctx, scope, lvalue_flat)?
            .bit_length()
            .get();
    }
    Ok(VectorSize::new(size).unwrap())
}

pub fn variable_lvalue_flat_ty<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    ast_lvalue: AstId<'a, VariableLValueFlat<'a>>,
) -> Result<VType, ()> {
    let VariableLValueFlat {
        ident,
        exprs,
        range_expression,
    } = &*ast_lvalue;

    let symbol_key =
        try_resolve_hident(scope, &ctx.table, ctx.arenas, *ident, &mut mctx.diagnostics)?;

    let exprs = *exprs;
    let (ty, array_dims, transform) = match &ctx.table[symbol_key].content {
        VSymbol::Net(s) => (s.ty, s.dims.as_slice(), s.transform),
        _ => todo!(),
    };

    let mut actx = AddressingConstantExprWidthContext {
        gl: &mctx.gl,
        arenas: ctx.arenas,
        table: &ctx.table,
        scope,
        diagnostics: &mut mctx.diagnostics,
        loc: ast_lvalue.loc,
        _pd: std::marker::PhantomData,
    };
    let range = range_expression.map(|r| match &*r {
        BitSlice::MsbLsb(msb, lsb) => RangeExpr::MsbLsb(*msb, *lsb),
        BitSlice::PlusWidth(_, width) => RangeExpr::PlusWidth((), *width),
        BitSlice::MinusWidth(_, width) => RangeExpr::MinusWidth((), *width),
    });
    let address = lower_addressing(
        &mut actx,
        ty.bit_length(),
        array_dims,
        transform,
        std::iter::repeat_n((), exprs.len()),
        range,
    )?;

    Ok(if address.is_unsigned {
        VType::UnsignedNet(address.output_width)
    } else {
        ty.truncate(address.output_width)
    })
}

pub fn assign_variable_lvalue_flat<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    builder: &mut BasicBlockBuilder,
    ast_lvalue: AstId<'a, VariableLValueFlat<'a>>,
    variable: VariableKey,
    variable_ty: VType,
    nba: bool,
) -> Result<(), ()> {
    let VariableLValueFlat {
        ident,
        exprs,
        range_expression,
    } = &*ast_lvalue;

    let symbol_key =
        try_resolve_hident(scope, &ctx.table, ctx.arenas, *ident, &mut mctx.diagnostics)?;

    let (ty, array_dims, transform) = match &ctx.table[symbol_key].content {
        VSymbol::Parameter(v) => (v.ty(), &[] as &[ArrayDim], VectorTransform::default()),
        VSymbol::Net(s) => (s.ty, s.dims.as_slice(), s.transform),
        v => todo!("lvalue assign: {v:?}"),
    };

    let range_expr = range_expression.map(|r| (*r).into());
    let mut actx = LValueAddressingContext {
        ctx,
        mctx,
        builder,
        loc: ast_lvalue.loc,
        scope,
    };

    let Address {
        elem_offset,
        output_width,
        array,
        is_unsigned: _,
    } = lower_addressing(
        &mut actx,
        ty.bit_length(),
        array_dims,
        transform,
        exprs.iter(),
        range_expr,
    )?;

    let VSymbol::Net(net) = &ctx.table[symbol_key].content else {
        mctx.diagnostics
            .not_yet_implemented(ctx.arenas.get_span(ast_lvalue), "assign not to non-net");
        return Err(());
    };

    // @TODO: Use array overflow.
    let partial = match (elem_offset, array) {
        (Some(elem_offset), Some((array_offset, _array_overflow))) => {
            Some(builder.plus(mctx.gl(), elem_offset, array_offset))
        }
        (Some(elem_offset), None) => Some(elem_offset),
        (None, Some((array_offset, _array_overflow))) => Some(array_offset),
        (None, None) => None,
    };
    let variable = expression::coerce_to(
        mctx.gl(),
        builder,
        variable,
        variable_ty,
        match net.ty {
            VType::SignedNet(_) => VType::SignedNet(output_width),
            VType::UnsignedNet(_) => VType::UnsignedNet(output_width),
            VType::Real => VType::Real,
        },
    );
    if nba {
        net.net
            .drive_non_blocking(&mut mctx.gl, &mut mctx.nbas, builder, variable, partial);
    } else {
        net.net
            .drive_blocking(mctx.gl(), builder, variable, partial);
    }

    Ok(())
}

pub fn assign_net_lvalue<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    builder: &mut BasicBlockBuilder,
    ast_lvalue: AstId<'a, NetLValue<'a>>,
    variable: VariableKey,
    variable_ty: VType,
) -> Result<(), ()> {
    let lvalue = &*ast_lvalue;
    if lvalue.0.len() == 1 {
        return assign_net_lvalue_flat(
            ctx,
            mctx,
            scope,
            builder,
            lvalue.0.get(0),
            variable,
            variable_ty,
        );
    }

    assert!(!lvalue.0.is_empty());
    let mut total_width = 0u32;
    for lvf in lvalue.0.iter() {
        let ty = net_lvalue_flat_ty(ctx, mctx, scope, lvf)?;
        total_width += ty.bit_length().get();
    }
    let variable = truncate_or_extend(
        mctx.gl(),
        builder,
        variable,
        variable_ty,
        VectorSize::new(total_width).unwrap(),
    );

    let mut offset = 0u32;
    for lvf in lvalue.0.iter().rev() {
        let ty = net_lvalue_flat_ty(ctx, mctx, scope, lvf)?;
        let width = ty.bit_length();
        let variable = builder.slice_constant(mctx.gl(), variable, offset, width);
        assign_net_lvalue_flat(ctx, mctx, scope, builder, lvf, variable, ty)?;
        offset += width.get();
    }
    Ok(())
}

pub fn net_lvalue_size<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    ast_lvalue: AstId<'a, NetLValue<'a>>,
) -> Result<VectorSize, ()> {
    // @TODO: Overflow checks
    let mut size = 0;
    for lvalue_flat in ast_lvalue.0.iter() {
        size += net_lvalue_flat_ty(ctx, mctx, scope, lvalue_flat)?
            .bit_length()
            .get();
    }
    Ok(VectorSize::new(size).unwrap())
}

pub fn net_lvalue_flat_ty<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    ast_lvalue: AstId<'a, NetLValueFlat<'a>>,
) -> Result<VType, ()> {
    let NetLValueFlat {
        ident,
        constant_exprs,
        constant_range_expression,
    } = &*ast_lvalue;

    let symbol_key =
        try_resolve_hident(scope, &ctx.table, ctx.arenas, *ident, &mut mctx.diagnostics)?;

    let (ty, dims, transform) = match &ctx.table[symbol_key].content {
        VSymbol::Parameter(v) => (v.ty(), &[] as &[ArrayDim], VectorTransform::default()),
        VSymbol::Net(s) => (s.ty, s.dims.as_slice(), s.transform),
        _ => todo!(),
    };

    let mut actx = AddressingConstantExprWidthContext {
        gl: &mctx.gl,
        arenas: ctx.arenas,
        table: &ctx.table,
        scope,
        diagnostics: &mut mctx.diagnostics,
        loc: ast_lvalue.loc,
        _pd: std::marker::PhantomData,
    };

    let range = constant_range_expression.map(|r| match r.kind {
        BitSliceKind::MsbLsb => RangeExpr::MsbLsb(r.fst, r.snd),
        BitSliceKind::PlusWidth => RangeExpr::PlusWidth((), r.snd),
        BitSliceKind::MinusWidth => RangeExpr::MinusWidth((), r.snd),
    });

    // @TODO: Statically check for overflow.
    let Address {
        elem_offset: _,
        output_width,
        array: _,
        is_unsigned,
    } = lower_addressing(
        &mut actx,
        ty.bit_length(),
        dims,
        transform,
        std::iter::repeat_n((), constant_exprs.len()),
        range,
    )?;

    Ok(if is_unsigned {
        VType::UnsignedNet(output_width)
    } else {
        ty.truncate(output_width)
    })
}

fn assign_net_lvalue_flat<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    builder: &mut BasicBlockBuilder,
    lvalue: AstId<'a, NetLValueFlat<'a>>,
    variable: VariableKey,
    variable_ty: VType,
) -> Result<(), ()> {
    let NetLValueFlat {
        ident,
        constant_exprs,
        constant_range_expression,
    } = &*lvalue;

    let s = try_resolve_net(scope, &ctx.table, ctx.arenas, *ident, &mut mctx.diagnostics)?;

    let mut actx = ConstantAddressingContext {
        gl: &mctx.gl,
        arenas: ctx.arenas,
        table: &ctx.table,
        scope,
        diagnostics: &mut mctx.diagnostics,
        loc: lvalue.loc,
        _pd: std::marker::PhantomData,
    };

    let range = constant_range_expression.map(|r| match r.kind {
        BitSliceKind::MsbLsb => RangeExpr::MsbLsb(r.fst, r.snd),
        BitSliceKind::PlusWidth => RangeExpr::PlusWidth(r.fst, r.snd),
        BitSliceKind::MinusWidth => RangeExpr::MinusWidth(r.fst, r.snd),
    });

    let Address {
        elem_offset,
        output_width,
        array,
        is_unsigned: _,
    } = lower_addressing(
        &mut actx,
        s.ty.bit_length(),
        &s.dims,
        s.transform,
        constant_exprs.iter(),
        range,
    )?;

    // @TODO: Use array overflow.
    let partial = match (elem_offset, array) {
        (Some(elem_offset), Some((array_offset, _array_overflow))) => {
            let offset = elem_offset.checked_add(array_offset).ok_or_else(|| {
                mctx.diagnostics
                    .not_yet_implemented(ctx.arenas.get_span(lvalue), "overflow");
            })?;
            Some(offset)
        }
        (Some(elem_offset), None) => Some(elem_offset),
        (None, Some((array_offset, _array_overflow))) => Some(array_offset),
        (None, None) => None,
    };
    let partial = match partial {
        None => None,
        Some(partial) => Some(builder.constant_u32(
            &mut mctx.gl,
            u32::try_from(partial).map_err(|_| {
                mctx.diagnostics
                    .not_yet_implemented(ctx.arenas.get_span(lvalue), "overflow");
            })?,
        )),
    };
    let var = expression::coerce_to(
        mctx.gl(),
        builder,
        variable,
        variable_ty,
        s.ty.resize_net_to(output_width),
    );
    s.net.drive_blocking(mctx.gl(), builder, var, partial);
    Ok(())
}

pub fn net_lvalue_bit_length<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    output_terminal: AstId<'a, NetLValue<'a>>,
) -> Result<VectorSize, ()> {
    let lvalue = &*output_terminal;
    let lvalue_flat = lvalue
        .0
        .first()
        .expect("Concatenation should have at least one value");
    let mut size = net_lvalue_flat_ty(ctx, mctx, scope, lvalue_flat)?.bit_length();

    for lvalue_flat in lvalue.0.iter().skip(1) {
        let lvalue_size = net_lvalue_flat_ty(ctx, mctx, scope, lvalue_flat)?.bit_length();
        size = size.checked_add(lvalue_size.get()).ok_or_else(|| {
            mctx.diagnostics
                .net_width_overflow(ctx.arenas.get_span(output_terminal));
        })?;
    }
    Ok(size)
}

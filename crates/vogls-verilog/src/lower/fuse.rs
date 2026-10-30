use std::cmp;

use slotmap::SlotMap;
use vogls_frontend::symbol_table::SymbolId;
use vogls_fuse_signals::{Driver, InputEdge};
use vogls_ir::{
    Bits, GlobalContext, SCALAR_VSIZE, Signal, SignalKey, SignalSlice, VSIZE_64, VectorSize,
};

use crate::ast::constant_expr::ConstantExpr;
use crate::ast::expr::Expr;
use crate::ast::statement::NetLValueFlat;
use crate::ast::{AstId, AstIdRange, HIdent};
use crate::elaborate::VSymbol;
use crate::lower::addressing::lower_addressing;
use crate::lower::try_resolve_hident;
use crate::number::Sign;

use super::addressing::{ConstantAddressingContext, RangeExpr};
use super::{Diagnostics, LowerContext, VType};

#[must_use]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FuseResult {
    Success,
    Failed,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FuseType {
    Unsigned,
    Signed,
    Real,
}

pub enum FuseSourceContent {
    Constant(Bits),
    Signal {
        signal: SignalKey,
        slice: SignalSlice,
    },
}

pub struct FuseSource {
    pub content: FuseSourceContent,
    pub ty: FuseType,
}

pub struct FuseDestination {
    pub signal: SignalKey,
    pub slice: SignalSlice,
    pub ty: FuseType,
}

impl FuseSource {
    pub fn bit_length(&self) -> VectorSize {
        match &self.content {
            FuseSourceContent::Constant(bits) => bits.size(),
            FuseSourceContent::Signal { signal: _, slice } => slice.width(),
        }
    }

    fn to_destination(&self) -> Option<FuseDestination> {
        match self.content {
            FuseSourceContent::Constant(_) => None,
            FuseSourceContent::Signal { signal, slice } => Some(FuseDestination {
                signal,
                slice,
                ty: self.ty,
            }),
        }
    }
}

impl From<VType> for FuseType {
    fn from(value: VType) -> Self {
        match value {
            VType::SignedNet(_) => FuseType::Signed,
            VType::UnsignedNet(_) => FuseType::Unsigned,
            VType::Real => FuseType::Real,
        }
    }
}

impl FuseDestination {
    pub fn bit_length(&self) -> VectorSize {
        self.slice.width()
    }
}

#[inline(always)]
fn signal_slice_to_opt(
    signals: &SlotMap<SignalKey, Signal>,
    signal: SignalKey,
    slice: SignalSlice,
) -> Option<SignalSlice> {
    ((slice.lsb() != 0) | (signals[signal].size != slice.width())).then_some(slice)
}

pub fn try_fuse_srcs_to_dsts(
    gl: &GlobalContext,
    connections: &mut Vec<InputEdge>,

    // Fuse sources from least significant bits to most significant bits.
    srcs: &[FuseSource],

    // Fuse destinations from least significant bits to most significant bits.
    dsts: &[FuseDestination],
) -> FuseResult {
    // Prevent the fuse if any source or destination reads a signal out of bounds.
    if srcs.iter().any(|src| {
        let FuseSourceContent::Signal { signal, slice } = src.content else {
            return false;
        };

        slice.msb() >= gl.signals[signal].size.get()
    }) {
        return FuseResult::Failed;
    }
    if dsts
        .iter()
        .any(|dst| dst.slice.msb() >= gl.signals[dst.signal].size.get())
    {
        return FuseResult::Failed;
    }

    // Fast path and base case: both sides consist of 1 signal that has the same bit length and
    // type.
    if let [src] = srcs
        && let [dst] = dsts
        && src.bit_length() == dst.bit_length()
        && (src.ty == T::Real) == (dst.ty == T::Real)
    {
        let driver = match &src.content {
            FuseSourceContent::Constant(bits) => Driver::Constant(bits.clone()),
            FuseSourceContent::Signal { signal, slice } => {
                Driver::Signal(*signal, signal_slice_to_opt(&gl.signals, *signal, *slice))
            }
        };
        connections.push(InputEdge {
            driver,
            drivee: dst.signal,
            drivee_slice: signal_slice_to_opt(&gl.signals, dst.signal, dst.slice),
        });
        return FuseResult::Success;
    }

    use FuseType as T;
    let start_connections_len = connections.len();

    let mut src_i = 0;
    let mut dst_i = 0;
    let mut src_bit_offset = 0u32;
    let mut dst_bit_offset = 0u32;
    while let Some(src) = srcs.get(src_i)
        && let Some(dst) = dsts.get(dst_i)
    {
        // Don't do any kind of fusing if only one side is a real.
        if (src.ty == T::Real) ^ (dst.ty == T::Real) {
            connections.truncate(start_connections_len);
            return FuseResult::Failed;
        }

        let src_bit_length = src.bit_length();
        let dst_bit_length = dst.bit_length();

        debug_assert!(src_bit_offset < src_bit_length.get());
        debug_assert!(dst_bit_offset < dst_bit_length.get());

        let src_rem_bit_length = VectorSize::new(src_bit_length.get() - src_bit_offset).unwrap();
        let dst_rem_bit_length = VectorSize::new(dst_bit_length.get() - dst_bit_offset).unwrap();

        let step_bit_length = cmp::min(src_rem_bit_length, dst_rem_bit_length);

        let driver = match &src.content {
            FuseSourceContent::Constant(bits) => {
                Driver::Constant(bits.slicez(src_bit_offset, step_bit_length))
            }
            FuseSourceContent::Signal { signal, slice } => {
                let offset_slice =
                    SignalSlice::from_width(src_bit_offset, step_bit_length).unwrap();
                let slice = slice.subslice(offset_slice).unwrap();
                Driver::Signal(*signal, signal_slice_to_opt(&gl.signals, *signal, slice))
            }
        };
        let offset_slice = SignalSlice::from_width(dst_bit_offset, step_bit_length).unwrap();
        let dst_slice = dst.slice.subslice(offset_slice).unwrap();
        connections.push(InputEdge {
            driver,
            drivee: dst.signal,
            drivee_slice: signal_slice_to_opt(&gl.signals, dst.signal, dst_slice),
        });

        src_bit_offset += step_bit_length.get();
        dst_bit_offset += step_bit_length.get();

        if src_bit_offset >= src_bit_length.get() {
            src_i += 1;
            src_bit_offset = 0;
        }
        if dst_bit_offset >= dst_bit_length.get() {
            dst_i += 1;
            dst_bit_offset = 0;
        }
    }

    // We don't sign-extend, we only zero-extend.
    //
    // @Performance:
    // 1. This is somewhat conservative. For example, `assign wide_net = {signed_scalar_net, 1'b0}`
    //    is rejected here, but could still zero-extend.
    // 2. For sign-extensions, we would make all remaining bit be driven by the last source bit. For
    //    that it would make sense to have some kind of repeated driver.
    if dst_i < dsts.len() && srcs.last().is_some_and(|src| src.ty == T::Signed) {
        connections.truncate(start_connections_len);
        return FuseResult::Failed;
    }

    // Zero-extend the destination.
    for dst in &dsts[dst_i..] {
        let dst_bit_length = dst.bit_length();
        let dst_rem_bit_length = VectorSize::new(dst_bit_length.get() - dst_bit_offset).unwrap();
        let offset_slice = SignalSlice::from_width(dst_bit_offset, dst_rem_bit_length).unwrap();
        let dst_slice = dst.slice.subslice(offset_slice).unwrap();

        connections.push(InputEdge {
            driver: Driver::Constant(Bits::new_zeroed(dst_rem_bit_length)),
            drivee: dst.signal,
            drivee_slice: signal_slice_to_opt(&gl.signals, dst.signal, dst_slice),
        });

        dst_bit_offset = 0;
    }

    FuseResult::Success
}

pub fn convert_fuse_srcs_to_dsts(
    srcs_start_length: usize,
    fuse_dsts: &mut Vec<FuseDestination>,
    fuse_srcs: &mut Vec<FuseSource>,
) -> FuseResult {
    let mut failed_conversion = false;
    let dsts_start_length = fuse_dsts.len();
    fuse_dsts.extend(fuse_srcs.drain(srcs_start_length..).map(|src| {
        let Some(dst) = src.to_destination() else {
            failed_conversion = true;

            // Insert a dummy element.
            return FuseDestination {
                signal: SignalKey::default(),
                slice: SignalSlice::with_end(SCALAR_VSIZE),
                ty: FuseType::Unsigned,
            };
        };

        dst
    }));

    if failed_conversion {
        fuse_dsts.truncate(dsts_start_length);
        return FuseResult::Failed;
    }

    FuseResult::Success
}

pub fn try_expr_to_fuse_sources<'a>(
    ctx: &LowerContext<'a, '_>,
    gl: &GlobalContext,
    fuse_srcs: &mut Vec<FuseSource>,
    scope: SymbolId,
    expr: AstId<'a, Expr<'a>>,
) -> FuseResult {
    try_expr_to_fuse(ctx, gl, fuse_srcs, scope, expr, SignalSide::Probe)
}

pub enum SignalSide {
    Probe,
    BlockingDrive,
}

pub fn try_expr_to_fuse<'a>(
    ctx: &LowerContext<'a, '_>,
    gl: &GlobalContext,
    fuse_srcs: &mut Vec<FuseSource>,
    scope: SymbolId,
    expr: AstId<'a, Expr<'a>>,
    side: SignalSide,
) -> FuseResult {
    // @TODO: This could also allow for `Concat` and `Replication`.

    match &*expr {
        Expr::Ident(ident, exprs, range_expr) => try_ident_to_fuse_sources(
            ctx,
            gl,
            fuse_srcs,
            scope,
            *ident,
            exprs.into_constant(),
            *range_expr,
            side,
        ),
        Expr::Sized(sized) => {
            let sized = &ctx.arenas.sized_numbers[sized.item.at];
            let ty = match sized.sign {
                Sign::Unsigned => FuseType::Unsigned,
                Sign::Signed => FuseType::Signed,
            };
            let value = sized.value.clone();
            fuse_srcs.push(FuseSource {
                content: FuseSourceContent::Constant(value),
                ty,
            });
            FuseResult::Success
        }
        _ => FuseResult::Failed,
    }
}

pub fn try_ident_to_fuse_sources<'a, R>(
    ctx: &LowerContext<'a, '_>,
    gl: &GlobalContext,
    fuse_srcs: &mut Vec<FuseSource>,
    scope: SymbolId,
    ident: HIdent<'_>,
    exprs: AstIdRange<'a, ConstantExpr<'a>>,
    range_expr: Option<R>,
    side: SignalSide,
) -> FuseResult
where
    R: for<'b> Into<RangeExpr<ConstantAddressingContext<'a, 'b>>>,
{
    let Ok(symbol_id) = try_resolve_hident(
        scope,
        &ctx.table,
        ctx.arenas,
        ident,
        &mut Diagnostics::default(),
    ) else {
        return FuseResult::Failed;
    };
    let net = match &ctx.table[symbol_id].content {
        VSymbol::Net(n) => n,
        _ => return FuseResult::Failed,
    };

    let signal = match side {
        SignalSide::Probe => net.net.probe_signal(),
        SignalSide::BlockingDrive => net.net.blocking_drive_signal(),
    };

    // Fast path: no part select or array indexing.
    if exprs.is_empty() && range_expr.is_none() && net.dims.is_empty() {
        let (ty, bit_length) = match net.ty {
            VType::SignedNet(bit_length) => (FuseType::Signed, bit_length),
            VType::UnsignedNet(bit_length) => (FuseType::Unsigned, bit_length),
            VType::Real => (FuseType::Real, VSIZE_64),
        };

        fuse_srcs.push(FuseSource {
            content: FuseSourceContent::Signal {
                signal,
                slice: SignalSlice::with_end(bit_length),
            },
            ty,
        });
        return FuseResult::Success;
    }

    let mut actx = ConstantAddressingContext {
        gl,
        arenas: ctx.arenas,
        table: &ctx.table,
        scope,
        diagnostics: &mut Diagnostics::default(),
        loc: 0,
        _pd: std::marker::PhantomData,
    };

    let Ok(address) = lower_addressing(
        &mut actx,
        net.ty.bit_length(),
        &net.dims,
        net.transform,
        exprs.iter(),
        range_expr.map(Into::into),
    ) else {
        return FuseResult::Failed;
    };

    // Array element overflow.
    if matches!(address.array, Some((_, true))) {
        return FuseResult::Failed;
    }

    // A bit- or part-select past the element reads x, not the neighbouring element.
    if let Some(elem_offset) = address.elem_offset
        && (elem_offset < 0
            || elem_offset + i64::from(address.output_width.get())
                > i64::from(net.ty.bit_length().get()))
    {
        return FuseResult::Failed;
    }

    let Some(offset) = address.signal_offset_as_u32() else {
        return FuseResult::Failed;
    };

    let Some(output_slice) = SignalSlice::from_width(offset, address.output_width) else {
        return FuseResult::Failed;
    };

    let ty = if net.ty.is_real() {
        FuseType::Real
    } else if address.is_unsigned {
        FuseType::Unsigned
    } else if net.ty.is_signed_net() {
        FuseType::Signed
    } else {
        FuseType::Unsigned
    };
    fuse_srcs.push(FuseSource {
        content: FuseSourceContent::Signal {
            signal,
            slice: output_slice,
        },
        ty,
    });
    FuseResult::Success
}

pub fn try_net_flat_lvalue_to_dsts<'a>(
    ctx: &LowerContext<'a, '_>,
    gl: &GlobalContext,
    fuse_dsts: &mut Vec<FuseDestination>,
    fuse_srcs: &mut Vec<FuseSource>,
    scope: SymbolId,
    net_lvalue: AstId<'a, NetLValueFlat<'a>>,
) -> FuseResult {
    let srcs_start_length = fuse_srcs.len();
    if try_ident_to_fuse_sources(
        ctx,
        gl,
        fuse_srcs,
        scope,
        net_lvalue.ident,
        net_lvalue.constant_exprs,
        net_lvalue.constant_range_expression.map(|r| *r),
        SignalSide::BlockingDrive,
    ) == FuseResult::Failed
    {
        return FuseResult::Failed;
    }

    convert_fuse_srcs_to_dsts(srcs_start_length, fuse_dsts, fuse_srcs)
}

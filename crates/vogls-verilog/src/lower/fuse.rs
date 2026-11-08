use std::cmp;

use slotmap::SlotMap;
use vogls_frontend::symbol_table::SymbolId;
use vogls_fuse_signals::{Driver, InputEdge};
use vogls_ir::{
    Bits, GlobalContext, SCALAR_VSIZE, Signal, SignalKey, SignalSlice, VSIZE_64, VectorSize,
};

use crate::ast::constant_expr::ConstantExpr;
use crate::ast::expr::{Expr, Replication};
use crate::ast::statement::NetLValueFlat;
use crate::ast::{AstId, AstIdRange, HIdent};
use crate::elaborate::{SignalKind, VSymbol};
use crate::lower::addressing::lower_addressing;
use crate::lower::try_resolve_hident;
use crate::number::Sign;

use super::addressing::{ConstantAddressingContext, RangeExpr};
use super::{Diagnostics, LowerContext, VType, eval_constant_expr, hident_span};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FuseFailure;
pub type FuseResult = Result<(), FuseFailure>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SiteType {
    Unsigned,
    Signed,
    Real,
}

pub enum SourceSiteContent {
    Constant(Bits),
    Signal {
        signal: SignalKey,
        slice: SignalSlice,
    },
}

pub struct SourceSite {
    pub content: SourceSiteContent,
    pub ty: SiteType,
}

pub struct DestinationSite {
    pub signal: SignalKey,
    pub slice: SignalSlice,
    pub ty: SiteType,
}

#[derive(Clone, Copy)]
pub enum SiteTerminal {
    Probe,
    BlockingDrive,
}

#[derive(Clone, Copy)]
pub enum RequiredSignalKind {
    None,
    Net,
}

impl SourceSite {
    pub fn bit_length(&self) -> VectorSize {
        match &self.content {
            SourceSiteContent::Constant(bits) => bits.size(),
            SourceSiteContent::Signal { signal: _, slice } => slice.width(),
        }
    }

    fn to_destination(&self) -> Option<DestinationSite> {
        match self.content {
            SourceSiteContent::Constant(_) => None,
            SourceSiteContent::Signal { signal, slice } => Some(DestinationSite {
                signal,
                slice,
                ty: self.ty,
            }),
        }
    }
}

impl From<VType> for SiteType {
    fn from(value: VType) -> Self {
        match value {
            VType::SignedNet(_) => SiteType::Signed,
            VType::UnsignedNet(_) => SiteType::Unsigned,
            VType::Real => SiteType::Real,
        }
    }
}

impl DestinationSite {
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

    // Source sites from least significant bits to most significant bits.
    srcs: &[SourceSite],

    // Destination sites from least significant bits to most significant bits.
    dsts: &[DestinationSite],
) -> FuseResult {
    // Prevent the fuse if any source or destination reads a signal out of bounds.
    if srcs.iter().any(|src| {
        let SourceSiteContent::Signal { signal, slice } = src.content else {
            return false;
        };

        slice.msb() >= gl.signals[signal].size.get()
    }) {
        return FuseResult::Err(FuseFailure);
    }
    if dsts
        .iter()
        .any(|dst| dst.slice.msb() >= gl.signals[dst.signal].size.get())
    {
        return FuseResult::Err(FuseFailure);
    }

    // Fast path and base case: both sides consist of 1 signal that has the same bit length and
    // type.
    if let [src] = srcs
        && let [dst] = dsts
        && src.bit_length() == dst.bit_length()
        && (src.ty == T::Real) == (dst.ty == T::Real)
    {
        let driver = match &src.content {
            SourceSiteContent::Constant(bits) => Driver::Constant(bits.clone()),
            SourceSiteContent::Signal { signal, slice } => {
                Driver::Signal(*signal, signal_slice_to_opt(&gl.signals, *signal, *slice))
            }
        };
        connections.push(InputEdge {
            driver,
            drivee: dst.signal,
            drivee_slice: signal_slice_to_opt(&gl.signals, dst.signal, dst.slice),
        });
        return FuseResult::Ok(());
    }

    use SiteType as T;
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
            return FuseResult::Err(FuseFailure);
        }

        let src_bit_length = src.bit_length();
        let dst_bit_length = dst.bit_length();

        debug_assert!(src_bit_offset < src_bit_length.get());
        debug_assert!(dst_bit_offset < dst_bit_length.get());

        let src_rem_bit_length = VectorSize::new(src_bit_length.get() - src_bit_offset).unwrap();
        let dst_rem_bit_length = VectorSize::new(dst_bit_length.get() - dst_bit_offset).unwrap();

        let step_bit_length = cmp::min(src_rem_bit_length, dst_rem_bit_length);

        let driver = match &src.content {
            SourceSiteContent::Constant(bits) => {
                Driver::Constant(bits.slicez(src_bit_offset, step_bit_length))
            }
            SourceSiteContent::Signal { signal, slice } => {
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
        return FuseResult::Err(FuseFailure);
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

    FuseResult::Ok(())
}

pub fn convert_src_sites_to_dsts(
    srcs_start_length: usize,
    dst_sites: &mut Vec<DestinationSite>,
    src_sites: &mut Vec<SourceSite>,
) -> FuseResult {
    let mut failed_conversion = false;
    let dsts_start_length = dst_sites.len();
    dst_sites.extend(src_sites.drain(srcs_start_length..).map(|src| {
        let Some(dst) = src.to_destination() else {
            failed_conversion = true;

            // Insert a dummy element.
            return DestinationSite {
                signal: SignalKey::default(),
                slice: SignalSlice::with_end(SCALAR_VSIZE),
                ty: SiteType::Unsigned,
            };
        };

        dst
    }));

    if failed_conversion {
        dst_sites.truncate(dsts_start_length);
        return FuseResult::Err(FuseFailure);
    }

    FuseResult::Ok(())
}

pub fn expr_to_src_sites<'a>(
    ctx: &LowerContext<'a, '_>,
    gl: &GlobalContext,
    fuse_srcs: &mut Vec<SourceSite>,
    scope: SymbolId,
    diagnostics: &mut Diagnostics,
    expr: AstId<'a, Expr<'a>>,
    side: SiteTerminal,
    required_signal_kind: RequiredSignalKind,
) -> Result<(), ()> {
    let mut error = false;
    let mut expr_stack = Vec::new();

    expr_stack.push(expr);

    while let Some(expr) = expr_stack.pop() {
        match &*expr {
            Expr::Ident(ident, exprs, range_expr) => {
                error |= ident_to_src_sites(
                    ctx,
                    gl,
                    fuse_srcs,
                    scope,
                    diagnostics,
                    *ident,
                    exprs.into_constant(),
                    *range_expr,
                    side,
                    required_signal_kind,
                )
                .is_err()
            }
            Expr::Sized(sized) => {
                let sized = &ctx.arenas.sized_numbers[sized.item.at];
                let ty = match sized.sign {
                    Sign::Unsigned => SiteType::Unsigned,
                    Sign::Signed => SiteType::Signed,
                };
                let value = sized.value.clone();
                fuse_srcs.push(SourceSite {
                    content: SourceSiteContent::Constant(value),
                    ty,
                });
            }
            Expr::Concatenation(exprs) => {
                // @TODO: Handle zero-sized replications.
                expr_stack.extend(exprs.iter());
            }
            Expr::Replication(Replication {
                constant_expr,
                exprs,
            }) => {
                let Ok(repetitions) = eval_constant_expr(
                    gl,
                    ctx.arenas,
                    &ctx.table,
                    scope,
                    diagnostics,
                    *constant_expr,
                    None,
                ) else {
                    error = true;
                    continue;
                };

                let Some(repetitions) = repetitions.as_integer().filter(|v| *v > 0) else {
                    diagnostics.not_yet_implemented(
                        ctx.arenas.get_span(*constant_expr),
                        "cannot be used for number of repetitions",
                    );
                    error = true;
                    continue;
                };

                for _ in 0..repetitions {
                    expr_stack.extend(exprs.iter());
                }
            }
            _ => {
                diagnostics.output_expr_not_allowed(ctx.arenas.get_span(expr));
                error = true;
            }
        }
    }

    if error {
        return Err(());
    }
    Ok(())
}

pub fn ident_to_src_sites<'a, R>(
    ctx: &LowerContext<'a, '_>,
    gl: &GlobalContext,
    fuse_srcs: &mut Vec<SourceSite>,
    scope: SymbolId,
    diagnostics: &mut Diagnostics,
    ident: HIdent<'_>,
    exprs: AstIdRange<'a, ConstantExpr<'a>>,
    range_expr: Option<R>,
    side: SiteTerminal,
    required_signal_kind: RequiredSignalKind,
) -> Result<(), ()>
where
    R: for<'b> Into<RangeExpr<ConstantAddressingContext<'a, 'b>>>,
{
    let symbol_id = try_resolve_hident(scope, &ctx.table, ctx.arenas, ident, diagnostics)?;
    let symbol = match &ctx.table[symbol_id].content {
        VSymbol::Signal(n) => n,
        _ => {
            diagnostics
                .not_yet_implemented(hident_span(ctx.arenas, ident), "cannot be used as signal");
            return Err(());
        }
    };

    match required_signal_kind {
        RequiredSignalKind::None => {}
        RequiredSignalKind::Net => {
            if symbol.kind != SignalKind::Net {
                diagnostics.not_yet_implemented(
                    hident_span(ctx.arenas, ident),
                    "only nets can be used here",
                );
                return Err(());
            }
        }
    }

    let signal = match side {
        SiteTerminal::Probe => symbol.terminals.probe_signal(),
        SiteTerminal::BlockingDrive => symbol.terminals.blocking_drive_signal(),
    };

    // Fast path: no part select or array indexing.
    if exprs.is_empty() && range_expr.is_none() && symbol.dims.is_empty() {
        let (ty, bit_length) = match symbol.ty {
            VType::SignedNet(bit_length) => (SiteType::Signed, bit_length),
            VType::UnsignedNet(bit_length) => (SiteType::Unsigned, bit_length),
            VType::Real => (SiteType::Real, VSIZE_64),
        };

        fuse_srcs.push(SourceSite {
            content: SourceSiteContent::Signal {
                signal,
                slice: SignalSlice::with_end(bit_length),
            },
            ty,
        });
        return Ok(());
    }

    let mut actx = ConstantAddressingContext {
        gl,
        arenas: ctx.arenas,
        table: &ctx.table,
        scope,
        diagnostics,
        loc: 0,
        _pd: std::marker::PhantomData,
    };

    let address = lower_addressing(
        &mut actx,
        symbol.ty.bit_length(),
        &symbol.dims,
        symbol.transform,
        exprs.iter(),
        range_expr.map(Into::into),
    )?;

    let ty = if symbol.ty.is_real() {
        SiteType::Real
    } else if address.is_unsigned {
        SiteType::Unsigned
    } else if symbol.ty.is_signed_net() {
        SiteType::Signed
    } else {
        SiteType::Unsigned
    };

    // Array element overflow.
    if matches!(address.array, Some((_, true))) {
        // @TODO: Warn.
        fuse_srcs.push(SourceSite {
            content: SourceSiteContent::Constant(Bits::new_unknown(address.output_width)),
            ty,
        });
        return Ok(());
    }

    // A bit- or part-select past the element reads x, not the neighbouring element.
    if let Some(elem_offset) = address.elem_offset
        && (elem_offset < 0
            || elem_offset + i64::from(address.output_width.get())
                > i64::from(symbol.ty.bit_length().get()))
    {
        // @TODO: Warn.
        fuse_srcs.push(SourceSite {
            content: SourceSiteContent::Constant(Bits::new_unknown(address.output_width)),
            ty,
        });
        return Ok(());
    }

    let Some(offset) = address.signal_offset_as_u32() else {
        diagnostics.not_yet_implemented(hident_span(ctx.arenas, ident), "overflow");
        return Err(());
    };

    let Some(output_slice) = SignalSlice::from_width(offset, address.output_width) else {
        diagnostics.not_yet_implemented(hident_span(ctx.arenas, ident), "overflow");
        return Err(());
    };

    fuse_srcs.push(SourceSite {
        content: SourceSiteContent::Signal {
            signal,
            slice: output_slice,
        },
        ty,
    });
    Ok(())
}

pub fn try_net_flat_lvalue_to_dsts<'a>(
    ctx: &LowerContext<'a, '_>,
    gl: &GlobalContext,
    dst_sites: &mut Vec<DestinationSite>,
    src_sites: &mut Vec<SourceSite>,
    scope: SymbolId,
    net_lvalue: AstId<'a, NetLValueFlat<'a>>,
) -> FuseResult {
    let srcs_start_length = src_sites.len();
    if ident_to_src_sites(
        ctx,
        gl,
        src_sites,
        scope,
        &mut Diagnostics::default(),
        net_lvalue.ident,
        net_lvalue.constant_exprs,
        net_lvalue.constant_range_expression.map(|r| *r),
        SiteTerminal::BlockingDrive,
        RequiredSignalKind::Net,
    )
    .is_err()
    {
        return Err(FuseFailure);
    };

    convert_src_sites_to_dsts(srcs_start_length, dst_sites, src_sites)
}

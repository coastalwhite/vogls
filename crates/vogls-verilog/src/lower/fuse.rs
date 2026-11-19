use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{Bits, GlobalContext, SCALAR_VSIZE, SignalKey, SignalSlice, VSIZE_64, VectorSize};

use crate::ast::constant_expr::ConstantExpr;
use crate::ast::expr::{Expr, Replication};
use crate::ast::statement::NetLValueFlat;
use crate::ast::{AstId, AstIdRange, HIdent};
use crate::elaborate::{SignalKind, VSymbol};
use crate::lower::addressing::lower_addressing;
use crate::lower::try_resolve_hident;
use crate::number::Sign;

use super::addressing::{ConstantAddressingContext, RangeExpr};
use super::expression::get_expr_type;
use super::{Diagnostics, LowerContext, VType, eval_constant_expr, hident_span};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FuseFailure;
pub type FuseResult = Result<(), FuseFailure>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SiteType {
    Unsigned,
    Signed,
    Real,
}

pub enum SourceSiteContent<'a> {
    Constant(Bits),
    Signal {
        signal: SignalKey,
        slice: SignalSlice,
        kind: SignalKind,
    },
    /// A read from out-of-bounds bits of a signal.
    OutOfBounds {
        signal: SignalKey,
        bit_length: VectorSize,
    },
    /// A read from `slice` of an expression that is not just wiring.
    ///
    /// The value of the expression is converted to `ty`.
    Expr {
        expr: AstId<'a, Expr<'a>>,
        scope: SymbolId,
        ty: VType,
        slice: SignalSlice,
    },
}

pub enum DestinationSiteContent {
    Signal {
        signal: SignalKey,
        slice: SignalSlice,
        kind: SignalKind,
    },

    /// A drive on out-of-bounds bits of a signal.
    OutOfBounds {
        signal: SignalKey,
        bit_length: VectorSize,
    },
}

pub struct SourceSite<'a> {
    pub content: SourceSiteContent<'a>,
    pub ty: SiteType,
}

pub struct DestinationSite {
    pub content: DestinationSiteContent,
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

impl SourceSite<'_> {
    pub fn bit_length(&self) -> VectorSize {
        match &self.content {
            SourceSiteContent::Constant(bits) => bits.size(),
            SourceSiteContent::Signal { slice, .. } => slice.width(),
            SourceSiteContent::Expr { slice, .. } => slice.width(),
            SourceSiteContent::OutOfBounds {
                signal: _,
                bit_length,
            } => *bit_length,
        }
    }

    fn to_destination(&self) -> Option<DestinationSite> {
        match self.content {
            SourceSiteContent::Constant(_) | SourceSiteContent::Expr { .. } => None,
            SourceSiteContent::Signal {
                signal,
                slice,
                kind,
            } => Some(DestinationSite {
                content: DestinationSiteContent::Signal {
                    signal,
                    slice,
                    kind,
                },
                ty: self.ty,
            }),
            SourceSiteContent::OutOfBounds { signal, bit_length } => Some(DestinationSite {
                content: DestinationSiteContent::OutOfBounds { signal, bit_length },
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
        match &self.content {
            DestinationSiteContent::Signal { slice, .. } => slice.width(),
            DestinationSiteContent::OutOfBounds {
                signal: _,
                bit_length,
            } => *bit_length,
        }
    }
}

pub fn convert_src_sites_to_dsts(
    srcs_start_length: usize,
    dst_sites: &mut Vec<DestinationSite>,
    src_sites: &mut Vec<SourceSite<'_>>,
) -> FuseResult {
    let mut failed_conversion = false;
    let dsts_start_length = dst_sites.len();
    dst_sites.extend(src_sites.drain(srcs_start_length..).map(|src| {
        let Some(dst) = src.to_destination() else {
            failed_conversion = true;

            // Insert a dummy element.
            return DestinationSite {
                content: DestinationSiteContent::OutOfBounds {
                    signal: SignalKey::default(),
                    bit_length: SCALAR_VSIZE,
                },
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

#[allow(clippy::too_many_arguments)]
pub fn expr_to_src_sites<'a>(
    ctx: &LowerContext<'a, '_>,
    gl: &GlobalContext,
    fuse_srcs: &mut Vec<SourceSite<'a>>,
    scope: SymbolId,
    diagnostics: &mut Diagnostics,
    expr: AstId<'a, Expr<'a>>,
    side: SiteTerminal,
    required_signal_kind: RequiredSignalKind,
) -> Result<(), ()> {
    let is_concatenation = matches!(&*expr, Expr::Concatenation(_) | Expr::Replication(_));
    let start = fuse_srcs.len();
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
                    Some(expr),
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

                // A zero replication contributes no bits.
                let Some(repetitions) = repetitions.as_integer().filter(|v| *v >= 0) else {
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
                error |= push_expr_site_of(ctx, gl, diagnostics, fuse_srcs, scope, expr).is_err();
            }
        }
    }

    if error {
        return Err(());
    }

    // The result of a concatenation is unsigned.
    if is_concatenation {
        for site in &mut fuse_srcs[start..] {
            if site.ty == SiteType::Signed {
                site.ty = SiteType::Unsigned;
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn ident_to_src_sites<'a, R>(
    ctx: &LowerContext<'a, '_>,
    gl: &GlobalContext,
    fuse_srcs: &mut Vec<SourceSite<'a>>,
    scope: SymbolId,
    diagnostics: &mut Diagnostics,
    expr: Option<AstId<'a, Expr<'a>>>,
    ident: HIdent<'_>,
    exprs: AstIdRange<'a, ConstantExpr<'a>>,
    range_expr: Option<R>,
    side: SiteTerminal,
    required_signal_kind: RequiredSignalKind,
) -> Result<(), ()>
where
    R: for<'b> Into<RangeExpr<ConstantAddressingContext<'a, 'b>>>,
{
    // Only values may be expressions, not destinations or bidirectional terminals.
    let expr = match required_signal_kind {
        RequiredSignalKind::None => expr,
        RequiredSignalKind::Net => None,
    };

    let symbol_id = try_resolve_hident(scope, &ctx.table, ctx.arenas, ident, diagnostics)?;
    let symbol = match &ctx.table[symbol_id].content {
        VSymbol::Signal(n) => n,
        VSymbol::Parameter(_) if let Some(expr) = expr => {
            let value = eval_constant_expr(
                gl,
                ctx.arenas,
                &ctx.table,
                scope,
                diagnostics,
                expr.into_constant(),
                None,
            )?;
            let ty = SiteType::from(value.ty());
            fuse_srcs.push(SourceSite {
                content: SourceSiteContent::Constant(value.into_bits()),
                ty,
            });
            return Ok(());
        }
        _ => {
            diagnostics
                .not_yet_implemented(hident_span(ctx.arenas, ident), "cannot be used as signal");
            return Err(());
        }
    };

    match required_signal_kind {
        RequiredSignalKind::None => {}
        RequiredSignalKind::Net => {
            if !symbol.kind.is_net() {
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
                kind: symbol.kind,
            },
            ty,
        });
        return Ok(());
    }

    // A select with an index that is not constant reads different bits over time, so it is an
    // expression. An index that is not constant fails to be addressed as a constant.
    let mut address_diagnostics = Diagnostics::default();
    let mut actx = ConstantAddressingContext {
        gl,
        arenas: ctx.arenas,
        table: &ctx.table,
        scope,
        diagnostics: match expr {
            Some(_) => &mut address_diagnostics,
            None => diagnostics,
        },
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
    );
    let address = match (address, expr) {
        (Ok(address), _) => address,
        (Err(()), Some(expr)) => {
            return push_expr_site_of(ctx, gl, diagnostics, fuse_srcs, scope, expr);
        }
        (Err(()), None) => return Err(()),
    };

    let ty = if symbol.ty.is_real() {
        SiteType::Real
    } else if address.is_unsigned {
        SiteType::Unsigned
    } else if symbol.ty.is_signed_net() {
        SiteType::Signed
    } else {
        SiteType::Unsigned
    };

    let elem_start = match address.array {
        // Array element overflow.
        Some((_, true)) => {
            // @TODO: Warn.
            fuse_srcs.push(SourceSite {
                content: SourceSiteContent::OutOfBounds {
                    signal,
                    bit_length: address.output_width,
                },
                ty,
            });
            return Ok(());
        }

        None => 0,
        // Unwrap here should never panic since the overflow is checked.
        Some((array_offset, _)) => u32::try_from(array_offset).unwrap(),
    };

    let mut elem_offset = address.elem_offset.unwrap_or(0);
    let mut rem_bit_length = address.output_width;

    // A bit- or part-select past the element reads x, not the neighbouring element.
    if elem_offset < 0 {
        // @TODO: Warn.
        let out_of_bounds_bit_length = VectorSize::new(
            elem_offset
                .abs_diff(0)
                .min(address.output_width.get() as u64) as u32,
        )
        .unwrap();
        fuse_srcs.push(SourceSite {
            content: SourceSiteContent::OutOfBounds {
                signal,
                bit_length: out_of_bounds_bit_length,
            },
            ty,
        });

        let Some(bit_length) =
            VectorSize::new(rem_bit_length.get() - out_of_bounds_bit_length.get())
        else {
            return Ok(());
        };

        elem_offset = 0;
        rem_bit_length = bit_length;
    };

    // Check that at-least 1 bit is in the signal's bounds.
    let elem_size = symbol.ty.bit_length();
    let Some(slice) = u32::try_from(elem_offset).ok().and_then(|e| {
        let abs_elem_offset = elem_start.checked_add(e)?;
        let rem_elem_size = VectorSize::new(elem_size.get().saturating_sub(e))?;
        let in_bounds_size = rem_bit_length.min(rem_elem_size);
        SignalSlice::from_width(abs_elem_offset, in_bounds_size)
    }) else {
        fuse_srcs.push(SourceSite {
            content: SourceSiteContent::OutOfBounds {
                signal,
                bit_length: rem_bit_length,
            },
            ty,
        });
        return Ok(());
    };

    fuse_srcs.push(SourceSite {
        content: SourceSiteContent::Signal {
            signal,
            slice,
            kind: symbol.kind,
        },
        ty,
    });

    if let Some(bit_length) = VectorSize::new(rem_bit_length.get() - slice.width().get()) {
        // @TODO: Warn.
        fuse_srcs.push(SourceSite {
            content: SourceSiteContent::OutOfBounds { signal, bit_length },
            ty,
        });
    }

    Ok(())
}

pub fn net_flat_lvalue_to_dsts<'a>(
    ctx: &LowerContext<'a, '_>,
    gl: &GlobalContext,
    dst_sites: &mut Vec<DestinationSite>,
    src_sites: &mut Vec<SourceSite<'a>>,
    diagnostics: &mut Diagnostics,
    scope: SymbolId,
    net_lvalue: AstId<'a, NetLValueFlat<'a>>,
) -> Result<(), ()> {
    let srcs_start_length = src_sites.len();
    ident_to_src_sites(
        ctx,
        gl,
        src_sites,
        scope,
        diagnostics,
        None,
        net_lvalue.ident,
        net_lvalue.constant_exprs,
        net_lvalue.constant_range_expression.map(|r| *r),
        SiteTerminal::BlockingDrive,
        RequiredSignalKind::Net,
    )?;

    // Signals always convert to destinations.
    if convert_src_sites_to_dsts(srcs_start_length, dst_sites, src_sites).is_err() {
        diagnostics.output_expr_not_allowed(ctx.arenas.get_span(net_lvalue));
        return Err(());
    }

    Ok(())
}

fn push_expr_site_of<'a>(
    ctx: &LowerContext<'a, '_>,
    gl: &GlobalContext,
    diagnostics: &mut Diagnostics,
    fuse_srcs: &mut Vec<SourceSite<'a>>,
    scope: SymbolId,
    expr: AstId<'a, Expr<'a>>,
) -> Result<(), ()> {
    let ty = get_expr_type(gl, ctx.arenas, &ctx.table, scope, diagnostics, expr)?;
    fuse_srcs.push(SourceSite {
        content: SourceSiteContent::Expr {
            expr,
            scope,
            ty,
            slice: SignalSlice::with_end(ty.bit_length()),
        },
        ty: SiteType::from(ty),
    });
    Ok(())
}

/// Restricts `srcs`, given from least to most significant bit, to the bits in `slice`.
pub fn slice_src_sites(srcs: &mut Vec<SourceSite<'_>>, slice: SignalSlice) -> Result<(), ()> {
    // Expression slice is [lo, hi)
    let (lo, hi) = (slice.lsb(), slice.msb() + 1);

    let mut failed = false;
    let mut bit_offset = 0u32;

    srcs.retain_mut(|src| {
        let src_offset = bit_offset;
        bit_offset += src.bit_length().get();

        // Entire source is included.
        if lo <= src_offset && hi >= bit_offset {
            return true;
        }

        // Source slice is [src_lo, src_hi)
        let src_lo = lo.max(src_offset);
        let src_hi = hi.min(bit_offset);

        // Entire source is excluded.
        if src_lo >= src_hi {
            return false;
        }

        // We cannot partially slice a real.
        failed |= src.ty == SiteType::Real;

        let sub = SignalSlice::from_width(
            src_lo - src_offset,
            VectorSize::new(src_hi - src_lo).unwrap(),
        )
        .unwrap();
        match &mut src.content {
            SourceSiteContent::Constant(bits) => *bits = bits.slicez(sub.lsb(), sub.width()),
            SourceSiteContent::Signal { slice, .. } | SourceSiteContent::Expr { slice, .. } => {
                *slice = slice.subslice(sub).unwrap();
            }
            SourceSiteContent::OutOfBounds { bit_length, .. } => *bit_length = sub.width(),
        }
        true
    });

    if failed {
        return Err(());
    }

    Ok(())
}

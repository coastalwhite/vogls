//! Insertion of assignments between sites into the drive graph.

use std::cmp;

use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{Bits, GlobalContext, SCALAR_VSIZE, SignalKey, SignalSlice, VectorSize};
use vogls_utils::TableKey;

use super::{AssignmentKey, DriveEdgeKey, DriveNode, DriveNodeKey, DriverResolution, ExprSource};
use crate::ast::AstId;
use crate::ast::expr::Expr;
use crate::elaborate::SignalKind;
use crate::lower::VType;
use crate::lower::fuse::{
    DestinationSite, DestinationSiteContent, FuseFailure, FuseResult, SiteType, SourceSite,
    SourceSiteContent,
};

impl<'a> DriverResolution<'a> {
    /// Adds the assignment of `srcs` to `dsts` as direct connections.
    ///
    /// Both sides are given from least to most significant bits and are connected bit for bit. A
    /// wider destination is zero-extended, a wider source is truncated. Out-of-bounds source bits
    /// read as `x` and out-of-bounds destination bits are dropped.
    ///
    /// If `context` is given, the sources are the self-determined sites of its expression, which
    /// is context-determined by its type. If the expression is not just wiring, or its conversion
    /// to the destinations is not wiring, i.e. a sign-extension or a conversion between a real and
    /// a net, the expression is evaluated as a whole with the context type instead.
    ///
    /// Without a `context`, this fails without adding anything if the assignment is not just
    /// wiring.
    pub fn push_sites(
        &mut self,
        gl: &GlobalContext,

        // Source sites from least significant bits to most significant bits.
        srcs: &[SourceSite<'a>],

        // Destination sites from least significant bits to most significant bits.
        dsts: &[DestinationSite],

        context: Option<(AstId<'a, Expr<'a>>, SymbolId, VType)>,
    ) -> FuseResult {
        use SiteType as T;

        if cfg!(debug_assertions) {
            for src in srcs {
                let SourceSiteContent::Signal { signal, slice, .. } = src.content else {
                    continue;
                };
                assert!(slice.msb() < gl.signals[signal].size.get());
            }
            for dst in dsts {
                let DestinationSiteContent::Signal { signal, slice, .. } = dst.content else {
                    continue;
                };
                assert!(slice.msb() < gl.signals[signal].size.get());
            }
        }

        // An expression is evaluated with the context, e.g. `a + b` with the width of the context
        // to keep its carry.
        if let Some(context) = context
            && let [
                SourceSite {
                    content: SourceSiteContent::Expr { .. },
                    ..
                },
            ] = srcs
        {
            return self.push_context_expr(gl, srcs, dsts, context);
        }

        // A signed source that is narrower than the destinations is sign-extended by its most
        // significant bit, an unsigned source is zero-extended.
        let src_bit_length = srcs
            .iter()
            .map(|s| u64::from(s.bit_length().get()))
            .sum::<u64>();
        let dst_bit_length = dsts
            .iter()
            .map(|d| u64::from(d.bit_length().get()))
            .sum::<u64>();
        let sign_src = srcs
            .last()
            .filter(|src| src.ty == T::Signed && src_bit_length < dst_bit_length);

        // All expressions and sign-extensions of the sources are evaluated together, as one
        // assignment.
        let assignment = (srcs
            .iter()
            .any(|src| matches!(src.content, SourceSiteContent::Expr { .. }))
            || sign_src.is_some_and(|src| matches!(src.content, SourceSiteContent::Signal { .. })))
        .then(|| self.new_assignment());

        // Fast path and base case: both sides consist of 1 site that has the same bit length and
        // type.
        if let [src] = srcs
            && let [dst] = dsts
            && src.bit_length() == dst.bit_length()
            && (src.ty == T::Real) == (dst.ty == T::Real)
        {
            self.push_site(gl, src, 0, dst, 0, src.bit_length(), assignment);
            return Ok(());
        }

        let start_edges_len = self.edges.len();

        let mut src_i = 0;
        let mut dst_i = 0;
        let mut src_bit_offset = 0u32;
        let mut dst_bit_offset = 0u32;
        while let Some(src) = srcs.get(src_i)
            && let Some(dst) = dsts.get(dst_i)
        {
            // Don't do any kind of fusing if only one side is a real.
            if (src.ty == T::Real) ^ (dst.ty == T::Real) {
                for edge in start_edges_len..self.edges.len() {
                    self.remove_edge(DriveEdgeKey::from_usize(edge).unwrap());
                }
                return match context {
                    Some(context) => self.push_context_expr(gl, srcs, dsts, context),
                    None => Err(FuseFailure),
                };
            }

            let src_bit_length = src.bit_length();
            let dst_bit_length = dst.bit_length();

            debug_assert!(src_bit_offset < src_bit_length.get());
            debug_assert!(dst_bit_offset < dst_bit_length.get());

            let src_rem_bit_length =
                VectorSize::new(src_bit_length.get() - src_bit_offset).unwrap();
            let dst_rem_bit_length =
                VectorSize::new(dst_bit_length.get() - dst_bit_offset).unwrap();

            let step_bit_length = cmp::min(src_rem_bit_length, dst_rem_bit_length);

            self.push_site(
                gl,
                src,
                src_bit_offset,
                dst,
                dst_bit_offset,
                step_bit_length,
                assignment,
            );

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

        // The remaining destination bits, as `(destination, offset, bit length)`.
        let mut rest = Vec::new();
        for dst in &dsts[dst_i..] {
            let bit_length = VectorSize::new(dst.bit_length().get() - dst_bit_offset).unwrap();
            rest.push((dst, dst_bit_offset, bit_length));
            dst_bit_offset = 0;
        }
        match sign_src {
            Some(src) => self.push_sign_extension(gl, src, &rest, assignment),
            None => {
                for &(dst, dst_offset, bit_length) in &rest {
                    let Some((signal, kind, slice)) = dst_slice(dst, dst_offset, bit_length) else {
                        continue;
                    };
                    let edge =
                        self.push_constant(gl, Bits::new_zeroed(bit_length), (signal, kind), slice);
                    self.edges[edge].assignment = assignment;
                }
            }
        }

        Ok(())
    }

    /// Pushes the expression of `context` as a whole, evaluated with and converted to its context
    /// type, in place of its self-determined sites `srcs`.
    fn push_context_expr(
        &mut self,
        gl: &GlobalContext,
        srcs: &[SourceSite<'a>],
        dsts: &[DestinationSite],
        (expr, scope, context_type): (AstId<'a, Expr<'a>>, SymbolId, VType),
    ) -> FuseResult {
        let is_real = srcs.iter().any(|src| src.ty == SiteType::Real);
        let bit_length = srcs.iter().map(|src| src.bit_length().get()).sum::<u32>();
        let ty = match context_type {
            VType::Real => VType::Real,
            _ if is_real => context_type,
            _ => context_type.resize_net_to(cmp::max(
                context_type.bit_length(),
                VectorSize::new(bit_length).unwrap_or(context_type.bit_length()),
            )),
        };
        let site = SourceSite {
            content: SourceSiteContent::Expr {
                expr,
                scope,
                ty,
                slice: SignalSlice::with_end(ty.bit_length()),
            },
            ty: SiteType::from(ty),
        };
        self.push_sites(gl, std::slice::from_ref(&site), dsts, None)
    }

    /// Connects `bit_length` bits of `src` at `src_offset` to `dst` at `dst_offset`.
    #[allow(clippy::too_many_arguments)]
    fn push_site(
        &mut self,
        gl: &GlobalContext,
        src: &SourceSite<'a>,
        src_offset: u32,
        dst: &DestinationSite,
        dst_offset: u32,
        bit_length: VectorSize,
        assignment: Option<AssignmentKey>,
    ) {
        // Out-of-bounds destination bits are not driven.
        let Some((dst_signal, dst_kind, dst_slice)) = dst_slice(dst, dst_offset, bit_length) else {
            return;
        };

        let edge = match &src.content {
            SourceSiteContent::Constant(bits) => {
                let value = bits.slicez(src_offset, bit_length);
                self.push_constant(gl, value, (dst_signal, dst_kind), dst_slice)
            }
            SourceSiteContent::Signal {
                signal,
                slice,
                kind,
            } => {
                let src_slice = slice
                    .subslice(SignalSlice::from_width(src_offset, bit_length).unwrap())
                    .unwrap();
                let from = self.signal_node(gl, *signal, *kind);
                let to = self.signal_node(gl, dst_signal, dst_kind);
                self.push_edge((from, src_slice.lsb()), (to, dst_slice.lsb()), bit_length)
            }
            SourceSiteContent::OutOfBounds { .. } => self.push_constant(
                gl,
                Bits::new_unknown(bit_length),
                (dst_signal, dst_kind),
                dst_slice,
            ),
            SourceSiteContent::Expr {
                expr,
                scope,
                ty,
                slice,
            } => {
                let assignment = assignment.expect("sources with expressions have an assignment");
                let src_slice = slice
                    .subslice(SignalSlice::from_width(src_offset, bit_length).unwrap())
                    .unwrap();
                let from = self.expr_node(*expr, *scope, *ty, assignment);
                let to = self.signal_node(gl, dst_signal, dst_kind);
                self.push_edge((from, src_slice.lsb()), (to, dst_slice.lsb()), bit_length)
            }
        };

        // Mark the edge as part of the assignment of the expressions, if there are any.
        self.edges[edge].assignment = assignment;
    }

    /// Returns the node of the expression `expr` of `assignment`, evaluated with `ty`.
    fn expr_node(
        &mut self,
        expr: AstId<'a, Expr<'a>>,
        scope: SymbolId,
        ty: VType,
        assignment: AssignmentKey,
    ) -> DriveNodeKey {
        let address = std::ptr::from_ref(expr.node) as usize;
        *self
            .expr_lut
            .entry((assignment, address, ty))
            .or_insert_with(|| {
                self.nodes.insert(DriveNode::Expr {
                    source: ExprSource::Assign {
                        expr,
                        scope,
                        assignment,
                        ty,
                    },
                    bit_length: ty.bit_length(),
                    fst_out: None,
                    fst_read_in: None,
                })
            })
    }

    /// Drives the destination bits `rest`, `(destination, offset, bit length)`, with the most
    /// significant bit of `src`.
    ///
    /// A signal is sign-extended by an [`ExprSource::SignExtend`] that drives all of `rest`, an
    /// expression by widening its node, which then sign-extends its value.
    fn push_sign_extension(
        &mut self,
        gl: &GlobalContext,
        src: &SourceSite<'a>,
        rest: &[(&DestinationSite, u32, VectorSize)],
        assignment: Option<AssignmentKey>,
    ) {
        let rest = rest
            .iter()
            .filter_map(|&(dst, offset, bit_length)| dst_slice(dst, offset, bit_length))
            .collect::<Vec<_>>();
        let Some(rest_bit_length) =
            VectorSize::new(rest.iter().map(|(_, _, slice)| slice.width().get()).sum())
        else {
            return;
        };
        let top = src.bit_length().get() - 1;

        // The node whose bits from `offset` on drive `rest`.
        let (from, offset) = match &src.content {
            SourceSiteContent::Signal { signal, slice, .. } => {
                let assignment = assignment.expect("sign-extended signals have an assignment");
                let from = self.nodes.insert(DriveNode::Expr {
                    source: ExprSource::SignExtend {
                        signal: *signal,
                        bit: slice.lsb() + top,
                        assignment,
                    },
                    bit_length: rest_bit_length,
                    fst_out: None,
                    fst_read_in: None,
                });
                (from, 0)
            }
            SourceSiteContent::Expr {
                expr,
                scope,
                ty,
                slice,
            } => {
                // Signed expression sites are the entire value of their expression, so its top bit
                // is the sign.
                assert_eq!(
                    slice.lsb() + top,
                    ty.bit_length().get() - 1,
                    "the sign of an expression is its most significant bit"
                );
                let assignment = assignment.expect("sources with expressions have an assignment");
                let from = self.expr_node(*expr, *scope, *ty, assignment);
                let DriveNode::Expr { bit_length, .. } = &mut self.nodes[from] else {
                    unreachable!();
                };
                *bit_length =
                    VectorSize::new(ty.bit_length().get() + rest_bit_length.get()).unwrap();
                (from, ty.bit_length().get())
            }
            SourceSiteContent::Constant(_) | SourceSiteContent::OutOfBounds { .. } => {
                let sign = match &src.content {
                    SourceSiteContent::Constant(bits) => bits.slicez(top, SCALAR_VSIZE),
                    _ => Bits::new_unknown(SCALAR_VSIZE),
                };
                for (signal, kind, slice) in rest {
                    let value = sign.sign_extend(slice.width());
                    let edge = self.push_constant(gl, value, (signal, kind), slice);
                    self.edges[edge].assignment = assignment;
                }
                return;
            }
        };

        let mut at = offset;
        for (signal, kind, slice) in rest {
            let to = self.signal_node(gl, signal, kind);
            let edge = self.push_edge((from, at), (to, slice.lsb()), slice.width());
            self.edges[edge].assignment = assignment;
            at += slice.width().get();
        }
    }

    /// Drives `signal[slice]` with a constant.
    fn push_constant(
        &mut self,
        gl: &GlobalContext,
        value: Bits,
        (signal, kind): (SignalKey, SignalKind),
        slice: SignalSlice,
    ) -> DriveEdgeKey {
        debug_assert_eq!(value.size(), slice.width());
        let from = self.nodes.insert(DriveNode::Constant {
            value,
            fst_out: None,
            fst_read_out: None,
        });
        let to = self.signal_node(gl, signal, kind);
        self.push_edge((from, 0), (to, slice.lsb()), slice.width())
    }
}

/// The bits `offset +: bit_length` of `dst`, or `None` if they are out of bounds.
fn dst_slice(
    dst: &DestinationSite,
    offset: u32,
    bit_length: VectorSize,
) -> Option<(SignalKey, SignalKind, SignalSlice)> {
    let DestinationSiteContent::Signal {
        signal,
        slice,
        kind,
    } = dst.content
    else {
        return None;
    };
    let slice = slice
        .subslice(SignalSlice::from_width(offset, bit_length).unwrap())
        .unwrap();
    Some((signal, kind, slice))
}

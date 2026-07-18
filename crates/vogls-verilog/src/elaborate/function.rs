use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{ConnectionDirection, GlobalContext, INTEGER_VSIZE, SCALAR_VSIZE, SignalKey};

use crate::ast::module::{
    FunctionDeclaration, FunctionRangeOrType, TaskDeclaration, TaskPortItemContent,
    TfInputDeclaration, TfType,
};
use crate::elaborate::{NetSymbol, VectorTransform, evaluate_net_msb_lsb};
use crate::lower::{Diagnostics, LowerContext, VType};

use super::VSymbol;

pub fn elaborate_fn<'a>(
    gl: &mut GlobalContext,
    symbol: SymbolId,
    ctx: &mut LowerContext<'a, '_>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let VSymbol::Function(i) = &ctx.table[symbol].content else {
        unreachable!();
    };

    let parent = ctx.table[symbol].parent().unwrap();
    let id = i.ast_id;
    let id = ctx.table_ast_refs.fns[id];
    let FunctionDeclaration {
        ident,
        tf_input_decls,
        statement: _,
        block_item_decls: _,
        range_or_type,
        ..
    } = &*id;

    let (transform, output_ty) = match &**range_or_type {
        FunctionRangeOrType::Unsigned(None) => {
            (VectorTransform::default(), VType::UnsignedNet(SCALAR_VSIZE))
        }
        FunctionRangeOrType::Signed(None) => {
            (VectorTransform::default(), VType::SignedNet(SCALAR_VSIZE))
        }
        FunctionRangeOrType::Unsigned(Some(range)) => {
            let (transform, size) =
                evaluate_net_msb_lsb(gl, ctx.arenas, *range, parent, &ctx.table, diagnostics)?;
            (transform, VType::UnsignedNet(size))
        }
        FunctionRangeOrType::Signed(Some(range)) => {
            let (transform, size) =
                evaluate_net_msb_lsb(gl, ctx.arenas, *range, parent, &ctx.table, diagnostics)?;
            (transform, VType::SignedNet(size))
        }
        FunctionRangeOrType::Integer => {
            (VectorTransform::default(), VType::SignedNet(INTEGER_VSIZE))
        }
        FunctionRangeOrType::Real | FunctionRangeOrType::Realtime | FunctionRangeOrType::Time => {
            diagnostics.not_yet_implemented(
                ctx.arenas.get_span(id),
                "real / time / realtime function output",
            );
            return Err(());
        }
    };

    let net = super::new_net(
        gl,
        ctx.logic_mode,
        ctx.arenas,
        &output_ty,
        &[],
        *ident,
        None,
    );
    let output_key = net.ba;
    if ctx
        .table
        .insert(
            ident.item.0,
            symbol,
            ctx.arenas.get_item_span(*ident),
            VSymbol::Net(NetSymbol {
                ty: output_ty,
                dims: [].into(),
                net,
                transform,
                port_idx: None,
            }),
        )
        .is_err()
    {
        diagnostics.duplicate_definition(ctx.arenas, *ident);
        return Err(());
    }

    let mut inputs = Vec::<(SignalKey, VType)>::new();
    for input_decl in tf_input_decls.iter() {
        let TfInputDeclaration {
            tf_type,
            port_identifiers,
        } = &*input_decl;
        for ident in port_identifiers.iter() {
            let (ty, transform) = match tf_type {
                TfType::Net {
                    reg: _,
                    signed,
                    range,
                } => {
                    let (transform, width) = match range {
                        None => (VectorTransform::default(), SCALAR_VSIZE),
                        // @TODO: Better error
                        Some(range) => evaluate_net_msb_lsb(
                            gl,
                            ctx.arenas,
                            *range,
                            symbol,
                            &ctx.table,
                            diagnostics,
                        )?,
                    };
                    (VType::net(width, *signed), transform)
                }
                TfType::Integer => (VType::SignedNet(INTEGER_VSIZE), VectorTransform::default()),
                TfType::Real | TfType::Realtime | TfType::Time => todo!(),
            };
            let ident = ctx.arenas.to_item(ident);
            let origin = ctx.arenas.get_item_span(ident);
            let net = super::new_net(gl, ctx.logic_mode, ctx.arenas, &ty, &[], ident, None);
            let signal = net.ba;
            if ctx
                .table
                .insert(
                    ident.item.0,
                    symbol,
                    origin,
                    VSymbol::Net(NetSymbol {
                        ty,
                        dims: [].into(),
                        net,
                        transform,
                        port_idx: None,
                    }),
                )
                .is_err()
            {
                diagnostics.duplicate_definition(ctx.arenas, ident);
                return Err(());
            }
            inputs.push((signal, ty));
        }
    }

    let VSymbol::Function(i) = &mut ctx.table[symbol].content else {
        unreachable!();
    };
    i.inputs = inputs;
    i.output = output_key;
    i.output_ty = output_ty;

    Ok(())
}

pub fn elaborate_task<'a>(
    gl: &mut GlobalContext,
    symbol: SymbolId,
    ctx: &mut LowerContext<'a, '_>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let VSymbol::Task(i) = &ctx.table[symbol].content else {
        unreachable!();
    };

    let parent = ctx.table[symbol].parent().unwrap();
    let id = i.ast_id;
    let TaskDeclaration {
        task_ports,
        block_item_decls: _,
        statement_or_null: _,
        ..
    } = &*ctx.table_ast_refs.tasks[id];

    let mut io = Vec::<(SignalKey, ConnectionDirection, VType)>::new();
    for decl in task_ports.iter() {
        use ConnectionDirection as D;
        let (tf_type, direction, port_identifiers) = match decl.content {
            TaskPortItemContent::Input(d) => (d.tf_type, D::In, d.port_identifiers),
            TaskPortItemContent::Output(d) => (d.tf_type, D::Out, d.port_identifiers),
            TaskPortItemContent::Inout(d) => (d.tf_type, D::Both, d.port_identifiers),
        };
        for ident in port_identifiers.iter() {
            let (ty, transform) = match tf_type {
                TfType::Net {
                    reg: _,
                    signed,
                    range,
                } => {
                    let (transform, width) = match range {
                        None => (VectorTransform::default(), SCALAR_VSIZE),
                        // @TODO: Better error
                        Some(range) => evaluate_net_msb_lsb(
                            gl,
                            ctx.arenas,
                            range,
                            parent,
                            &ctx.table,
                            diagnostics,
                        )?,
                    };
                    (VType::net(width, signed), transform)
                }
                TfType::Integer => (VType::SignedNet(INTEGER_VSIZE), VectorTransform::default()),
                TfType::Real | TfType::Realtime | TfType::Time => todo!(),
            };
            let ident = ctx.arenas.to_item(ident);
            let origin = ctx.arenas.get_item_span(ident);
            let net = super::new_net(gl, ctx.logic_mode, ctx.arenas, &ty, &[], ident, None);
            let signal = net.ba;
            if ctx
                .table
                .insert(
                    ident.item.0,
                    symbol,
                    origin,
                    VSymbol::Net(NetSymbol {
                        ty,
                        dims: [].into(),
                        net,
                        transform,
                        port_idx: None,
                    }),
                )
                .is_err()
            {
                diagnostics.duplicate_definition(ctx.arenas, ident);
                return Err(());
            }
            io.push((signal, direction, ty));
        }
    }

    let VSymbol::Task(i) = &mut ctx.table[symbol].content else {
        unreachable!();
    };
    i.io = io;

    Ok(())
}

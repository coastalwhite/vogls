use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{
    ConnectionDirection, GlobalContext, INTEGER_VSIZE, SCALAR_VSIZE, SignalKey,
};

use crate::ast::AstIdRange;
use crate::ast::module::{
    FunctionDeclaration, FunctionRangeOrType, TaskDeclaration, TaskPortItemContent,
    TfInputDeclaration, TfType,
};
use crate::elaborate::NetSymbol;
use crate::lower::{Diagnostics, EvalScope, VType, evaluate_range};
use crate::parser::AstArenas;

use super::{VSymbol, VSymbolTable, eval_constant_range};

pub fn elaborate_fn<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    symbol: SymbolId,
    table: &mut VSymbolTable,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let VSymbol::Function(i) = &table[symbol].content else {
        unreachable!();
    };

    let parent = table[symbol].parent().unwrap();
    let id = i.ast_id;
    let FunctionDeclaration {
        ident,
        tf_input_decls,
        statement: _,
        block_item_decls: _,
        range_or_type,
        ..
    } = arenas.get(id);

    let (_, _, output_ty) = match arenas.get(*range_or_type) {
        FunctionRangeOrType::Unsigned(None) => (0, 0, VType::UnsignedNet(SCALAR_VSIZE)),
        FunctionRangeOrType::Signed(None) => (0, 0, VType::SignedNet(SCALAR_VSIZE)),
        FunctionRangeOrType::Unsigned(Some(range)) => {
            let (msb, lsb, size) =
                eval_constant_range(gl, arenas, parent, table, diagnostics, *range)?;
            (msb, lsb, VType::UnsignedNet(size))
        }
        FunctionRangeOrType::Signed(Some(range)) => {
            let (msb, lsb, size) =
                eval_constant_range(gl, arenas, parent, table, diagnostics, *range)?;
            (msb, lsb, VType::SignedNet(size))
        }
        FunctionRangeOrType::Integer => (31, 0, VType::SignedNet(INTEGER_VSIZE)),
        FunctionRangeOrType::Real | FunctionRangeOrType::Realtime | FunctionRangeOrType::Time => {
            diagnostics.not_yet_implemented(
                arenas.get_span(id),
                "real / time / realtime function output",
            );
            return Err(());
        }
    };

    let output_key = super::new_signal(gl, arenas, &output_ty, &[], *ident);
    if table
        .insert(
            ident.item.0,
            symbol,
            arenas.get_item_span(*ident),
            VSymbol::Net(NetSymbol {
                ty: output_ty,
                dims: [].into(),
                signal: output_key,
                nba: None,
                port_idx: None,
            }),
        )
        .is_err()
    {
        diagnostics.duplicate_definition(arenas, *ident);
        return Err(());
    }

    let mut inputs = Vec::<(SignalKey, VType)>::new();
    for input_decl in tf_input_decls.iter() {
        let TfInputDeclaration {
            tf_type,
            port_identifiers,
        } = arenas.get(input_decl);
        for ident in port_identifiers.iter() {
            let ty = match tf_type {
                TfType::Net {
                    reg: _,
                    signed,
                    range,
                } => {
                    let (_, _, width) = match range {
                        None => (0, 0, SCALAR_VSIZE),
                        // @TODO: Better error
                        Some(range) => evaluate_range(
                            gl,
                            arenas,
                            EvalScope { table, key: symbol },
                            diagnostics,
                            *range,
                        )
                        .unwrap(),
                    };
                    VType::net(width, *signed)
                }
                TfType::Integer => VType::SignedNet(INTEGER_VSIZE),
                TfType::Real | TfType::Realtime | TfType::Time => todo!(),
            };
            let ident = arenas.to_item(ident);
            let origin = arenas.get_item_span(ident);
            let signal = super::new_signal(gl, arenas, &ty, &[], ident);
            if table
                .insert(
                    ident.item.0,
                    symbol,
                    origin,
                    VSymbol::Net(NetSymbol {
                        ty,
                        dims: [].into(),
                        signal,
                        nba: None,
                        port_idx: None,
                    }),
                )
                .is_err()
            {
                diagnostics.duplicate_definition(arenas, ident);
                return Err(());
            }
            inputs.push((signal, ty));
        }
    }

    let VSymbol::Function(i) = &mut table[symbol].content else {
        unreachable!();
    };
    i.inputs = inputs;
    i.output = output_key;
    i.output_ty = output_ty;

    Ok(())
}

pub fn elaborate_task<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    symbol: SymbolId,
    table: &mut VSymbolTable,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let VSymbol::Task(i) = &table[symbol].content else {
        unreachable!();
    };

    let parent = table[symbol].parent().unwrap();
    let id = i.ast_id;
    let TaskDeclaration {
        task_ports,
        block_item_decls: _,
        statement_or_null: _,
        ..
    } = arenas.get(id);

    let mut io = Vec::<(SignalKey, ConnectionDirection, VType)>::new();
    for decl in task_ports.iter() {
        use ConnectionDirection as D;
        let (tf_type, direction, port_identifiers) = match arenas.get(decl).content {
            TaskPortItemContent::Input(d) => (d.tf_type, D::In, d.port_identifiers),
            TaskPortItemContent::Output(d) => (d.tf_type, D::Out, d.port_identifiers),
            TaskPortItemContent::Inout(d) => (d.tf_type, D::Both, d.port_identifiers),
        };
        for ident in port_identifiers.iter() {
            let ty = match tf_type {
                TfType::Net {
                    reg: _,
                    signed,
                    range,
                } => {
                    let (_, _, width) = match range {
                        None => (0, 0, SCALAR_VSIZE),
                        // @TODO: Better error
                        Some(range) => {
                            eval_constant_range(gl, arenas, parent, table, diagnostics, range)?
                        }
                    };
                    VType::net(width, signed)
                }
                TfType::Integer => VType::SignedNet(INTEGER_VSIZE),
                TfType::Real | TfType::Realtime | TfType::Time => todo!(),
            };
            let ident = arenas.to_item(ident);
            let origin = arenas.get_item_span(ident);
            let signal = super::new_signal(gl, arenas, &ty, &[], ident);
            if table
                .insert(
                    ident.item.0,
                    symbol,
                    origin,
                    VSymbol::Net(NetSymbol {
                        ty,
                        dims: [].into(),
                        signal,
                        nba: None,
                        port_idx: None,
                    }),
                )
                .is_err()
            {
                diagnostics.duplicate_definition(arenas, ident);
                return Err(());
            }
            io.push((signal, direction, ty));
        }
    }

    let VSymbol::Task(i) = &mut table[symbol].content else {
        unreachable!();
    };
    i.io = io;

    Ok(())
}

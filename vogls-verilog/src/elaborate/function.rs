use slotmap::SlotMap;
use vogls_ir::{INTEGER_VSIZE, SCALAR_VSIZE, Signal, SignalKey};

use crate::ast::AstIdRange;
use crate::ast::module::{FunctionDeclaration, TaskDeclaration, TaskPortItemContent, TfInputDeclaration, TfType};
use crate::hierarchy::{HierarchyItem, HierarchyNet, ScopeBuilder};
use crate::lower::{Diagnostics, VType, evaluate_range};
use crate::parser::AstArenas;

pub fn elaborate_fn<'a>(
    signals: &mut SlotMap<SignalKey, Signal>,
    arenas: &'a AstArenas,
    builder: &mut ScopeBuilder<'a>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let HierarchyItem::Function(i) = builder.hierarchy.symbols[builder.key.as_idx()] else {
        unreachable!();
    };
    let id = builder.hierarchy.functions[i].ast;
    let FunctionDeclaration {
        tf_input_decls,
        statement,
        block_item_decls,
        ..
    } = arenas.get(id);

    if !block_item_decls.is_empty() {
        diagnostics.not_yet_implemented(arenas.get_span(id), "block item decls");
        return Err(());
    }

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
                        Some(range) => {
                            evaluate_range(arenas, builder.eval_scope(), diagnostics, *range)
                                .unwrap()
                        }
                    };
                    VType::net(width, *signed)
                }
                TfType::Integer => VType::SignedNet(INTEGER_VSIZE),
                TfType::Real | TfType::Realtime | TfType::Time => todo!(),
            };
            let ident = arenas.to_item(ident);
            let name = arenas.get_ident(ident.item.0);
            let signal = signals.insert(Signal {
                name: name.to_string(),
                size: ty.force_net_width(),
                initialize: None,
                origin: arenas.get_item_span(ident),
            });
            builder.insert_net(HierarchyNet {
                name: name.to_string(),
                parent: builder.key(),
                signal,
                ty,
                dims: [].into(),
            });
        }
    }

    super::elaborate_statements(
        signals,
        arenas,
        builder,
        diagnostics,
        AstIdRange::single(*statement),
    )
}

pub fn elaborate_task<'a>(
    signals: &mut SlotMap<SignalKey, Signal>,
    arenas: &'a AstArenas,
    builder: &mut ScopeBuilder<'a>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let HierarchyItem::Task(i) = builder.hierarchy.symbols[builder.key.as_idx()] else {
        unreachable!();
    };
    let id = builder.hierarchy.tasks[i].ast;
    let TaskDeclaration {
        task_ports,
        block_item_decls,
        statement_or_null,
        ..
    } = arenas.get(id);

    if !block_item_decls.is_empty() {
        diagnostics.not_yet_implemented(arenas.get_span(id), "block item decls");
        return Err(());
    }

    for decl in task_ports.iter() {
        let (tf_type, port_identifiers) = match arenas.get(decl).content {
            TaskPortItemContent::Input(d) => (d.tf_type, d.port_identifiers),
            TaskPortItemContent::Output(d) => (d.tf_type, d.port_identifiers),
            TaskPortItemContent::Inout(d) => (d.tf_type, d.port_identifiers),
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
                            evaluate_range(arenas, builder.eval_scope(), diagnostics, range)
                                .unwrap()
                        }
                    };
                    VType::net(width, signed)
                }
                TfType::Integer => VType::SignedNet(INTEGER_VSIZE),
                TfType::Real | TfType::Realtime | TfType::Time => todo!(),
            };
            let ident = arenas.to_item(ident);
            let name = arenas.get_ident(ident.item.0);
            let signal = signals.insert(Signal {
                name: name.to_string(),
                size: ty.force_net_width(),
                initialize: None,
                origin: arenas.get_item_span(ident),
            });
            builder.insert_net(HierarchyNet {
                name: name.to_string(),
                parent: builder.key(),
                signal,
                ty,
                dims: [].into(),
            });
        }
    }

    super::elaborate_statement_or_null(
        signals,
        arenas,
        builder,
        diagnostics,
        *statement_or_null,
    )
}

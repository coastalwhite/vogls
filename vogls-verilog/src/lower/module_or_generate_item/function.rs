use std::collections::{HashMap, HashSet};

use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{
    BasicBlockTerminator, Bits, ConnectionDirection, GlobalContext, INTEGER_VSIZE, SCALAR_VSIZE,
    Signal, SignalKey, VariableKey, new_anonymous_builder,
};

use crate::ast::module::{
    FunctionDeclaration, FunctionRangeOrType, TaskDeclaration, TaskPortItemContent,
    TfInputDeclaration, TfType,
};
use crate::ast::{AstId, AstIdRange};
use crate::elaborate::ElabTable;
use crate::lower::{Diagnostics, VType, evaluate_range};
use crate::lower::{EvalScope, Scope};
use crate::parser::AstArenas;

pub fn lower<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    diagnostics: &mut Diagnostics,
    scope: &mut Scope,
    signal_map: &mut HashMap<SignalKey, SignalKey>,
    id: AstId<FunctionDeclaration>,
) -> Result<(), ()> {
    Ok(())
    /*
    use vogls_ir::Instruction as I;

    let FunctionDeclaration {
        automatic: _,
        range_or_type,
        ident,
        tf_input_decls,
        block_item_decls,
        statement,
    } = arenas.get(id);

    if !block_item_decls.is_empty() {
        diagnostics.not_yet_implemented(arenas.get_span(id), "more complex functions (1)");
        return Err(());
    }

    let name = &arenas.ident_table[ ident.item.0 ];
    let parent_key = hierarchy.functions[hierarchy_function_i].parent;

    macro_rules! scope {
        () => {
            EvalScope {
                hierarchy,
                key: parent_key,
            }
        };
    }
    let fn_key = *hierarchy
        .lookup()
        .get(&(parent_key, name.to_string()))
        .unwrap();

    // @FIXME: This is an extremely simplified implementation of functions and
    // basically only allows for the simplest of functions. Expand this to a more
    // complete solution. Do I even want to support recursive functions...
    let builder = new_anonymous_builder(gl, "function".into(), arenas.get_span(id));

    let dummy_process_key = builder.process();
    let entry_key = builder.key();

    let (_, _, output_ty) = match arenas.get(*range_or_type) {
        FunctionRangeOrType::Unsigned(None) => (0, 0, VType::UnsignedNet(SCALAR_VSIZE)),
        FunctionRangeOrType::Signed(None) => (0, 0, VType::SignedNet(SCALAR_VSIZE)),
        FunctionRangeOrType::Unsigned(Some(range)) => {
            let (msb, lsb, size) = evaluate_range(arenas, scope!(), diagnostics, *range)?;
            (msb, lsb, VType::UnsignedNet(size))
        }
        FunctionRangeOrType::Signed(Some(range)) => {
            let (msb, lsb, size) = evaluate_range(arenas, scope!(), diagnostics, *range)?;
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

    let mut scope = Scope {
        hierarchy,
        key: fn_key,
        signal_map,
    };
    let output_key = gl.signals.insert(Signal {
        name: name.to_string(),
        size: output_ty.force_net_width(),
        initialize: None,
        origin: arenas.get_item_span(*ident),
    });
    scope.builder().insert_net(crate::hierarchy::HierarchyNet {
        name: name.to_string(),
        parent: fn_key,
        signal: output_key,
        ty: output_ty,
        dims: [].into(),
        nba: None,
    });

    let mut input_types = Vec::<VType>::with_capacity(tf_input_decls.len());
    let mut input_lut = HashMap::<SignalKey, usize>::with_capacity(tf_input_decls.len());

    let mut num_inputs = 0;
    for (i, input) in tf_input_decls.iter().enumerate() {
        let TfInputDeclaration {
            tf_type,
            port_identifiers,
        } = arenas.get(input);
        let input_ty = match tf_type {
            TfType::Net {
                reg: _,
                signed,
                range,
            } => {
                let size = match range {
                    Some(range) => evaluate_range(arenas, scope.eval(), diagnostics, *range)?.2,
                    None => SCALAR_VSIZE,
                };
                if *signed {
                    VType::SignedNet(size)
                } else {
                    VType::UnsignedNet(size)
                }
            }
            TfType::Integer => VType::SignedNet(INTEGER_VSIZE),
            TfType::Real | TfType::Realtime | TfType::Time => {
                diagnostics.not_yet_implemented(
                    arenas.get_span(id),
                    "real / time / realtime function input",
                );
                return Err(());
            }
        };

        for input_ident in port_identifiers.iter() {
            let name = &arenas.ident_table[ arenas.get(input_ident).0 ];
            let input_key = gl.signals.insert(Signal {
                name: name.to_string(),
                size: input_ty.force_net_width(),
                initialize: None,
                origin: arenas.get_span(input_ident),
            });

            input_types.push(input_ty);
            input_lut.insert(input_key, i);
            scope.builder().insert_net(crate::hierarchy::HierarchyNet {
                name: name.to_string(),
                parent: fn_key,
                signal: input_key,
                ty: input_ty,
                dims: [].into(),
                nba: None,
            });
            num_inputs += 1;
        }
    }

    let builder = crate::lower::statement::statements_to_process(
        gl,
        arenas,
        &mut scope,
        diagnostics,
        builder,
        AstIdRange::single(*statement),
    )?;
    if builder.key() != entry_key {
        diagnostics.not_yet_implemented(arenas.get_span(id), "more complex functions (2)");
        return Err(());
    }
    builder.halt(gl);

    let mut nyi = false;
    let entry_key = gl.processes[dummy_process_key].entry;
    gl.processes.remove(dummy_process_key);
    let mut var_map = HashMap::<VariableKey, VariableKey>::new();
    let mut input_vars: Vec<Option<VariableKey>> = vec![None; num_inputs];
    let mut output_var: Option<VariableKey> = None;
    gl.bbs[entry_key].instrs.retain_mut(|i| match i {
        I::Probe(variable_key, signal_key) => {
            let Some(i) = input_lut.get(signal_key) else {
                nyi = true;
                return false;
            };
            match input_vars[*i] {
                None => input_vars[*i] = Some(*variable_key),
                Some(v) => _ = var_map.insert(*variable_key, v),
            }

            false
        }
        I::Drive(signal_key, variable_key, partial) => {
            if *signal_key != output_key || partial.is_some() {
                nyi = true;
            }
            output_var = Some(*variable_key);
            false
        }
        _ => true,
    });
    if nyi {
        diagnostics.not_yet_implemented(arenas.get_span(id), "more complex functions (3)");
        return Err(());
    }
    if !var_map.is_empty() {
        gl.bbs[entry_key].map_vars(|v| var_map.get(&v).copied().unwrap_or(v));
    }

    let output_var = match output_var {
        None => {
            let output_var = gl.vars.insert(vogls_ir::Variable {
                size: output_ty.force_net_width(),
            });
            gl.bbs[entry_key].instrs.push(I::Constant(
                output_var,
                Bits::new_zeroed(output_ty.force_net_width()),
            ));
            output_var
        }
        Some(v) => v,
    };

    let input_vars = input_vars
        .into_iter()
        .zip(&input_types)
        .map(|(i, ty)| {
            i.unwrap_or_else(|| {
                gl.vars.insert(vogls_ir::Variable {
                    size: ty.force_net_width(),
                })
            })
        })
        .collect();

    hierarchy.functions[hierarchy_function_i].lower = Some(LoweredFunction {
        entry: entry_key,
        input_vars,
        input_types,
        output_var,
        output_ty,
    });

    Ok(())
        */
}

pub fn lower_task<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    diagnostics: &mut Diagnostics,
    elab_table: &mut ElabTable,
    task: SymbolId,
    signal_map: &mut HashMap<SignalKey, SignalKey>,
    id: AstId<TaskDeclaration>,
) -> Result<(), ()> {
    return Ok(());
    /*
    use vogls_ir::Instruction as I;

    let TaskDeclaration {
        automatic: _,
        ident,
        task_ports,
        block_item_decls,
        statement_or_null,
    } = arenas.get(id);

    if !block_item_decls.is_empty() {
        diagnostics.not_yet_implemented(arenas.get_span(id), "more complex functions (1)");
        return Err(());
    }

    let name = &arenas.ident_table[ ident.item.0 ];
    let parent_key = hierarchy.tasks[hierarchy_task_i].parent;

    let fn_key = *hierarchy
        .lookup()
        .get(&(parent_key, name.to_string()))
        .unwrap();

    // @FIXME: This is an extremely simplified implementation of functions and
    // basically only allows for the simplest of functions. Expand this to a more
    // complete solution. Do I even want to support recursive functions...
    let builder = new_anonymous_builder(gl, "task".into(), arenas.get_span(id));

    let dummy_process_key = builder.process();
    let entry_key = builder.key();

    let mut io_types = Vec::<(ConnectionDirection, VType)>::new();
    let mut input_lut = HashMap::<SignalKey, usize>::new();
    let mut output_lut = HashMap::<SignalKey, usize>::new();

    let mut scope = Scope {
        hierarchy,
        key: fn_key,
        signal_map,
    };

    let mut num_ports = 0;
    for (_, port) in task_ports.iter().enumerate() {
        use ConnectionDirection as D;
        use TaskPortItemContent as TPIC;
        let (direction, tf_type, port_identifiers) = match arenas.get(port).content {
            TPIC::Input(d) => (D::In, d.tf_type, d.port_identifiers),
            TPIC::Output(d) => (D::Out, d.tf_type, d.port_identifiers),
            TPIC::Inout(d) => (D::Both, d.tf_type, d.port_identifiers),
        };
        let port_ty = match tf_type {
            TfType::Net {
                reg: _,
                signed,
                range,
            } => {
                let size = match range {
                    Some(range) => evaluate_range(arenas, scope.eval(), diagnostics, range)?.2,
                    None => SCALAR_VSIZE,
                };
                if signed {
                    VType::SignedNet(size)
                } else {
                    VType::UnsignedNet(size)
                }
            }
            TfType::Integer => VType::SignedNet(INTEGER_VSIZE),
            TfType::Real | TfType::Realtime | TfType::Time => {
                diagnostics.not_yet_implemented(
                    arenas.get_span(id),
                    "real / time / realtime function input",
                );
                return Err(());
            }
        };

        for port_ident in port_identifiers.iter() {
            let name = &arenas.ident_table[ arenas.get(port_ident).0 ];
            let port_key = gl.signals.insert(Signal {
                name: name.to_string(),
                size: port_ty.force_net_width(),
                initialize: None,
                origin: arenas.get_span(port_ident),
            });

            io_types.push((direction, port_ty));
            match direction {
                D::In => {
                    input_lut.insert(port_key, num_ports);
                }
                D::Out => {
                    output_lut.insert(port_key, num_ports);
                }
                D::Both => {
                    input_lut.insert(port_key, num_ports);
                    output_lut.insert(port_key, num_ports);
                }
            }
            scope.builder().insert_net(crate::hierarchy::HierarchyNet {
                name: name.to_string(),
                parent: fn_key,
                signal: port_key,
                ty: port_ty,
                dims: [].into(),
                nba: None,
            });
            num_ports += 1;
        }
    }

    let builder = crate::lower::statement::lower_statement_or_null(
        gl,
        arenas,
        &mut scope,
        diagnostics,
        builder,
        *statement_or_null,
    )?;
    builder.halt(gl);

    let mut bb_key = entry_key;
    loop {
        match gl.bbs[bb_key].terminator {
            BasicBlockTerminator::Wait(next, _)
            | BasicBlockTerminator::WaitRegion(next, _)
            | BasicBlockTerminator::Watch(next, _)
            | BasicBlockTerminator::Jump(next) => bb_key = next,
            BasicBlockTerminator::Halt => break,
            BasicBlockTerminator::Branch(..) => {
                diagnostics.not_yet_implemented(arenas.get_span(id), "more complex functions (2)");
                return Err(());
            }
        };
    }

    let mut nyi = false;
    let entry_key = gl.processes[dummy_process_key].entry;
    gl.processes.remove(dummy_process_key);
    let mut var_map = HashMap::<VariableKey, VariableKey>::new();
    let mut io_vars: Vec<Option<VariableKey>> = vec![None; num_ports];

    let mut bb_stack = Vec::new();
    let mut bb_seen = HashSet::new();

    bb_stack.push(entry_key);
    bb_seen.insert(entry_key);

    while let Some(bb_key) = bb_stack.pop() {
        gl.bbs[bb_key]
            .terminator
            .extend_next_rev(&mut bb_stack, &mut bb_seen);
        gl.bbs[bb_key].instrs.retain_mut(|i| match i {
            I::Probe(variable_key, signal_key) => {
                let Some(i) = input_lut.get(signal_key) else {
                    return true;
                };
                match io_vars[*i] {
                    None => io_vars[*i] = Some(*variable_key),
                    Some(v) => _ = var_map.insert(*variable_key, v),
                }

                false
            }
            I::Drive(signal_key, variable_key, partial) => {
                let Some(i) = output_lut.get(signal_key) else {
                    return true;
                };

                if partial.is_some() {
                    nyi = true;
                    return false;
                }

                match io_vars[*i] {
                    None => io_vars[*i] = Some(*variable_key),
                    Some(v) => _ = var_map.insert(*variable_key, v),
                }

                false
            }
            _ => true,
        });
    }
    if nyi {
        diagnostics.not_yet_implemented(arenas.get_span(id), "more complex functions (3)");
        return Err(());
    }

    if !var_map.is_empty() {
        let mut bb_stack = Vec::new();
        let mut bb_seen = HashSet::new();

        bb_stack.push(entry_key);
        bb_seen.insert(entry_key);

        while let Some(bb_key) = bb_stack.pop() {
            gl.bbs[bb_key]
                .terminator
                .extend_next_rev(&mut bb_stack, &mut bb_seen);
            gl.bbs[bb_key].map_vars(|v| var_map.get(&v).copied().unwrap_or(v));
        }
    }

    let io_vars = io_vars
        .into_iter()
        .zip(&io_types)
        .map(|(i, (direction, ty))| {
            i.unwrap_or_else(|| match direction {
                ConnectionDirection::In => gl.vars.insert(vogls_ir::Variable {
                    size: ty.force_net_width(),
                }),
                ConnectionDirection::Out | ConnectionDirection::Both => {
                    let output_var = gl.vars.insert(vogls_ir::Variable {
                        size: ty.force_net_width(),
                    });
                    gl.bbs[entry_key].instrs.push(I::Constant(
                        output_var,
                        Bits::new_zeroed(ty.force_net_width()),
                    ));
                    output_var
                }
            })
        })
        .collect();

    hierarchy.tasks[hierarchy_task_i].lower = Some(LoweredTask {
        entry: entry_key,
        io_vars,
        io_types,
    });

    Ok(())
        */
}

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
use crate::elaborate::{LoweredFunction, LoweredTask, NetSymbol, VSymbol, VSymbolTable};
use crate::lower::{
    Diagnostics, VType, evaluate_range, try_resolve_net, unwrap_get_fn_mut, unwrap_get_task_mut,
    unwrap_resolve_net,
};
use crate::lower::{EvalScope, Scope};
use crate::parser::AstArenas;

pub fn lower<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    diagnostics: &mut Diagnostics,
    scope: &mut Scope<'a>,
    id: AstId<FunctionDeclaration>,
) -> Result<(), ()> {
    use vogls_ir::Instruction as I;

    let FunctionDeclaration {
        automatic: _,
        range_or_type: _,
        ident,
        tf_input_decls,
        block_item_decls,
        statement,
    } = arenas.get(id);

    if !block_item_decls.is_empty() {
        diagnostics.not_yet_implemented(arenas.get_span(id), "more complex functions (1)");
        return Err(());
    }

    // @TODO: This is an extremely simplified implementation of functions and
    // basically only allows for the simplest of functions. Expand this to a more
    // complete solution. Do I even want to support recursive functions...
    let builder = new_anonymous_builder(gl, "function".into(), arenas.get_span(id));

    let dummy_process_key = builder.process();
    let entry_key = builder.key();

    let output = unwrap_resolve_net(scope.key, scope.table, ident.item.0);
    let output_key = output.signal;
    let output_ty = output.ty.clone();

    let mut input_types = Vec::<VType>::with_capacity(tf_input_decls.len());
    let mut input_lut = HashMap::<SignalKey, usize>::with_capacity(tf_input_decls.len());

    let mut num_inputs = 0;
    for (i, input) in tf_input_decls.iter().enumerate() {
        let TfInputDeclaration {
            tf_type: _,
            port_identifiers,
        } = arenas.get(input);

        for input_ident in port_identifiers.iter() {
            let input = unwrap_resolve_net(scope.key, scope.table, arenas.get(input_ident).0);
            let input_key = input.signal;
            let input_ty = input.ty.clone();

            input_types.push(input_ty);
            input_lut.insert(input_key, i);
            num_inputs += 1;
        }
    }

    let builder = crate::lower::statement::statements_to_process(
        gl,
        arenas,
        scope,
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

    unwrap_get_fn_mut(scope.table, scope.key).lowered = Some(LoweredFunction {
        entry: entry_key,
        input_vars,
        input_types,
        output_var,
        output_ty,
    });

    Ok(())
}

pub fn lower_task<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    diagnostics: &mut Diagnostics,
    scope: &mut Scope<'a>,
    id: AstId<TaskDeclaration>,
) -> Result<(), ()> {
    use vogls_ir::Instruction as I;

    let TaskDeclaration {
        automatic: _,
        ident: _,
        task_ports,
        block_item_decls,
        statement_or_null,
    } = arenas.get(id);

    if !block_item_decls.is_empty() {
        diagnostics.not_yet_implemented(arenas.get_span(id), "more complex functions (1)");
        return Err(());
    }

    // let name = &arenas.ident_table[ident.item.0];
    // let parent = hierarchy.tasks[hierarchy_task_i].parent;
    //
    // let fn_key = *hierarchy
    //     .lookup()
    //     .get(&(parent_key, name.to_string()))
    //     .unwrap();

    // @FIXME: This is an extremely simplified implementation of functions and
    // basically only allows for the simplest of functions. Expand this to a more
    // complete solution. Do I even want to support recursive functions...
    let builder = new_anonymous_builder(gl, "task".into(), arenas.get_span(id));

    let dummy_process_key = builder.process();
    let entry_key = builder.key();

    let mut io_types = Vec::<(ConnectionDirection, VType)>::new();
    let mut input_lut = HashMap::<SignalKey, usize>::new();
    let mut output_lut = HashMap::<SignalKey, usize>::new();

    let mut num_ports = 0;
    for (_, port) in task_ports.iter().enumerate() {
        use ConnectionDirection as D;
        use TaskPortItemContent as TPIC;
        let (direction, port_identifiers) = match arenas.get(port).content {
            TPIC::Input(d) => (D::In, d.port_identifiers),
            TPIC::Output(d) => (D::Out, d.port_identifiers),
            TPIC::Inout(d) => (D::Both, d.port_identifiers),
        };

        for port_ident in port_identifiers.iter() {
            let port_net = unwrap_resolve_net(scope.key, scope.table, arenas.get(port_ident).0);
            let port_key = port_net.signal;
            let port_ty = port_net.ty;

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
            num_ports += 1;
        }
    }

    let builder = crate::lower::statement::lower_statement_or_null(
        gl,
        arenas,
        scope,
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

    unwrap_get_task_mut(scope.table, scope.key).lowered = Some(LoweredTask {
        entry: entry_key,
        io_vars,
        io_types,
    });

    Ok(())
}

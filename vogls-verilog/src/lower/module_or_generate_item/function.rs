use std::collections::HashMap;

use vogls_ir::{
    Bits, GlobalContext, INTEGER_VSIZE, SCALAR_VSIZE, Signal, SignalKey, VariableKey,
    new_anonymous_builder,
};

use crate::ast::module::{FunctionDeclaration, FunctionRangeOrType, TfInputDeclaration, TfType};
use crate::ast::{AstId, AstIdRange};
use crate::lower::Scope;
use crate::lower::{Diagnostics, VType, evaluate_range};
use crate::parser::AstArenas;

pub fn lower<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    id: AstId<FunctionDeclaration>,
) -> Result<(), ()> {
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

    let name = arenas.get_ident(ident.item.0);

    let fn_key = scope
        .hierarchy
        .lookup()
        .get(&(scope.key, name.to_string()))
        .unwrap();
    // let HierarchyItem::Function(i) = &scope.hierarchy.items()[fn_key.as_idx()] else { panic!() };
    // let HierarchyFunction { .. } = &scope.hierarchy.function()[*i] else { panic!() };

    // @FIXME: This is an extremely simplified implementation of functions and
    // basically only allows for the simplest of functions. Expand this to a more
    // complete solution. Do I even want to support recursive functions...
    let builder = new_anonymous_builder(gl, "function".into(), arenas.get_span(id));

    let dummy_process_key = builder.process();
    let entry_key = builder.key();

    let (msb, lsb, output_ty) = match arenas.get(*range_or_type) {
        FunctionRangeOrType::Unsigned(None) => (0, 0, VType::UnsignedNet(SCALAR_VSIZE)),
        FunctionRangeOrType::Signed(None) => (0, 0, VType::SignedNet(SCALAR_VSIZE)),
        FunctionRangeOrType::Unsigned(Some(range)) => {
            let (msb, lsb, size) = evaluate_range(arenas, scope.eval(), diagnostics, *range)?;
            (msb, lsb, VType::UnsignedNet(size))
        }
        FunctionRangeOrType::Signed(Some(range)) => {
            let (msb, lsb, size) = evaluate_range(arenas, scope.eval(), diagnostics, *range)?;
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

    let fn_key = *fn_key;
    let mut fn_scope = Scope {
        hierarchy: scope.hierarchy,
        key: fn_key,
        signal_map: scope.signal_map,
    };
    let output_key = gl.signals.insert(Signal {
        name: name.to_string(),
        size: output_ty.force_net_width(),
        initialize: None,
        origin: arenas.get_item_span(*ident),
    });

    let mut input_types = Vec::<VType>::with_capacity(tf_input_decls.len());
    let mut input_lut = HashMap::<SignalKey, usize>::with_capacity(tf_input_decls.len());

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
                    Some(range) => evaluate_range(arenas, fn_scope.eval(), diagnostics, *range)?.2,
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
            let name = arenas.get_ident(arenas.get(input_ident).0);
            let input_key = gl.signals.insert(Signal {
                name: name.to_string(),
                size: input_ty.force_net_width(),
                initialize: None,
                origin: arenas.get_span(input_ident),
            });

            input_types.push(input_ty);
            input_lut.insert(input_key, i);
        }
    }

    let builder = crate::lower::statement::statements_to_process(
        gl,
        arenas,
        &mut fn_scope,
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
    let mut var_map = HashMap::<VariableKey, VariableKey>::new();
    let mut input_vars: Vec<Option<VariableKey>> = vec![None; tf_input_decls.len()];
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
        I::Drive(signal_key, variable_key, region, partial) => {
            if *signal_key != output_key || *region != 0 || partial.is_some() {
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

    // let input_vars = input_vars
    //     .into_iter()
    //     .zip(&input_types)
    //     .map(|(i, ty)| {
    //         i.unwrap_or_else(|| {
    //             gl.vars.insert(vogls_ir::Variable {
    //                 size: ty.force_net_width(),
    //             })
    //         })
    //     })
    //     .collect();

    Ok(())
}

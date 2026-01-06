use std::collections::HashMap;

use vogls_ir::{
    Bits, GlobalContext, INTEGER_VSIZE, SCALAR_VSIZE, Signal, SignalKey, VariableKey,
    new_anonymous_builder,
};

use crate::ast::module::{FunctionDeclaration, FunctionRangeOrType, TfInputDeclaration, TfType};
use crate::ast::{AstId, AstIdRange};
use crate::lower::scope::{FunctionSymbol, Scope, SignalSymbol, Symbol, SymbolVariant};
use crate::lower::{Diagnostics, ModuleContext, VType, evaluate_range};
use crate::parser::AstArenas;

pub fn lower<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    mc: &mut ModuleContext<'a>,
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
            let (msb, lsb, size) = evaluate_range(gl, arenas, scope, diagnostics, *range)?;
            (msb, lsb, VType::UnsignedNet(size))
        }
        FunctionRangeOrType::Signed(Some(range)) => {
            let (msb, lsb, size) = evaluate_range(gl, arenas, scope, diagnostics, *range)?;
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

    let mut fn_scope = Scope::new();
    let output_key = gl.signals.insert(Signal {
        name: name.to_string(),
        size: output_ty.force_net_width(),
        initialize: None,
        origin: arenas.get_item_span(*ident),
    });
    let output_symkey = fn_scope.symbols.insert(Symbol {
        name: name.to_string(),
        definition_site: arenas.get_item_span(*ident),
        variant: SymbolVariant::Signal(SignalSymbol {
            dims: Vec::new(),
            ty: output_ty,
            key: output_key,
            msb,
            lsb,
        }),
    });
    fn_scope.push(arenas.get_ident(ident.item.0), output_symkey);

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
                    Some(range) => evaluate_range(gl, arenas, scope, diagnostics, *range)?.2,
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
            let input_symkey = fn_scope.symbols.insert(Symbol {
                name: name.to_string(),
                definition_site: arenas.get_span(input_ident),
                variant: SymbolVariant::Signal(SignalSymbol {
                    dims: Vec::new(),
                    ty: input_ty,
                    key: input_key,
                    msb,
                    lsb,
                }),
            });
            fn_scope.push(name, input_symkey);

            input_types.push(input_ty);
            input_lut.insert(input_key, i);
        }
    }

    let builder = crate::lower::statement::statements_to_process(
        gl,
        arenas,
        &mut fn_scope,
        mc,
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

    // Clean up all the dummy signals
    for sym in fn_scope.symbols.iter() {
        if let SymbolVariant::Signal(s) = &sym.variant {
            gl.signals.remove(s.key);
        }
    }
    gl.processes.remove(dummy_process_key);

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

    let fn_key = scope.fns.len();
    scope.fns.push(FunctionSymbol {
        entry: entry_key,
        input_vars,
        input_types,
        output_var,
        output_ty,
    });
    let exists = scope.fns_lut.insert(name, fn_key).is_some();
    if exists {
        diagnostics.not_yet_implemented(arenas.get_span(id), "duplicate function name");
        return Err(());
    }

    Ok(())
}

use std::collections::HashMap;

use vogls_ir::{BasicBlockBuilder, BasicBlockTerminator, GlobalContext, VariableKey};

use crate::ast::expr::Expr;
use crate::ast::{AstId, AstItem, Identifier};
use crate::lower::expression::truncate_or_extend;
use crate::lower::Scope;
use crate::lower::{Diagnostics, VType};
use crate::parser::AstArenas;

pub fn lower_function_call<'a>(
    _gl: &mut GlobalContext,
    _arenas: &'a AstArenas,
    _scope: &Scope<'a>,
    _diagnostics: &mut Diagnostics,
    _builder: &mut BasicBlockBuilder,
    _expr: AstId<Expr>,
    _ident: AstItem<Identifier>,
    _arguments: &[Option<(VariableKey, VType)>],
) -> Result<(VariableKey, VType), ()> {
    todo!()
    /*
    let fn_name = arenas.get_ident(ident.item.0);
    let Some(fn_symbol) = scope.fns_lut.get(fn_name) else {
        diagnostics.var_not_found(arenas, ident);
        return Err(());
    };

    let fn_symbol = &scope.fns[*fn_symbol];

    assert_eq!(fn_symbol.input_vars.len(), fn_symbol.input_types.len());
    if fn_symbol.input_vars.len() != arguments.len() {
        diagnostics.not_yet_implemented(arenas.get_span(expr), "invalid number of arguments");
        return Err(());
    }

    let mut map = HashMap::new();
    for i in 0..fn_symbol.input_vars.len() {
        let Some((arg_variable, arg_ty)) = arguments[i] else {
            return Err(());
        };
        let input_var = fn_symbol.input_vars[i];
        let input_ty = fn_symbol.input_types[i];
        let arg_variable = truncate_or_extend(
            gl,
            builder,
            arg_variable,
            arg_ty,
            input_ty.force_net_width(),
        );
        map.insert(input_var, arg_variable);
    }

    let mut fn_bb = gl.bbs[fn_symbol.entry].clone();
    fn_bb.map_vars(|v| {
        *map.entry(v).or_insert_with(|| {
            let fn_var = gl.vars[v].clone();
            gl.vars.insert(fn_var)
        })
    });

    let origin_bb = builder.key();
    *builder = builder.next_terminate_later(gl);
    fn_bb.terminator = BasicBlockTerminator::Jump(builder.key());
    let fn_bb = gl.bbs.insert(fn_bb);
    gl.bbs[origin_bb].terminator = BasicBlockTerminator::Jump(fn_bb);

    Ok((map[&fn_symbol.output_var], fn_symbol.output_ty))
    */
}

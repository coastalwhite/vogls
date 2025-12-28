mod assign;
mod constant_expr;
mod diagnostics;
mod expression;
mod module_or_generate_item;
mod parameter;
mod scope;
mod statement;
mod vtype;
mod vvalue;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Region {
    Active = 0,
    Inactive = 1,
    NonBlocking = 2,
    Monitor = 3,
}

use std::collections::{HashMap, HashSet};

use scope::Scope;

use constant_expr::eval_constant_expr;
use vogls_ir::{
    BasicBlockBuilder, ConnectionDirection, GlobalContext, ProcessKey, SCALAR_VSIZE, Signal,
    SignalKey, VariableKey, VectorSize, new_process,
};

use crate::ast::constant_expr::{
    ConstantExpr, ConstantMinTypMaxExpression, ConstantRangeExpression,
};
use crate::ast::expr::{BitSlice, Expr};
use crate::ast::module::{
    GenerateRegion, Module, ModuleItem, ModulePorts, NonPortModuleItem, ParamAssignment,
    ParameterDeclaration, Port, PortDeclaration, PortExpression, PortReference, Range,
};
use crate::ast::statement::{
    LoopStatementVariant, NetLValue, ProceduralTimingControlStatement, Statement, StatementContent,
    StatementOrNull, VariableAssignment,
};
use crate::ast::{AstId, AstIdRange};
use crate::parser::{AstArenas, TokenRange};

use self::expression::lower_expr;
use self::scope::{Symbol, SymbolKey, SymbolVariant};
pub use self::vtype::VType;
use self::vvalue::VValue;
pub use diagnostics::Diagnostics;

pub struct ModuleInitialization<'a> {
    pub name: &'a str,
    pub parameters: ModuleParameters<'a>,
    pub io: ModuleIo<'a>,
    pub args: ModuleArgs,
}

#[derive(Clone)]
pub struct ModuleIo<'a> {
    pub lut: HashMap<&'a str, usize>,
    pub ports: Vec<(&'a str, ConnectionDirection, VType)>,
}
#[derive(Clone)]
pub struct ModuleParameters<'a> {
    pub lut: HashMap<&'a str, usize>,
    pub params: Vec<&'a str>,
}

#[derive(Clone)]
pub struct ModuleArgs {
    pub parameters: Vec<VValue>,
    pub signals: Vec<SignalKey>,
}

pub fn fetch_module_interface<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    module: AstId<Module>,
    parameters: &[(&'a str, VValue, TokenRange)],
    diagnostics: &mut Diagnostics,
) -> Result<(ModuleParameters<'a>, ModuleIo<'a>, Vec<VValue>), ()> {
    let Module {
        attribute_instances: _,
        module_identifier: _,
        module_parameter_port_list,
        ports,
        module_items,
    } = arenas.get(module);

    let mut param_lut = HashMap::new();
    let mut params = Vec::new();
    let mut param_values = Vec::new();
    let mut scope = Scope::new();
    let mut error = false;
    if let Some(parameters) = module_parameter_port_list {
        for p in parameters.iter() {
            let ParameterDeclaration {
                typing,
                assignments,
            } = arenas.get(p);
            // @FIXME: Coerce value to ty.
            let _ty =
                parameter::parameter_typing_to_type(gl, arenas, &mut scope, diagnostics, *typing)?;
            for assignment in assignments.iter() {
                let ParamAssignment { param, constant } = arenas.get(assignment);
                let key = arenas.get_ident(param.item.0);
                let value = arenas.get(*constant);
                match value {
                    ConstantMinTypMaxExpression::Single(id) => {
                        let value = eval_constant_expr(gl, arenas, &scope, diagnostics, *id)?;
                        let symbol_key = scope.symbols.insert(Symbol {
                            name: key.to_string(),
                            definition_site: arenas.get_item_span(*param),
                            variant: SymbolVariant::Constant(value.clone()),
                        });
                        scope.push(key, symbol_key);
                        param_values.push(value);
                    }
                    ConstantMinTypMaxExpression::MinTypMax { .. } => todo!(),
                }

                if param_lut.insert(key, params.len()).is_some() {
                    diagnostics.duplicate_definition(arenas, *param);
                    error = true;
                    continue;
                }
                params.push(key);
            }
        }
    }

    for (key, value, tr) in parameters {
        let Some(param_idx) = param_lut.get(key) else {
            diagnostics.not_yet_implemented(*tr, "missing parameter");
            error = true;
            continue;
        };
        let symbol_key = scope.get(key).unwrap();
        scope.symbols[symbol_key].variant = SymbolVariant::Constant(value.clone());
        param_values[*param_idx] = value.clone();
    }
    if error {
        return Err(());
    }

    let mut lut = HashMap::new();
    let mut io = Vec::new();
    match ports {
        ModulePorts::Ports(ports) => {
            let mut error = false;
            for port in ports.iter() {
                match arenas.get(port) {
                    Port::PortExpression(id) => {
                        let PortExpression { references } = arenas.get(*id);
                        let PortReference { identifier } = arenas.get(*references);

                        let name = arenas.get_ident(identifier.item.0);
                        if lut.insert(name, io.len()).is_some() {
                            diagnostics.duplicate_definition(arenas, *identifier);
                            error = true;
                            continue;
                        }
                        io.push((
                            name,
                            ConnectionDirection::Both,
                            VType::UnsignedNet(SCALAR_VSIZE),
                        ));
                    }
                }
            }

            if error {
                return Err(());
            }

            let mut port_seen = HashSet::<&str>::new();
            for item in module_items.iter() {
                let ModuleItem::PortDeclaration(ast_port_declaration) = arenas.get(item) else {
                    continue;
                };

                let port_declaration = arenas.get(*ast_port_declaration);
                let (direction, range, signed, identifiers) = match port_declaration {
                    PortDeclaration::Inout(id) => {
                        let inout = arenas.get(*id);
                        (
                            ConnectionDirection::Both,
                            inout.range,
                            inout.signed,
                            inout.port_identifiers,
                        )
                    }
                    PortDeclaration::Input(id) => {
                        let input = arenas.get(*id);
                        (
                            ConnectionDirection::In,
                            input.range,
                            input.signed,
                            input.port_identifiers,
                        )
                    }
                    PortDeclaration::Output(id) => {
                        let output = arenas.get(*id);
                        (
                            ConnectionDirection::Out,
                            output.range,
                            output.signed,
                            output.identifiers,
                        )
                    }
                };
                let size = match range {
                    None => SCALAR_VSIZE,
                    Some(range) => range_to_width(gl, arenas, &scope, diagnostics, range)?,
                };

                for ast_ident in identifiers {
                    let ast_ident = arenas.to_item(ast_ident);
                    let ident = arenas.get_ident(ast_ident.item.0);

                    let Some(idx) = lut.get(ident) else {
                        diagnostics.port_not_defined(arenas, ast_ident);
                        error = true;
                        continue;
                    };
                    if !port_seen.insert(ident) {
                        diagnostics.duplicate_definition(arenas, ast_ident);
                        error = true;
                        continue;
                    }

                    io[*idx].1 = direction;
                    io[*idx].2 = VType::net(size, signed);
                }
            }

            if port_seen.len() != io.len() {
                diagnostics
                    .not_yet_implemented(arenas.get_range_span(*ports), "not all ports referenced");
                return Err(());
            }

            if error {
                return Err(());
            }
        }
        ModulePorts::PortDeclarations(port_declarations) => {
            for ast_port_declaration in port_declarations.iter() {
                let port_declaration = arenas.get(ast_port_declaration);
                let (direction, range, signed, identifiers) = match port_declaration {
                    PortDeclaration::Inout(id) => {
                        let inout = arenas.get(*id);
                        (
                            ConnectionDirection::Both,
                            inout.range,
                            inout.signed,
                            inout.port_identifiers,
                        )
                    }
                    PortDeclaration::Input(id) => {
                        let input = arenas.get(*id);
                        (
                            ConnectionDirection::In,
                            input.range,
                            input.signed,
                            input.port_identifiers,
                        )
                    }
                    PortDeclaration::Output(id) => {
                        let output = arenas.get(*id);
                        (
                            ConnectionDirection::Out,
                            output.range,
                            output.signed,
                            output.identifiers,
                        )
                    }
                };
                let size = match range {
                    None => SCALAR_VSIZE,
                    Some(range) => range_to_width(gl, arenas, &scope, diagnostics, range)?,
                };

                let mut error = false;
                for ident in identifiers.iter() {
                    let name = arenas.get_ident(arenas.get(ident).0);
                    if lut.insert(name, io.len()).is_some() {
                        diagnostics.duplicate_definition(arenas, arenas.to_item(ident));
                        error = true;
                        continue;
                    }
                    io.push((name, direction, VType::net(size, signed)));
                }

                if error {
                    return Err(());
                }
            }
        }
    }
    Ok((
        ModuleParameters {
            lut: param_lut,
            params,
        },
        ModuleIo { lut, ports: io },
        param_values,
    ))
}

pub fn lower_module_to_ir<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    root: AstId<Module>,
    parameters: &ModuleParameters<'a>,
    io: &ModuleIo<'a>,
    args: &ModuleArgs,
    module_lut: &HashMap<&'a str, AstId<Module>>,
    next_modules: &mut Vec<ModuleInitialization<'a>>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let Module {
        attribute_instances: _,
        module_identifier: _,
        module_parameter_port_list: _,
        ports: _,
        module_items,
    } = arenas.get(root);

    let mut scope = Scope::new();
    let mut processes = Vec::new();

    for (key, param) in parameters.params.iter().zip(&args.parameters) {
        let symbol_key = scope.symbols.insert(Symbol {
            name: key.to_string(),
            // @TODO: better definition site
            definition_site: arenas.get_span(root),
            variant: SymbolVariant::Constant(param.clone()),
        });
        scope.push(key, symbol_key);
    }

    for ((name, _con, ty), signal) in io.ports.iter().zip(&args.signals) {
        let symbol_key = scope.symbols.insert(Symbol {
            name: name.to_string(),
            // @TODO: better definition site
            definition_site: arenas.get_span(root),
            variant: SymbolVariant::Signal(Vec::new(), *ty, *signal),
        });
        scope.push(name, symbol_key);
    }

    for module_item in module_items.iter() {
        match arenas.get(module_item) {
            ModuleItem::PortDeclaration(_) => {}
            ModuleItem::NonPortModuleItem(p) => match arenas.get(*p) {
                NonPortModuleItem::ModuleOrGenerateItem(id) => module_or_generate_item::lower(
                    gl,
                    arenas,
                    module_lut,
                    next_modules,
                    &mut scope,
                    &mut processes,
                    *id,
                    diagnostics,
                )?,
                NonPortModuleItem::GenerateRegion(region) => {
                    let GenerateRegion {
                        module_or_generate_item,
                    } = region;
                    for id in module_or_generate_item.iter() {
                        module_or_generate_item::lower(
                            gl,
                            arenas,
                            module_lut,
                            next_modules,
                            &mut scope,
                            &mut processes,
                            id,
                            diagnostics,
                        )?;
                    }
                }
                NonPortModuleItem::SpecifyBlock => todo!(),
                NonPortModuleItem::ParameterDeclaration(id) => {
                    let ParameterDeclaration {
                        typing: _,
                        assignments,
                    } = arenas.get(*id);
                    for assignment in assignments.iter() {
                        let ParamAssignment { param: _, constant } = arenas.get(assignment);
                        let ConstantMinTypMaxExpression::Single(constant) = arenas.get(*constant)
                        else {
                            todo!();
                        };

                        let _value =
                            eval_constant_expr(gl, arenas, &scope, diagnostics, *constant)?;
                        todo!()
                        // scope.push(arenas.get_ident(param.item.0), ScopeItem::Constant(v));
                    }
                }
                NonPortModuleItem::SpecParamDeclaration => todo!(),
            },
        }
    }
    Ok(())
}

enum WatchCondition {
    None,
    Posedge,
    Negedge,
}

fn add_var_assign_intersect_symbols_generated<'a>(
    _gl: &mut GlobalContext,
    scope: &Scope<'a>,
    var_assign: AstId<VariableAssignment>,
    arenas: &'a AstArenas,
    black_list: &mut HashSet<&'a str>,
    symbol_keys: &mut Vec<SymbolKey>,
) {
    let va = arenas.get(var_assign);
    let lvalue = arenas.get(va.lvalue);
    if lvalue.0.len() != 1 {
        panic!("not supported");
    }
    let lvalue = arenas.get(lvalue.0.get(0));
    let ident = arenas.get_ident(lvalue.ident.item.0);
    if black_list.insert(ident) {
        symbol_keys.push(scope.get(ident).unwrap());
    }
}

fn get_intersect_symbols_generated<'a>(
    gl: &mut GlobalContext,
    scope: &Scope<'a>,
    stmts: AstIdRange<Statement>,
    arenas: &'a AstArenas,
) -> Vec<SymbolKey> {
    use StatementContent as S;

    let mut symbols = Vec::new();
    let mut black_list = HashSet::<&str>::new();
    let mut stack = Vec::new();
    stack.push(stmts);
    while let Some(mut stmts) = stack.pop() {
        while let Some(stmt) = stmts.pop_front() {
            let stmt = arenas.get(stmt);
            match stmt.content {
                S::BlockingAssignment(id) => {
                    let ba = arenas.get(id);
                    let lvalue = arenas.get(ba.variable_lvalue);
                    for lvalue in lvalue.0.iter() {
                        let lvalue = arenas.get(lvalue);
                        let ident = arenas.get_ident(lvalue.ident.item.0);
                        if black_list.insert(ident) {
                            symbols.push(scope.get(ident).unwrap());
                        }
                    }
                }
                S::NonBlockingAssignment(id) => {
                    let nba = arenas.get(id);
                    let lvalue = arenas.get(nba.variable_lvalue);
                    for lvalue in lvalue.0.iter() {
                        let lvalue = arenas.get(lvalue);
                        let ident = arenas.get_ident(lvalue.ident.item.0);
                        if black_list.insert(ident) {
                            symbols.push(scope.get(ident).unwrap());
                        }
                    }
                }
                S::CaseStatement(id) => {
                    let c = arenas.get(id);
                    stack.push(stmts);
                    stack.extend(c.items.iter().filter_map(|c| {
                        match arenas.get(arenas.get(c).statement_or_null) {
                            StatementOrNull::Attribute(_) => None,
                            StatementOrNull::Statement(stmt) => Some(AstIdRange::single(*stmt)),
                        }
                    }));
                    break;
                }
                S::ConditionalStatement(id) => {
                    let c = arenas.get(id);
                    stack.push(stmts);
                    match arenas.get(c.if_branch.statement) {
                        StatementOrNull::Attribute(_) => {}
                        StatementOrNull::Statement(stmt) => stack.push(AstIdRange::single(*stmt)),
                    }
                    stack.extend(c.else_ifs.iter().filter_map(|c| {
                        match arenas.get(arenas.get(c).statement) {
                            StatementOrNull::Attribute(_) => None,
                            StatementOrNull::Statement(stmt) => Some(AstIdRange::single(*stmt)),
                        }
                    }));
                    if let Some(else_branch) = c.else_branch {
                        match arenas.get(else_branch) {
                            StatementOrNull::Attribute(_) => {}
                            StatementOrNull::Statement(stmt) => {
                                stack.push(AstIdRange::single(*stmt))
                            }
                        }
                    }
                    break;
                }
                S::DisableStatement => todo!(),
                S::EventTrigger => todo!(),
                S::LoopStatement(id) => {
                    let ls = arenas.get(id);
                    if let LoopStatementVariant::For(init, _, step) = &ls.variant {
                        add_var_assign_intersect_symbols_generated(
                            gl,
                            scope,
                            *init,
                            arenas,
                            &mut black_list,
                            &mut symbols,
                        );
                        add_var_assign_intersect_symbols_generated(
                            gl,
                            scope,
                            *step,
                            arenas,
                            &mut black_list,
                            &mut symbols,
                        );
                    }
                    stack.push(stmts);
                    stack.push(AstIdRange::single(ls.statement));
                    break;
                }
                S::ParBlock => todo!(),
                S::ProceduralContinuousAssignments => todo!(),
                S::ProceduralTimingControlStatement(ptc) => {
                    let ProceduralTimingControlStatement {
                        procedural_timing_control: _,
                        statement_or_null,
                    } = arenas.get(ptc);
                    stack.push(stmts);
                    match arenas.get(*statement_or_null) {
                        StatementOrNull::Attribute(_) => {}
                        StatementOrNull::Statement(stmt) => stack.push(AstIdRange::single(*stmt)),
                    };
                    break;
                }
                S::SeqBlock(id) => {
                    let seq_block = arenas.get(id);
                    stack.push(stmts);
                    stack.push(seq_block.statements);
                    break;
                }
                S::SystemTaskEnable(_) => continue,
                S::TaskEnable(_) => continue,
                S::WaitStatement => todo!(),
            }
        }
    }
    symbols
}

fn lower_to_signal<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    processes: &mut Vec<ProcessKey>,
    expr: AstId<Expr>,
    ty: VType,
) -> Result<SignalKey, ()> {
    if let Expr::Ident(ast_ident, exprs, range_expression) = arenas.get(expr)
        && exprs.is_empty()
        && range_expression.is_none()
    {
        let ident = arenas.get_ident(ast_ident.item.0);
        let Some(symbol_key) = scope.get(&ident) else {
            diagnostics.var_not_found(arenas, *ast_ident);
            return Err(());
        };
        if let SymbolVariant::Signal(_dims, signal_ty, key) = &scope.symbols[symbol_key].variant
            && *signal_ty == ty
        {
            return Ok(*key);
        }
    }

    let signal = gl.signals.insert(Signal {
        name: "anon_port_assignment".to_string(),
        size: ty.force_net_width(),
        initialize: None,
    });

    let (section_key, mut bb_builder) = new_process(gl, "port_assignment".into());
    let bb_key = bb_builder.key();
    let (v, v_ty) = lower_expr(gl, arenas, scope, diagnostics, &mut bb_builder, expr)?;
    let v = expression::sign_or_zero_extend(gl, &mut bb_builder, v, v_ty, ty.force_net_width());

    bb_builder.drive(gl, signal, v);

    bb_builder.watch_for_ins_to(gl, bb_key);
    processes.push(section_key);
    Ok(signal)
}

fn assign_port_output<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    processes: &mut Vec<ProcessKey>,
    expr: AstId<Expr>,
    ty: VType,
) -> Result<SignalKey, ()> {
    if let Expr::Ident(ast_ident, exprs, range_expression) = arenas.get(expr)
        && exprs.is_empty()
        && range_expression.is_none()
    {
        let ident = arenas.get_ident(ast_ident.item.0);
        let Some(symbol_key) = scope.get(&ident) else {
            diagnostics.var_not_found(arenas, *ast_ident);
            return Err(());
        };
        if let SymbolVariant::Signal(_dims, signal_ty, key) = &scope.symbols[symbol_key].variant
            && *signal_ty == ty
        {
            return Ok(*key);
        }
    }

    let size = ty.force_net_width();
    let mut driving: Vec<(AstId<Expr>, Option<VariableKey>, VectorSize)> = Vec::new();
    driving.push((expr, None, size));

    let signal = gl.signals.insert(Signal {
        name: "anon_port_assignment".to_string(),
        size,
        initialize: None,
    });

    let (section_key, mut bb_builder) = new_process(gl, "port_assignment".into());
    let bb_key = bb_builder.key();

    let probed = bb_builder.probe(gl, signal);

    let mut error = false;
    while let Some((expr, offset_src, length_src)) = driving.pop() {
        match arenas.get(expr) {
            Expr::Concatenation(_) => {
                todo!()
            }
            Expr::Ident(ast_ident, exprs, range_expression) => {
                let (offset_dst, length_dst) = if range_expression.is_none() && exprs.is_empty() {
                    (bb_builder.constant_u32(gl, 0), None)
                } else if range_expression.is_none() && exprs.len() == 1 {
                    (
                        lower_expr(
                            gl,
                            arenas,
                            scope,
                            diagnostics,
                            &mut bb_builder,
                            exprs.first().unwrap(),
                        )?
                        .0,
                        None,
                    )
                } else if let Some(slice) = range_expression
                    && exprs.is_empty()
                {
                    match slice {
                        BitSlice::MsbLsb(msb, lsb) => {
                            let (_, lsb, width) =
                                msb_lsb_to_width(gl, arenas, scope, diagnostics, *msb, *lsb)?;
                            let offset = bb_builder.constant_u32(gl, lsb as u32);
                            (offset, Some(width as VectorSize))
                        }
                        BitSlice::PlusWidth(base, width) => {
                            let offset =
                                lower_expr(gl, arenas, scope, diagnostics, &mut bb_builder, *base);
                            let width = eval_constant_expr(gl, arenas, scope, diagnostics, *width);
                            let width = width?.as_integer().unwrap();
                            (offset?.0, Some(VectorSize::new(width as u32).unwrap()))
                        }
                        BitSlice::MinusWidth(base, width) => {
                            let offset =
                                lower_expr(gl, arenas, scope, diagnostics, &mut bb_builder, *base);
                            let width = eval_constant_expr(gl, arenas, scope, diagnostics, *width)?;
                            let width =
                                VectorSize::new(width.as_integer().unwrap() as u32).unwrap();
                            let width_v =
                                bb_builder.constant_u32(gl, width.checked_add(1).unwrap().get());
                            let offset = bb_builder.minus(gl, offset?.0, width_v);
                            (offset, Some(width))
                        }
                    }
                } else {
                    diagnostics.not_yet_implemented(arenas.get_span(expr), "multiple braced");
                    error = true;
                    continue;
                };

                let ident = arenas.get_ident(ast_ident.item.0);
                let Some(symbol_key) = scope.get(&ident) else {
                    diagnostics.var_not_found(arenas, *ast_ident);
                    error = true;
                    continue;
                };
                let SymbolVariant::Signal(_dims, _ty, key) = &scope.symbols[symbol_key].variant
                else {
                    diagnostics.output_expr_not_allowed(arenas.get_span(expr));
                    error = true;
                    continue;
                };

                let mut src = probed;
                if let Some(offset_src) = offset_src {
                    src = bb_builder.logical_shift_right(gl, src, offset_src);
                }
                let src = bb_builder.slice(gl, src, length_src);
                bb_builder.drive_partial(
                    gl,
                    *key,
                    src,
                    offset_dst,
                    length_dst.unwrap_or(SCALAR_VSIZE),
                );
            }

            Expr::Replication(_) => {
                diagnostics.not_yet_implemented(arenas.get_span(expr), "repetition in net assign");
                error = true;
            }

            Expr::FunctionCall(..)
            | Expr::SystemFunctionCall(..)
            | Expr::Decimal(..)
            | Expr::Sized(..)
            | Expr::Ternary(..)
            | Expr::String(..)
            | Expr::Unary(..)
            | Expr::Binary(..) => {
                diagnostics.output_expr_not_allowed(arenas.get_span(expr));
                error = true;
            }
        }
    }

    bb_builder.watch_for_ins_to(gl, bb_key);
    processes.push(section_key);

    if error {
        return Err(());
    }

    Ok(signal)
}

fn assign_net_lvalue<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    builder: &mut BasicBlockBuilder,
    lvalue: AstId<NetLValue>,
    variable: VariableKey,
    variable_ty: VType,
) -> Result<(), ()> {
    let NetLValue {
        ident,
        constant_exprs,
        constant_range_expression,
    } = arenas.get(lvalue);

    let Some(symbol_key) = scope.get(arenas.get_ident(ident.item.0)) else {
        diagnostics.var_not_found(arenas, *ident);
        return Err(());
    };

    let SymbolVariant::Signal(dims, ty, key) = &scope.symbols[symbol_key].variant else {
        panic!("not a signal");
    };
    let key = *key;
    let mut dims = &dims[..];

    let mut exprs = *constant_exprs;
    let mut arr_idx = if !dims.is_empty()
        && let Some(fst) = exprs.pop_front()
    {
        dims = &dims[1..];
        let mut leaf_arr_items = dims.iter().product::<u32>();
        let fst = eval_constant_expr(gl, arenas, scope, diagnostics, fst)?;
        let fst = fst.as_integer().unwrap();
        let mut offset = fst as u32 * leaf_arr_items;

        while let Some(dim) = dims.first()
            && let Some(expr) = exprs.pop_front()
        {
            leaf_arr_items /= *dim;
            let expr = eval_constant_expr(gl, arenas, scope, diagnostics, expr)?;
            let expr = expr.as_integer().unwrap();
            let expr = expr as u32 * leaf_arr_items;
            offset += expr;
            dims = &dims[1..];
        }

        Some(offset)
    } else {
        None
    };
    if !exprs.is_empty() {
        diagnostics.not_yet_implemented(arenas.get_range_span(exprs), "variable_lvalue::exprs");
        return Err(());
    }

    let mut range_expression = *constant_range_expression;
    if !dims.is_empty()
        && let Some(ConstantRangeExpression::Single(expr)) = range_expression.map(|e| arenas.get(e))
    {
        _ = range_expression.take();

        dims = &dims[1..];
        let leaf_arr_items = dims.iter().product::<u32>();
        let fst = eval_constant_expr(gl, arenas, scope, diagnostics, *expr)?;
        let fst = fst.as_integer().unwrap();
        let offset = fst as u32 * leaf_arr_items;

        arr_idx = Some(match arr_idx {
            None => offset,
            Some(arr_idx) => arr_idx + offset,
        });
    }

    if !dims.is_empty() {
        diagnostics.not_yet_implemented(
            arenas.get_range_span(exprs),
            "driving array without indices",
        );
        return Err(());
    }

    let size = ty.force_net_width();
    let partial = match range_expression {
        None => match arr_idx {
            None => None,
            Some(idx) => Some((builder.constant_u32(gl, idx * size.get()), size)),
        },
        Some(range_expression) => {
            let (offset, length) = match arenas.get(range_expression) {
                ConstantRangeExpression::Single(expr) => (
                    eval_constant_expr(gl, arenas, scope, diagnostics, *expr)?
                        .as_integer()
                        .unwrap(),
                    1,
                ),
                _ => todo!("MsbLsb"),
            };

            let length = VectorSize::new(length).unwrap();
            Some(match arr_idx {
                None => (builder.constant_u32(gl, offset as u32), length),
                Some(idx) => (
                    builder.constant_u32(gl, idx * size.get() + offset as u32),
                    length,
                ),
            })
        }
    };
    let size = partial.map_or(ty.force_net_width(), |(_, s)| s);
    let variable = expression::sign_or_zero_extend(gl, builder, variable, variable_ty, size);
    builder.drive_opt_partial(gl, key, variable, partial);
    Ok(())
}

fn msb_lsb_to_width<'a>(
    gl: &GlobalContext,
    arenas: &'a AstArenas,
    scope: &Scope<'a>,
    diagnostics: &mut Diagnostics,
    msb: AstId<ConstantExpr>,
    lsb: AstId<ConstantExpr>,
) -> Result<(u64, u64, VectorSize), ()> {
    let msb = eval_constant_expr(gl, arenas, scope, diagnostics, msb);
    let lsb = eval_constant_expr(gl, arenas, scope, diagnostics, lsb);

    let (Ok(VValue::SignedNet(msb)), Ok(VValue::SignedNet(lsb))) = (msb, lsb) else {
        return Err(());
    };
    let msb = msb.as_i64().unwrap();
    let lsb = lsb.as_i64().unwrap();
    Ok((
        msb as u64,
        lsb as u64,
        VectorSize::new((msb - lsb + 1) as u32).unwrap(),
    ))
}

fn range_to_width<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &Scope<'a>,
    diagnostics: &mut Diagnostics,
    range: AstId<Range>,
) -> Result<VectorSize, ()> {
    let range = arenas.get(range);
    msb_lsb_to_width(gl, arenas, scope, diagnostics, range.msb, range.lsb).map(|(_, _, w)| w)
}

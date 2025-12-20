mod diagnostics;
mod module_or_generate_item;
mod scope;
mod statement;
mod vtype;
mod vvalue;

use std::collections::{HashMap, HashSet};

use scope::Scope;

use vogls_ir::{
    BasicBlockBuilder, Bits, ConnectionDirection, GlobalContext, IntrinsicArg, IntrinsicOp,
    ProcessKey, Signal, SignalKey, Time, Type, Value, VariableKey, VectorSize, new_process,
};

use crate::ast::constant_expr::{
    ConstantExpr, ConstantMinTypMaxExpression, ConstantRangeExpression,
};
use crate::ast::expr::{BinaryOperator, BitPartSelect, BitSlice, Expr, UnaryOperator};
use crate::ast::module::{
    GenerateRegion, Module, ModuleItem, ModulePorts, NonPortModuleItem, ParamAssignment,
    ParameterDeclaration, Port, PortDeclaration, PortExpression, PortReference, Range,
};
use crate::ast::statement::{
    BlockingAssignment, DelayControl, DelayValue, EventControl, EventExpression,
    LoopStatementVariant, NetLValue, NonBlockingAssignment, ProceduralTimingControl, Statement,
    StatementOrNull, VariableAssignment, VariableLValue,
};
use crate::ast::{AstId, AstIdRange, RangeExpression};
use crate::number::Decimal;
use crate::parser::{AstArenas, TokenRange};

use self::scope::{Symbol, SymbolKey, SymbolVariant};
pub use self::vtype::{VType, VTypeKey, VTypeTable};
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
    pub ports: Vec<(&'a str, ConnectionDirection, Option<VectorSize>)>,
}
#[derive(Clone)]
pub struct ModuleParameters<'a> {
    pub lut: HashMap<&'a str, usize>,
    pub params: Vec<&'a str>,
}

#[derive(Clone)]
pub struct ModuleArgs {
    pub parameters: Vec<i64>,
    pub signals: Vec<SignalKey>,
}

pub fn fetch_module_interface<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    types: &mut VTypeTable,
    module: AstId<Module>,
    parameters: &[(&'a str, i64, TokenRange)],
    diagnostics: &mut Diagnostics,
) -> Result<(ModuleParameters<'a>, ModuleIo<'a>, Vec<i64>), ()> {
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
            let ParameterDeclaration { assignments } = arenas.get(p);
            for assignment in assignments.iter() {
                let ParamAssignment { param, constant } = arenas.get(assignment);
                let key = arenas.get_ident(param.item.0);
                let value = arenas.get(*constant);
                match value {
                    ConstantMinTypMaxExpression::Single(id) => {
                        let value =
                            eval_constant_expr(gl, arenas, types, &scope, diagnostics, *id)?;
                        let value = value.as_integer().unwrap();
                        let symbol_key = scope.symbols.insert(Symbol {
                            name: key.to_string(),
                            definition_site: arenas.get_item_span(*param),
                            ty: types.insert(VType::Integer),
                            variant: SymbolVariant::Constant(Some(value as i64)),
                        });
                        scope.push(key, symbol_key);
                        param_values.push(value as i64);
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
        scope.symbols[symbol_key].variant = SymbolVariant::Constant(Some(*value));
        param_values[*param_idx] = *value;
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
                        io.push((name, ConnectionDirection::Both, None));
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
                let (direction, range, identifiers) = match port_declaration {
                    PortDeclaration::Inout(id) => {
                        let inout = arenas.get(*id);
                        (
                            ConnectionDirection::Both,
                            inout.range,
                            inout.port_identifiers,
                        )
                    }
                    PortDeclaration::Input(id) => {
                        let input = arenas.get(*id);
                        (ConnectionDirection::In, input.range, input.port_identifiers)
                    }
                    PortDeclaration::Output(id) => {
                        let output = arenas.get(*id);
                        (ConnectionDirection::Out, output.range, output.identifiers)
                    }
                };
                let width = match range {
                    None => None,
                    Some(range) => Some(range_to_width(
                        gl,
                        arenas,
                        types,
                        &scope,
                        diagnostics,
                        range,
                    )?),
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
                    io[*idx].2 = width;
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
                let (direction, range, identifiers) = match port_declaration {
                    PortDeclaration::Inout(id) => {
                        let inout = arenas.get(*id);
                        (
                            ConnectionDirection::Both,
                            inout.range,
                            inout.port_identifiers,
                        )
                    }
                    PortDeclaration::Input(id) => {
                        let input = arenas.get(*id);
                        (ConnectionDirection::In, input.range, input.port_identifiers)
                    }
                    PortDeclaration::Output(id) => {
                        let output = arenas.get(*id);
                        (ConnectionDirection::Out, output.range, output.identifiers)
                    }
                };
                let width = match range {
                    None => None,
                    Some(range) => Some(range_to_width(
                        gl,
                        arenas,
                        types,
                        &scope,
                        diagnostics,
                        range,
                    )?),
                };

                io.extend(identifiers.iter().map(|ident| {
                    let ident = arenas.get_ident(arenas.get(ident).0);
                    (ident, direction, width)
                }));
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
    types: &mut VTypeTable,
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
            ty: types.insert(VType::Integer),
            variant: SymbolVariant::Constant(Some(*param)),
        });
        scope.push(key, symbol_key);
    }

    for ((name, _con, width), signal) in io.ports.iter().zip(&args.signals) {
        let symbol_key = scope.symbols.insert(Symbol {
            name: name.to_string(),
            // @TODO: better definition site
            definition_site: arenas.get_span(root),
            ty: types.insert(VType::net(*width)),
            variant: SymbolVariant::Signal(*signal),
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
                    types,
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
                            types,
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
                    let ParameterDeclaration { assignments } = arenas.get(*id);
                    for assignment in assignments.iter() {
                        let ParamAssignment { param: _, constant } = arenas.get(assignment);
                        let ConstantMinTypMaxExpression::Single(constant) = arenas.get(*constant)
                        else {
                            todo!();
                        };

                        let _value =
                            eval_constant_expr(gl, arenas, types, &scope, diagnostics, *constant)?;
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

fn statements_to_process<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    types: &mut VTypeTable,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    mut builder: BasicBlockBuilder,
    stmts: &[Statement],
) -> Result<BasicBlockBuilder, ()> {
    for statement in stmts.iter() {
        match statement {
            Statement::BlockingAssignment(ba) => {
                // @Incorrect
                let ba = arenas.get(*ba);
                let BlockingAssignment {
                    variable_lvalue,
                    delay_or_event_control,
                    expression,
                } = ba;
                assert!(delay_or_event_control.is_none());

                let value = lower_expr(
                    gl,
                    arenas,
                    types,
                    scope,
                    diagnostics,
                    &mut builder,
                    arenas.get(*expression),
                )?;
                assign_variable_lvalue(
                    gl,
                    arenas,
                    types,
                    scope,
                    diagnostics,
                    &mut builder,
                    *variable_lvalue,
                    value,
                )?;
            }
            Statement::CaseStatement(case_statement) => {
                builder = statement::conditional::lower_case_statement(
                    gl,
                    arenas,
                    types,
                    scope,
                    diagnostics,
                    builder,
                    *case_statement,
                )?
            }
            Statement::ConditionalStatement(conditional) => {
                builder = statement::conditional::lower(
                    gl,
                    arenas,
                    types,
                    scope,
                    diagnostics,
                    builder,
                    *conditional,
                )?
            }
            Statement::DisableStatement => todo!(),
            Statement::EventTrigger => todo!(),
            Statement::LoopStatement(ls) => {
                builder = statement::loop_statement::lower_loop_statement(
                    gl,
                    arenas,
                    types,
                    scope,
                    diagnostics,
                    builder,
                    *ls,
                )?
            }
            Statement::NonBlockingAssignment(nba) => {
                let NonBlockingAssignment {
                    variable_lvalue,
                    delay_or_event_control,
                    expression,
                } = arenas.get(*nba);
                assert!(delay_or_event_control.is_none());

                let value = lower_expr(
                    gl,
                    arenas,
                    types,
                    scope,
                    diagnostics,
                    &mut builder,
                    arenas.get(*expression),
                )?;
                assign_variable_lvalue(
                    gl,
                    arenas,
                    types,
                    scope,
                    diagnostics,
                    &mut builder,
                    *variable_lvalue,
                    value,
                )?;
            }
            Statement::ParBlock => todo!(),
            Statement::ProceduralContinuousAssignments => todo!(),
            Statement::ProceduralTimingControlStatement(ptc, statement) => {
                match arenas.get(*ptc) {
                    ProceduralTimingControl::DelayControl(delay_control) => {
                        let delay_control = arenas.get(*delay_control);
                        match delay_control {
                            DelayControl::DelayValue(value) => {
                                let value = match arenas.get(*value) {
                                    DelayValue::UnsignedNumber(value) => {
                                        let value = &arenas.decimals[value.at];
                                        let value = match value {
                                            Decimal::Small(v) => *v as usize,
                                            _ => todo!(),
                                        };
                                        value
                                    }
                                    DelayValue::Identifier(_) => {
                                        todo!()
                                        // let ScopeItem::Constant(v) = scope
                                        //     .get(&arenas.get_ident(value.0))
                                        //     .expect("unknown ident")
                                        // else {
                                        //     todo!();
                                        // };
                                        // *v as usize
                                    }
                                };

                                // @TODO:
                                // IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 159
                                //
                                // """
                                // An explicit zero delay (#0) requires that the process be
                                // suspended and added as an inactive event for the current time so
                                // that the process is resumed in the next simulation cycle in the
                                // current time.
                                // """
                                assert_ne!(value, 0);

                                builder = builder.wait(gl, Time(value as u64));
                            }
                        }
                    }
                    ProceduralTimingControl::EventControl(event_control) => {
                        builder = builder.jump(gl);
                        let start_key = builder.key();

                        let mut conditions = Vec::new();
                        let mut signals = Vec::new();
                        match arenas.get(*event_control) {
                            EventControl::EventExpression(event_expression) => {
                                let (expr, condition) = match arenas.get(*event_expression) {
                                    EventExpression::Expression(expr) => {
                                        (expr, WatchCondition::None)
                                    }
                                    EventExpression::Posedge(expr) => {
                                        (expr, WatchCondition::Posedge)
                                    }
                                    EventExpression::Negedge(expr) => {
                                        (expr, WatchCondition::Negedge)
                                    }
                                    EventExpression::OrList(_, _) => todo!(),
                                };

                                let Expr::Ident(ast_ident) = arenas.get(*expr) else {
                                    panic!("not an ident");
                                };
                                let ident = arenas.get_ident(ast_ident.item.0);
                                let Some(symbol_key) = scope.get(ident) else {
                                    diagnostics.var_not_found(arenas, *ast_ident);
                                    return Err(());
                                };
                                let SymbolVariant::Signal(key) = &scope.symbols[symbol_key].variant
                                else {
                                    panic!("not a signal");
                                };

                                conditions.push((condition, *key));
                                signals.push(*key);
                            }
                        }

                        let mut before = Vec::new();
                        for (_, signal) in &conditions {
                            before.push(builder.probe(gl, *signal));
                        }

                        builder = builder.watch(gl, signals);

                        let mut acc = builder.constant(gl, Value::Bits(Bits::Small(1, 1)));
                        for ((condition, signal), before) in conditions.into_iter().zip(before) {
                            use WatchCondition as C;

                            let cond = match condition {
                                C::Posedge => {
                                    let after = builder.probe(gl, signal);
                                    let t = builder.binary_neg(gl, before);
                                    builder.and(gl, t, after)
                                }
                                C::Negedge => {
                                    let after = builder.probe(gl, signal);
                                    let t = builder.binary_neg(gl, after);
                                    builder.and(gl, before, t)
                                }
                                C::None => builder.constant(gl, Value::Bits(Bits::Small(1, 1))),
                            };
                            acc = builder.and(gl, acc, cond);
                        }

                        builder = builder.branch_false_to(gl, acc, start_key);
                    }
                }

                if let Some(stmt) = statement {
                    let stmt = arenas.get(*stmt);
                    builder = statements_to_process(
                        gl,
                        arenas,
                        types,
                        scope,
                        diagnostics,
                        builder,
                        std::slice::from_ref(stmt),
                    )?;
                }
            }
            Statement::SeqBlock(id) => {
                let seq_block = arenas.get(*id);
                let statements = seq_block
                    .statements
                    .iter()
                    .map(|v| arenas.get(v).clone())
                    .collect::<Vec<_>>();
                builder = statements_to_process(
                    gl,
                    arenas,
                    types,
                    scope,
                    diagnostics,
                    builder,
                    &statements,
                )?;
            }
            Statement::SystemTaskEnable(id) => {
                let system_task_enable = arenas.get(*id);

                let ident = system_task_enable.system_task_identifier.item;
                let ident = &arenas.text[ident.0.start..ident.0.end];

                match ident {
                    "display" => {
                        let expressions = system_task_enable.expressions;
                        assert_eq!(expressions.len(), 1); // @Improve: Error message

                        let expr = arenas.get(expressions.first().unwrap());
                        let arg = if let Some(str_literal) = expr.into_str_literal() {
                            let str_literal = &arenas.text[str_literal.0.start..str_literal.0.end];
                            IntrinsicArg::StringLiteral(str_literal.to_string())
                        } else {
                            let var = lower_expr(
                                gl,
                                arenas,
                                types,
                                scope,
                                diagnostics,
                                &mut builder,
                                expr,
                            )?;
                            IntrinsicArg::Variable(var)
                        };

                        builder.intrinsic(gl, IntrinsicOp::Display, vec![arg]);
                    }
                    "vogls_assert_eq" | "vogls_assert_ne" => {
                        let expressions = system_task_enable.expressions;
                        assert_eq!(expressions.len(), 2); // @Improve: Error message

                        let lhs = expressions.get(0);
                        let rhs = expressions.get(1);

                        let lhs = arenas.get(lhs);
                        let rhs = arenas.get(rhs);

                        let lhs =
                            lower_expr(gl, arenas, types, scope, diagnostics, &mut builder, lhs)?;
                        let rhs =
                            lower_expr(gl, arenas, types, scope, diagnostics, &mut builder, rhs)?;

                        let (lhs, rhs) = builder.coerce_binary_bitwise_srcs(gl, lhs, rhs);

                        builder.intrinsic(
                            gl,
                            IntrinsicOp::AssertEq(ident == "vogls_assert_eq"),
                            vec![IntrinsicArg::Variable(lhs), IntrinsicArg::Variable(rhs)],
                        )
                    }
                    "finish" => builder.intrinsic(gl, IntrinsicOp::Finish, vec![]),

                    // @Incomplete: Many variants here.
                    _ => todo!(),
                }
            }
            Statement::TaskEnable => todo!(),
            Statement::WaitStatement => todo!(),
        }
    }

    Ok(builder)
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
    let mut symbols = Vec::new();
    let mut black_list = HashSet::<&str>::new();
    let mut stack = Vec::new();
    stack.push(stmts);
    while let Some(mut stmts) = stack.pop() {
        while let Some(stmt) = stmts.pop_front() {
            let stmt = arenas.get(stmt);
            match stmt {
                Statement::BlockingAssignment(id) => {
                    let ba = arenas.get(*id);
                    let lvalue = arenas.get(ba.variable_lvalue);
                    let ident = arenas.get_ident(lvalue.ident.item.0);
                    if black_list.insert(ident) {
                        symbols.push(scope.get(ident).unwrap());
                    }
                }
                Statement::CaseStatement(id) => {
                    let c = arenas.get(*id);
                    stack.push(stmts);
                    stack.extend(c.items.iter().filter_map(|c| {
                        match arenas.get(arenas.get(c).statement_or_null) {
                            StatementOrNull::Attribute(_) => None,
                            StatementOrNull::Statement(stmt) => Some(AstIdRange::single(*stmt)),
                        }
                    }));
                    break;
                }
                Statement::ConditionalStatement(id) => {
                    let c = arenas.get(*id);
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
                Statement::DisableStatement => todo!(),
                Statement::EventTrigger => todo!(),
                Statement::LoopStatement(id) => {
                    let ls = arenas.get(*id);
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
                Statement::NonBlockingAssignment(id) => {
                    let nba = arenas.get(*id);
                    let lvalue = arenas.get(nba.variable_lvalue);
                    let ident = arenas.get_ident(lvalue.ident.item.0);
                    if black_list.insert(ident) {
                        symbols.push(scope.get(ident).unwrap());
                    }
                }
                Statement::ParBlock => todo!(),
                Statement::ProceduralContinuousAssignments => todo!(),
                Statement::ProceduralTimingControlStatement(_, statement) => {
                    if let Some(statement) = statement {
                        stack.push(stmts);
                        stack.push(AstIdRange::single(*statement));
                        break;
                    }
                }
                Statement::SeqBlock(id) => {
                    let seq_block = arenas.get(*id);
                    stack.push(stmts);
                    stack.push(seq_block.statements);
                    break;
                }
                Statement::SystemTaskEnable(_) => continue,
                Statement::TaskEnable => todo!(),
                Statement::WaitStatement => todo!(),
            }
        }
    }
    symbols
}

fn lower_to_signal<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    types: &mut VTypeTable,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    processes: &mut Vec<ProcessKey>,
    expr: AstId<Expr>,
    width: Option<VectorSize>,
) -> Result<SignalKey, ()> {
    if let Expr::Ident(ast_ident) = arenas.get(expr) {
        let ident = arenas.get_ident(ast_ident.item.0);
        let Some(symbol_key) = scope.get(&ident) else {
            diagnostics.var_not_found(arenas, *ast_ident);
            return Err(());
        };
        if let SymbolVariant::Signal(key) = &scope.symbols[symbol_key].variant {
            return Ok(*key);
        }
    }

    let ty = Type::net(width);
    let signal = gl.signals.insert(Signal {
        name: "anon_port_assignment".to_string(),
        ty,
    });

    let (section_key, mut bb_builder) = new_process(gl, "port_assignment".into());
    let bb_key = bb_builder.key();
    let variable = lower_expr(
        gl,
        arenas,
        types,
        scope,
        diagnostics,
        &mut bb_builder,
        arenas.get(expr),
    )?;

    bb_builder.drive(gl, signal, variable);

    bb_builder.watch_for_ins_to(gl, bb_key);
    processes.push(section_key);
    Ok(signal)
}

fn assign_port_output<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    types: &mut VTypeTable,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    processes: &mut Vec<ProcessKey>,
    expr: AstId<Expr>,
    width: Option<VectorSize>,
) -> Result<SignalKey, ()> {
    if let Expr::Ident(ast_ident) = arenas.get(expr) {
        let ident = arenas.get_ident(ast_ident.item.0);
        let Some(symbol_key) = scope.get(&ident) else {
            diagnostics.var_not_found(arenas, *ast_ident);
            return Err(());
        };
        if let SymbolVariant::Signal(key) = &scope.symbols[symbol_key].variant {
            return Ok(*key);
        }
    }

    let mut driving: Vec<(
        AstId<Expr>,
        Option<VariableKey>,
        Option<VectorSize>,
        Option<VariableKey>,
        Option<VectorSize>,
    )> = Vec::new();
    driving.push((expr, None, width, None, width));

    let signal = gl.signals.insert(Signal {
        name: "anon_port_assignment".to_string(),
        ty: Type::net(width),
    });

    let (section_key, mut bb_builder) = new_process(gl, "port_assignment".into());
    let bb_key = bb_builder.key();

    let probed = bb_builder.probe(gl, signal);

    let mut error = false;
    while let Some((expr, offset_src, length_src, offset_dst, length_dst)) = driving.pop() {
        match arenas.get(expr) {
            Expr::BitPartSelect(bit_part_select) => {
                let BitPartSelect { subject, braced } = bit_part_select;
                let offset = lower_expr(
                    gl,
                    arenas,
                    types,
                    scope,
                    diagnostics,
                    &mut bb_builder,
                    arenas.get(*braced),
                )?;

                let offset_dst = match offset_dst {
                    None => offset,
                    Some(offset_dst) => bb_builder.plus(gl, offset_dst, offset),
                };

                driving.push((*subject, offset_src, length_src, Some(offset_dst), None));
            }
            Expr::BitSlice(subject, slice) => {
                let (offset, length) = match slice {
                    BitSlice::MsbLsb(msb, lsb) => {
                        let (_, lsb, width) =
                            msb_lsb_to_width(gl, arenas, types, scope, diagnostics, *msb, *lsb)?;
                        let offset = bb_builder.constant(gl, Value::Decimal(lsb as i64));
                        (offset, width as VectorSize)
                    }
                    BitSlice::PlusWidth(base, width) => {
                        let offset = lower_expr(
                            gl,
                            arenas,
                            types,
                            scope,
                            diagnostics,
                            &mut bb_builder,
                            arenas.get(*base),
                        );
                        let width =
                            eval_constant_expr(gl, arenas, types, scope, diagnostics, *width);
                        let width = width?.as_integer().unwrap();
                        (offset?, width as VectorSize)
                    }
                    BitSlice::MinusWidth(base, width) => {
                        let offset = lower_expr(
                            gl,
                            arenas,
                            types,
                            scope,
                            diagnostics,
                            &mut bb_builder,
                            arenas.get(*base),
                        );
                        let width =
                            eval_constant_expr(gl, arenas, types, scope, diagnostics, *width)?;
                        let width = width.as_integer().unwrap() as VectorSize;
                        let width_v = bb_builder.constant(gl, Value::Decimal((width + 1) as i64));
                        let offset = bb_builder.minus(gl, offset?, width_v);
                        (offset, width)
                    }
                };

                let offset_dst = match offset_dst {
                    None => offset,
                    Some(offset_dst) => bb_builder.plus(gl, offset_dst, offset),
                };
                let length_dst = Some(length);

                driving.push((
                    *subject,
                    offset_src,
                    length_src,
                    Some(offset_dst),
                    length_dst,
                ));
            }
            Expr::Concatenation(_) => {
                todo!()
            }
            Expr::Ident(ast_ident) => {
                let ident = arenas.get_ident(ast_ident.item.0);
                let Some(symbol_key) = scope.get(&ident) else {
                    diagnostics.var_not_found(arenas, *ast_ident);
                    error = true;
                    continue;
                };
                let SymbolVariant::Signal(key) = &scope.symbols[symbol_key].variant else {
                    diagnostics.output_expr_not_allowed(arenas.get_span(expr));
                    error = true;
                    continue;
                };

                let offset_dst = match offset_dst {
                    None => bb_builder.constant(gl, Value::Decimal(0)),
                    Some(v) => v,
                };

                let mut src = probed;
                if let Some(offset_src) = offset_src {
                    src = bb_builder.lsr(gl, src, offset_src);
                }
                let src = bb_builder.slice(gl, src, length_src.unwrap_or(1));
                bb_builder.drive_partial(gl, *key, src, offset_dst, length_dst.unwrap_or(1));
            }

            Expr::Replication(_) => {
                diagnostics.not_yet_implemented(arenas.get_span(expr), "repetition in net assign");
                error = true;
            }

            Expr::Decimal(..)
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

// fn expr_to_type<'a>(
//     gl: &mut GlobalContext,
//     scope: &Scope<'a>,
//     expr: AstId<Expr>,
//     arenas: &'a AstArenas,
//     diagnostics: &mut Diagnostics,
// ) -> Result<Type, ()> {
//     Ok(match arenas.get(expr) {
//         Expr::BitPartSelect(select) => {
//             let BitPartSelect { subject, braced } = select;
//             let subject_v = expr_to_type(gl, scope, *subject, arenas, diagnostics)?;
//             let braced_v = expr_to_type(gl, scope, *braced, arenas, diagnostics)?;
//             Type::Bits(1)
//         }
//         Expr::BitSlice(subject, slice) => {
//             let subject_v = expr_to_type(gl, scope, *subject, arenas, diagnostics)?;
//
//             match slice {
//                 BitSlice::MsbLsb(msb, lsb) => {
//                     let (_, _, width) =
//                         msb_lsb_to_width(gl, scope, *msb, *lsb, arenas, diagnostics)?;
//                     Type::Bits(width)
//                 }
//                 BitSlice::PlusWidth(_base, width) => {
//                     let width = eval_constant_expr(gl, scope, *width, arenas, diagnostics)?;
//                     Type::Bits(width as VectorSize)
//                 }
//                 BitSlice::MinusWidth(_base, width) => {
//                     let width = eval_constant_expr(gl, scope, *width, arenas, diagnostics)?;
//                     Type::Bits(width as VectorSize)
//                 }
//             }
//         }
//         Expr::Unary(op, child) => {
//             let child = expr_to_type(gl, scope, *child, arenas, diagnostics)?;
//             use UnaryOperator as O;
//             match op {
//                 O::LogicalNegation | O::BitwiseNegation => child,
//                 O::ReductionAnd
//                 | O::ReductionOr
//                 | O::ReductionNand
//                 | O::ReductionNor
//                 | O::ReductionXor
//                 | O::ReductionXnor => Type::Bits(1),
//                 O::SignPlus => todo!(),
//                 O::SignMinus => todo!(),
//             }
//         }
//         Expr::Binary(op, l, r) => {
//             let l = expr_to_type(gl, scope, *l, arenas, diagnostics)?;
//             let r = expr_to_type(gl, scope, *r, arenas, diagnostics)?;
//             _ = (l, r);
//             use BinaryOperator as O;
//             match op {
//                 O::Multiply => todo!(),
//                 O::Divide => todo!(),
//                 O::Modulus => todo!(),
//                 O::BinaryPlus => todo!(),
//                 O::BinaryMinus => todo!(),
//                 O::ShiftLeft => todo!(),
//                 O::ShiftRight => todo!(),
//                 O::GreaterThan => todo!(),
//                 O::GreaterThanEqual => todo!(),
//                 O::LessThan => todo!(),
//                 O::LessThanEqual => todo!(),
//                 O::LogicalEquality => todo!(),
//                 O::LogicalInequality => todo!(),
//                 O::CaseEquality => todo!(),
//                 O::CaseInequality => todo!(),
//                 O::BitwiseAnd => todo!(),
//                 O::BitwiseXor => todo!(),
//                 O::BitwiseXnor => todo!(),
//                 O::BitwiseOr => todo!(),
//                 O::LogicalAnd => todo!(),
//                 O::LogicalOr => todo!(),
//             }
//         }
//         Expr::Concatenation(exprs) => {
//             let mut width = 0;
//             let mut error = false;
//             for expr in exprs.iter() {
//                 match expr_to_type(gl, scope, expr, arenas, diagnostics)
//                     .and_then(|t| t.try_net_width())
//                 {
//                     Ok(ew) => width += ew,
//                     Err(_) => error = true,
//                 }
//             }
//             if error {
//                 return Err(());
//             }
//             Type::Bits(width)
//         }
//         Expr::Replication(_) => todo!(),
//         Expr::Ternary(_, _, _) => todo!(),
//         Expr::Ident(ast_ident) => {
//             let ident = arenas.get_ident(ast_ident.item.0);
//             let Some(symbol_key) = scope.get(&ident) else {
//                 diagnostics.var_not_found(arenas, *ast_ident);
//                 return Err(());
//             };
//             scope.symbols[symbol_key].ty.clone()
//         }
//         Expr::Decimal(_) => Type::Decimal,
//         Expr::Sized(sized) => {
//             let sized = &arenas.sized_numbers[sized.item.at];
//             let Some(size) = sized.size else { todo!() };
//             Type::Bits(size.as_u32())
//         }
//         Expr::String(_) => todo!(),
//     })
// }

fn lower_expr<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    types: &mut VTypeTable,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    builder: &mut BasicBlockBuilder,
    expr: &Expr,
) -> Result<VariableKey, ()> {
    Ok(match expr {
        Expr::BitPartSelect(select) => {
            let BitPartSelect { subject, braced } = select;
            let subject = arenas.get(*subject);
            let braced = arenas.get(*braced);

            let subject_v = lower_expr(gl, arenas, types, scope, diagnostics, builder, subject)?;
            let braced_v = lower_expr(gl, arenas, types, scope, diagnostics, builder, braced)?;

            builder.select_bit(gl, subject_v, braced_v)
        }
        Expr::BitSlice(subject, slice) => {
            let subject = arenas.get(*subject);
            let subject_v = lower_expr(gl, arenas, types, scope, diagnostics, builder, subject)?;

            let (lsb, width) = match slice {
                BitSlice::MsbLsb(msb, lsb) => {
                    let (_msb, lsb, width) =
                        msb_lsb_to_width(gl, arenas, types, scope, diagnostics, *msb, *lsb)?;
                    let lsb_v = builder.constant(gl, Value::Decimal(lsb as i64));
                    (lsb_v, width)
                }
                BitSlice::PlusWidth(base, width) => {
                    let lsb = lower_expr(
                        gl,
                        arenas,
                        types,
                        scope,
                        diagnostics,
                        builder,
                        arenas.get(*base),
                    )?;
                    let width = eval_constant_expr(gl, arenas, types, scope, diagnostics, *width)?;
                    let width = width.as_integer().unwrap() as VectorSize;
                    (lsb, width)
                }
                BitSlice::MinusWidth(base, width) => {
                    let width = eval_constant_expr(gl, arenas, types, scope, diagnostics, *width)?;
                    let width = width.as_integer().unwrap();
                    let width_v = builder.constant(gl, Value::Decimal(width - 1));
                    let lsb = lower_expr(
                        gl,
                        arenas,
                        types,
                        scope,
                        diagnostics,
                        builder,
                        arenas.get(*base),
                    )?;
                    let lsb = builder.minus(gl, lsb, width_v);
                    (lsb, width as VectorSize)
                }
            };

            let shifted = builder.lsr(gl, subject_v, lsb);
            builder.slice(gl, shifted, width as VectorSize)
        }
        Expr::Unary(op, child) => {
            let child = lower_expr(
                gl,
                arenas,
                types,
                scope,
                diagnostics,
                builder,
                arenas.get(*child),
            )?;
            use UnaryOperator as O;
            match op {
                O::LogicalNegation => builder.logical_neg(gl, child),
                O::BitwiseNegation => builder.binary_neg(gl, child),
                O::ReductionAnd => todo!(),
                O::ReductionOr => todo!(),
                O::ReductionNand => todo!(),
                O::ReductionNor => todo!(),
                O::ReductionXor => builder.reduce_xor(gl, child),
                O::ReductionXnor => todo!(),
                O::SignPlus => todo!(),
                O::SignMinus => todo!(),
            }
        }
        Expr::Binary(op, l, r) => {
            let l = lower_expr(
                gl,
                arenas,
                types,
                scope,
                diagnostics,
                builder,
                arenas.get(*l),
            )?;
            let r = lower_expr(
                gl,
                arenas,
                types,
                scope,
                diagnostics,
                builder,
                arenas.get(*r),
            )?;
            use BinaryOperator as O;
            match op {
                O::Multiply => builder.multiply(gl, l, r),
                O::Divide => todo!(),
                O::Modulus => todo!(),
                O::BinaryPlus => builder.plus(gl, l, r),
                O::BinaryMinus => builder.minus(gl, l, r),
                O::ShiftLeft => todo!(),
                O::ShiftRight => todo!(),
                O::GreaterThan => builder.unsigned_gt(gl, l, r),
                O::GreaterThanEqual => builder.unsigned_ge(gl, l, r),
                O::LessThan => builder.unsigned_lt(gl, l, r),
                O::LessThanEqual => builder.unsigned_le(gl, l, r),
                O::LogicalEquality => builder.equals(gl, l, r),
                O::LogicalInequality => todo!(),
                O::CaseEquality => todo!(),
                O::CaseInequality => todo!(),
                O::BitwiseAnd => builder.and(gl, l, r),
                O::BitwiseXor => builder.xor(gl, l, r),
                O::BitwiseXnor => builder.xnor(gl, l, r),
                O::BitwiseOr => builder.or(gl, l, r),
                O::LogicalAnd => todo!(),
                O::LogicalOr => todo!(),
            }
        }
        Expr::Concatenation(exprs) => {
            let Some(fst) = exprs.first() else {
                return Ok(builder.constant(gl, Value::Bits(Bits::Small(0, 0))));
            };

            let mut output = lower_expr(
                gl,
                arenas,
                types,
                scope,
                diagnostics,
                builder,
                arenas.get(fst),
            )?;
            for expr in exprs.iter().skip(1) {
                let lexpr = lower_expr(
                    gl,
                    arenas,
                    types,
                    scope,
                    diagnostics,
                    builder,
                    arenas.get(expr),
                )?;
                output = builder.concat(gl, output, lexpr);
            }
            output
        }
        Expr::Replication(_) => todo!(),
        Expr::Ternary(_, _, _) => todo!(),
        Expr::Ident(ast_ident) => {
            let ident = arenas.get_ident(ast_ident.item.0);
            let Some(symbol_key) = scope.get(&ident) else {
                diagnostics.var_not_found(arenas, *ast_ident);
                return Err(());
            };
            match &scope.symbols[symbol_key].variant {
                SymbolVariant::Constant(value) => {
                    builder.constant(gl, Value::Decimal(value.unwrap()))
                }
                SymbolVariant::Genvar(value) => {
                    builder.constant(gl, Value::Decimal(value.unwrap()))
                }
                SymbolVariant::Signal(key) => builder.probe(gl, *key),
                SymbolVariant::Variable(None) => todo!(),
                SymbolVariant::Variable(Some(key)) => *key,
            }
        }
        Expr::Decimal(decimal) => {
            let decimal = &arenas.decimals[decimal.at];
            let decimal = match decimal {
                Decimal::Small(v) => *v as i64,
                _ => todo!(),
            };

            builder.constant(gl, Value::Decimal(decimal))
        }
        Expr::Sized(sized) => {
            let sized = &arenas.sized_numbers[sized.item.at];
            let Some(size) = sized.size else { todo!() };
            let crate::number::Bits::Small(v) = sized.value else {
                todo!()
            };
            builder.constant(gl, Value::Bits(Bits::Small(v, size.as_u32())))
        }
        Expr::String(_) => todo!(),
    })
}

fn assign_variable_lvalue<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    types: &mut VTypeTable,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    builder: &mut BasicBlockBuilder,
    lvalue: AstId<VariableLValue>,
    variable: VariableKey,
) -> Result<(), ()> {
    let VariableLValue {
        ident,
        exprs,
        range_expression,
    } = arenas.get(lvalue);

    let lvalue_ident = arenas.get_ident(ident.item.0);
    let Some(symbol_key) = scope.get(&lvalue_ident) else {
        diagnostics.var_not_found(arenas, *ident);
        return Err(());
    };

    if !exprs.is_empty() {
        diagnostics.not_yet_implemented(arenas.get_range_span(*exprs), "variable_lvalue::exprs");
        return Err(());
    }

    match &mut scope.symbols[symbol_key].variant {
        SymbolVariant::Constant(_) => todo!(),
        SymbolVariant::Genvar(_) => todo!(),
        SymbolVariant::Signal(key) => {
            let key = *key;
            match range_expression {
                None => {
                    if &gl.signals[key].ty != &gl.vars[variable].ty {
                        diagnostics.warn_assign_type_mismatch(
                            arenas.get_span(lvalue),
                            gl.signals[key].ty.clone(),
                            gl.vars[variable].ty.clone(),
                        );
                    }
                    builder.drive(gl, key, variable)
                }
                Some(range_expression) => {
                    let (offset, length) = match arenas.get(*range_expression) {
                        RangeExpression::Expr(expr) => (
                            lower_expr(gl, arenas, types, scope, diagnostics, builder, expr)?,
                            1,
                        ),
                        RangeExpression::MsbLsb(_, _) => todo!("MsbLsb"),
                        RangeExpression::BasePlus(_, _) => todo!("BasePlus"),
                        RangeExpression::BaseMinus(_, _) => todo!("BaseMinus"),
                    };

                    if Type::Bits(length) != gl.vars[variable].ty {
                        diagnostics.warn_assign_type_mismatch(
                            arenas.get_span(lvalue),
                            Type::Bits(length),
                            gl.vars[variable].ty.clone(),
                        );
                    }

                    builder.drive_partial(gl, key, variable, offset, length);
                }
            }
        }
        SymbolVariant::Variable(v) => {
            if let Some(range_expression) = range_expression {
                diagnostics.not_yet_implemented(
                    arenas.get_span(*range_expression),
                    "variable_lvalue::range_expression[variable]",
                );
                return Err(());
            }

            *v = Some(variable);
            scope.assign(symbol_key, variable);
        }
    }
    Ok(())
}

fn assign_net_lvalue<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    types: &mut VTypeTable,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    builder: &mut BasicBlockBuilder,
    lvalue: AstId<NetLValue>,
    variable: VariableKey,
) -> Result<(), ()> {
    let lvalue = arenas.get(lvalue);
    let lvalue_ident = lvalue.ident.item;

    let ident = arenas.get_ident(lvalue_ident.0);
    let Some(symbol_key) = scope.get(ident) else {
        diagnostics.var_not_found(arenas, lvalue.ident);
        return Err(());
    };

    let SymbolVariant::Signal(signal_key) = &scope.symbols[symbol_key].variant else {
        panic!("not a signal");
    };
    let signal_key = *signal_key;

    if !lvalue.constant_exprs.is_empty() {
        diagnostics.not_yet_implemented(
            arenas.get_range_span(lvalue.constant_exprs),
            "net_lvalue::constant_exprs",
        );
        return Err(());
    }
    match lvalue.constant_range_expression {
        None => builder.drive(gl, signal_key, variable),
        Some(range_expression) => {
            let (offset, length) = match arenas.get(range_expression) {
                ConstantRangeExpression::Single(expr) => (
                    eval_constant_expr(gl, arenas, types, scope, diagnostics, *expr)?
                        .as_integer()
                        .unwrap(),
                    1,
                ),
                ConstantRangeExpression::MsbLsb { msb, lsb } => {
                    let (_, offset, length) =
                        msb_lsb_to_width(gl, arenas, types, scope, diagnostics, *msb, *lsb)?;
                    (offset as i64, length)
                }
            };

            let offset = builder.constant(gl, Value::Decimal(offset));
            builder.drive_partial(gl, signal_key, variable, offset, length);
        }
    }

    Ok(())
}

fn eval_constant_expr<'a>(
    _gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    types: &mut VTypeTable,
    scope: &Scope<'a>,
    diagnostics: &mut Diagnostics,
    expr: AstId<ConstantExpr>,
) -> Result<VValue, ()> {
    let expr = expr.into_expr();
    struct StackItem {
        expr: AstId<Expr>,
        dispatched: bool,
    }

    let mut error = false;
    let mut dispatch_stack: Vec<StackItem> = Vec::new();
    let mut result_stack: Vec<Option<VValue>> = Vec::new();

    dispatch_stack.push(StackItem {
        expr,
        dispatched: false,
    });

    while let Some(mut item) = dispatch_stack.pop() {
        match arenas.get(item.expr) {
            Expr::Decimal(decimal) => {
                let decimal = &arenas.decimals[decimal.at];
                let Decimal::Small(v) = decimal else {
                    result_stack.push(None);
                    diagnostics.not_yet_implemented(
                        arenas.get_span(item.expr),
                        "constant expression of this kind not yet implemented",
                    );
                    error = true;
                    continue;
                };

                result_stack.push(Some(VValue::Integer(*v as i64)));
            }
            Expr::Binary(op, lhs, rhs) => {
                if !item.dispatched {
                    item.dispatched = true;
                    dispatch_stack.push(item);
                    dispatch_stack.extend([*rhs, *lhs].into_iter().map(|expr| StackItem {
                        expr,
                        dispatched: false,
                    }));
                    continue;
                }

                let rhs = result_stack.pop().unwrap();
                let lhs = result_stack.pop().unwrap();

                let (Some(VValue::Integer(lhs)), Some(VValue::Integer(rhs))) = (lhs, rhs) else {
                    result_stack.push(None);
                    continue;
                };

                use BinaryOperator as O;
                let result = match op {
                    O::Multiply => lhs * rhs,
                    O::Divide => lhs / rhs,
                    O::Modulus => lhs % rhs,
                    O::BinaryPlus => lhs + rhs,
                    O::BinaryMinus => lhs - rhs,
                    O::ShiftLeft => lhs << rhs,
                    O::ShiftRight => lhs >> rhs,
                    O::BitwiseAnd => lhs & rhs,
                    O::BitwiseXor => lhs ^ rhs,
                    O::BitwiseXnor => !(lhs ^ rhs),
                    O::BitwiseOr => lhs | rhs,
                    O::LessThan => i64::from(lhs < rhs),
                    O::GreaterThan
                    | O::GreaterThanEqual
                    | O::LessThanEqual
                    | O::LogicalEquality
                    | O::LogicalInequality
                    | O::CaseEquality
                    | O::CaseInequality
                    | O::LogicalAnd
                    | O::LogicalOr => {
                        result_stack.push(None);
                        diagnostics.not_yet_implemented(
                            arenas.get_span(item.expr),
                            "constant expression of this kind not yet implemented",
                        );
                        error = true;
                        continue;
                    }
                };
                result_stack.push(Some(VValue::Integer(result)));
            }
            Expr::Ident(ast_ident) => {
                let ident = arenas.get_ident(ast_ident.item.0);
                let Some(symbol_key) = scope.get(ident) else {
                    result_stack.push(None);
                    diagnostics.var_not_found(arenas, *ast_ident);
                    error = true;
                    continue;
                };
                let n = match scope.symbols[symbol_key].variant {
                    SymbolVariant::Genvar(n) => n.unwrap(),
                    SymbolVariant::Constant(n) => n.unwrap(),
                    SymbolVariant::Variable(_) | SymbolVariant::Signal(_) => {
                        result_stack.push(None);
                        diagnostics.not_yet_implemented(
                            arenas.get_item_span(*ast_ident),
                            "non-constant symbol in constant-expr",
                        );
                        error = true;
                        continue;
                    }
                };
                result_stack.push(Some(VValue::Integer(n)));
            }
            Expr::Sized(..)
            | Expr::String(..)
            | Expr::BitPartSelect(_)
            | Expr::BitSlice(..)
            | Expr::Unary(..)
            | Expr::Concatenation(..)
            | Expr::Replication(..)
            | Expr::Ternary(..) => {
                result_stack.push(None);
                diagnostics.not_yet_implemented(
                    arenas.get_span(item.expr),
                    "constant expression of this kind not yet implemented",
                );
                error = true;
            }
        }
    }

    if error {
        return Err(());
    }

    assert_eq!(result_stack.len(), 1);
    let Some(value) = result_stack.pop().unwrap() else {
        panic!();
    };

    Ok(value)
}

fn msb_lsb_to_width<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    types: &mut VTypeTable,
    scope: &Scope<'a>,
    diagnostics: &mut Diagnostics,
    msb: AstId<ConstantExpr>,
    lsb: AstId<ConstantExpr>,
) -> Result<(u64, u64, VectorSize), ()> {
    let msb = eval_constant_expr(gl, arenas, types, scope, diagnostics, msb);
    let lsb = eval_constant_expr(gl, arenas, types, scope, diagnostics, lsb);

    let (Ok(VValue::Integer(msb)), Ok(VValue::Integer(lsb))) = (msb, lsb) else {
        return Err(());
    };
    Ok((msb as u64, lsb as u64, (msb - lsb + 1) as VectorSize))
}

fn range_to_width<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    types: &mut VTypeTable,
    scope: &Scope<'a>,
    diagnostics: &mut Diagnostics,
    range: AstId<Range>,
) -> Result<u32, ()> {
    let range = arenas.get(range);
    msb_lsb_to_width(gl, arenas, types, scope, diagnostics, range.msb, range.lsb).map(|(_, _, w)| w)
}

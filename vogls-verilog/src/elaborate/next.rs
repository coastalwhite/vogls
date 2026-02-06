use std::collections::VecDeque;

use vogls_frontend::ident_table::IdentTable;
use vogls_frontend::symbol_table::SymbolId;
use vogls_frontend::{VgHashMap, VgHashSet};
use vogls_ir::token_range::TokenRange;
use vogls_ir::{ConnectionDirection, GlobalContext, INTEGER_VSIZE, SCALAR_VSIZE};

use crate::ast::constant_expr::{ConstantExpr, ConstantMinTypMaxExpression};
use crate::ast::expr::{BitSlice, Expr, Replication};
use crate::ast::module::{
    AlwaysConstruct, CaseGenerateConstruct, Dimension, GenerateRegion, IfGenerateConstruct,
    InitialConstruct, IntegerDeclaration, LocalParameterDeclaration, LoopGenerateConstruct, Module,
    ModuleInstantiation, ModuleItem, ModuleOrGenerateItem, ModuleOrGenerateItemDeclaration,
    ModulePorts, NonPortModuleItem, ParamAssignment, ParameterDeclaration,
    ParameterDeclarationTyping, Port, PortDeclaration, PortExpression, PortReference, Range,
    VariableType, VariableTypeVariant,
};
use crate::ast::statement::Statement;
use crate::ast::{AstId, AstIdRange, AstItem, Identifier};
use crate::lower::{
    Diagnostics, VType, VValue, resolve_symbol_id, resolve_symbol_id_hier, try_resolve_symbol_id,
    unwrap_get_module_mut, unwrap_get_net_mut, unwrap_get_param_mut,
};
use crate::parser::AstArenas;

use super::{NetSymbol, VSymbol, VSymbolTable, port_declaration_to_info, try_table_insert};

pub enum ElabLevel {
    GenerateIf(AstId<IfGenerateConstruct>),
    GenerateLoop(AstId<LoopGenerateConstruct>),
    GenerateCase(AstId<CaseGenerateConstruct>),
    GenerateRegion(GenerateRegion),
    ModuleInstantiation(AstId<ModuleInstantiation>),
    Module(AstId<Module>),
}

pub enum InLevelSymbol {
    Param(
        AstId<ParameterDeclarationTyping>,
        AstId<ConstantMinTypMaxExpression>,
    ),
    Integer(AstId<VariableType>),
    Port(AstId<PortDeclaration>, AstItem<Identifier>),
}

pub fn elaborate_level<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    level: ElabLevel,
    scope: SymbolId,
    table: &mut VSymbolTable,
    lvl_symbols: &mut VgHashMap<SymbolId, InLevelSymbol>,
    next_levels: &mut VecDeque<(SymbolId, ElabLevel)>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    // 1. Assign a SymbolId per Symbol at level
    // 2. Resolve Symbol per SymbolId in a queue where dependencies are handled
    // 3. Add all next levels into the queue

    lvl_symbols.clear();
    match level {
        ElabLevel::GenerateIf(..) => todo!(),
        ElabLevel::GenerateLoop(..) => todo!(),
        ElabLevel::GenerateCase(..) => todo!(),
        ElabLevel::GenerateRegion(..) => todo!(),
        ElabLevel::ModuleInstantiation(..) => todo!(),
        ElabLevel::Module(module) => elaborate_module(
            gl,
            arenas,
            module,
            scope,
            table,
            lvl_symbols,
            next_levels,
            diagnostics,
        ),
    }
}

fn extend_param_decl_idents_into_scope(
    arenas: &AstArenas,
    scope: SymbolId,
    table: &mut VSymbolTable,
    lvl_symbols: &mut VgHashMap<SymbolId, InLevelSymbol>,
    typing: AstId<ParameterDeclarationTyping>,
    assignments: AstIdRange<ParamAssignment>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let mut error = false;
    for assignment in assignments.iter() {
        let ParamAssignment { param, constant } = arenas.get(assignment);
        let Ok(sid) = try_table_insert(
            arenas,
            table,
            scope,
            *param,
            super::VSymbol::Parameter(VValue::scalar_from_bool(false)),
            diagnostics,
        ) else {
            error = true;
            continue;
        };
        unwrap_get_module_mut(table, scope).parameters.push(sid);
        lvl_symbols.insert(sid, InLevelSymbol::Param(typing, *constant));
    }
    if error { Err(()) } else { Ok(()) }
}

fn elaborate_module<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    module: AstId<Module>,
    scope: SymbolId,
    table: &mut VSymbolTable,
    lvl_symbols: &mut VgHashMap<SymbolId, InLevelSymbol>,
    next_levels: &mut VecDeque<(SymbolId, ElabLevel)>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let Module {
        attribute_instances: _,
        module_identifier: _,
        module_parameter_port_list,
        ports,
        module_items,
        default_nettype: _,
    } = arenas.get(module);

    let dummy_signal = gl.signals.insert(vogls_ir::Signal {
        name: "".to_string(),
        size: SCALAR_VSIZE,
        initialize: None,
        origin: TokenRange { start: 0, end: 0 },
    });

    // 1. Assign a SymbolId to each symbol.
    let mut error = false;
    {
        if let Some(module_parameter_port_list) = module_parameter_port_list {
            for id in module_parameter_port_list.iter() {
                let ParameterDeclaration {
                    typing,
                    assignments,
                } = arenas.get(id);
                error |= extend_param_decl_idents_into_scope(
                    arenas,
                    scope,
                    table,
                    lvl_symbols,
                    *typing,
                    *assignments,
                    diagnostics,
                )
                .is_err();
            }
        }

        let mut port_idx = 0;
        match ports {
            ModulePorts::Ports(ports) => {
                for id in ports.iter() {
                    match arenas.get(id) {
                        Port::PortExpression(id) => {
                            let PortExpression { references } = arenas.get(*id);
                            let PortReference { identifier } = arenas.get(*references);
                            let symbol = NetSymbol {
                                ty: VType::SCALAR_NET,
                                dims: Vec::new(),
                                signal: dummy_signal,
                                nba: None,
                                port_idx: Some(port_idx),
                            };
                            let symbol = VSymbol::Net(symbol);
                            let Ok(sid) = try_table_insert(
                                arenas,
                                table,
                                scope,
                                *identifier,
                                symbol,
                                diagnostics,
                            ) else {
                                error = true;
                                continue;
                            };

                            unwrap_get_module_mut(table, scope)
                                .ports
                                .push((sid, ConnectionDirection::Both));
                            port_idx += 1;
                        }
                    }
                }
            }
            ModulePorts::PortDeclarations(port_declarations) => {
                for id in port_declarations.iter() {
                    use ConnectionDirection as D;
                    let (direction, identifiers) = match arenas.get(id) {
                        PortDeclaration::Inout(id) => (D::Both, arenas.get(*id).port_identifiers),
                        PortDeclaration::Input(id) => (D::In, arenas.get(*id).port_identifiers),
                        PortDeclaration::Output(id) => (D::Out, arenas.get(*id).identifiers),
                    };

                    for ident in identifiers.iter() {
                        let ident = arenas.to_item(ident);
                        let symbol = NetSymbol {
                            ty: VType::SCALAR_NET,
                            dims: Vec::new(),
                            signal: dummy_signal,
                            nba: None,
                            port_idx: Some(port_idx),
                        };
                        let symbol = VSymbol::Net(symbol);
                        let Ok(sid) =
                            try_table_insert(arenas, table, scope, ident, symbol, diagnostics)
                        else {
                            error = true;
                            continue;
                        };

                        lvl_symbols.insert(sid, InLevelSymbol::Port(id, ident));
                        unwrap_get_module_mut(table, scope)
                            .ports
                            .push((sid, direction));
                        port_idx += 1;
                    }
                }

                if error {
                    return Err(());
                }
            }
        }

        for item in module_items.iter() {
            match arenas.get(item) {
                ModuleItem::PortDeclaration(id) => {
                    let id = *id;

                    use ConnectionDirection as D;
                    let (direction, identifiers) = match arenas.get(id) {
                        PortDeclaration::Inout(id) => (D::Both, arenas.get(*id).port_identifiers),
                        PortDeclaration::Input(id) => (D::In, arenas.get(*id).port_identifiers),
                        PortDeclaration::Output(id) => (D::Out, arenas.get(*id).identifiers),
                    };

                    for ident in identifiers.iter() {
                        let Some(sid) = table.resolve(scope, arenas.get(ident).0) else {
                            diagnostics.var_not_found(arenas, arenas.to_item(ident));
                            error = true;
                            continue;
                        };
                        let VSymbol::Net(net) = &mut table[sid].content else {
                            diagnostics.not_yet_implemented(
                                arenas.get_span(ident),
                                "non-port used as port",
                            );
                            error = true;
                            continue;
                        };
                        let Some(port_idx) = net.port_idx else {
                            diagnostics.not_yet_implemented(
                                arenas.get_span(ident),
                                "non-port used as port",
                            );
                            error = true;
                            continue;
                        };

                        lvl_symbols.insert(sid, InLevelSymbol::Port(id, arenas.to_item(ident)));
                        unwrap_get_module_mut(table, scope).ports[port_idx].1 = direction;
                    }
                }
                ModuleItem::NonPortModuleItem(id) => match arenas.get(*id) {
                    NonPortModuleItem::ModuleOrGenerateItem(id) => {
                        error |= extend_module_or_generate_item_sids(
                            gl,
                            arenas,
                            *id,
                            scope,
                            table,
                            lvl_symbols,
                            next_levels,
                            diagnostics,
                        )
                        .is_err();
                    }
                    NonPortModuleItem::GenerateRegion(region) => {
                        let sid = table.insert_unlinked(
                            IdentTable::EMPTY_IDENT,
                            scope,
                            arenas.get_span(*id),
                            VSymbol::GenerateBlock(region.module_or_generate_item),
                        );
                        next_levels.push_back((ElabLevel::GenerateRegion(*region), sid));
                    }
                    NonPortModuleItem::SpecifyBlock => todo!(),
                    NonPortModuleItem::ParameterDeclaration(id) => {
                        let ParameterDeclaration {
                            typing,
                            assignments,
                        } = arenas.get(*id);
                        error |= extend_param_decl_idents_into_scope(
                            arenas,
                            scope,
                            table,
                            lvl_symbols,
                            *typing,
                            *assignments,
                            diagnostics,
                        )
                        .is_err();
                    }
                    NonPortModuleItem::SpecParamDeclaration => todo!(),
                },
            }
        }
    }

    gl.signals.remove(dummy_signal);
    if error {
        return Err(());
    }

    let mut error = false;
    {
        let mut needed_by_adjacency_list = VgHashMap::<SymbolId, Vec<SymbolId>>::default();
        needed_by_adjacency_list.reserve(lvl_symbols.len());
        for sid in lvl_symbols.keys() {
            needed_by_adjacency_list.insert(*sid, Vec::new());
        }

        let mut seen = VgHashSet::<SymbolId>::default();
        seen.reserve(lvl_symbols.len());
        for (sid, symbol) in lvl_symbols.iter() {
            symbol.extend_needed_by(
                arenas,
                *sid,
                table,
                &mut seen,
                &mut needed_by_adjacency_list,
            );
            seen.clear();
        }

        let mut poison = VgHashSet::<SymbolId>::default();
        let mut done = seen;
        while !needed_by_adjacency_list.is_empty() {
            let mut start_length = needed_by_adjacency_list.len();
            needed_by_adjacency_list.retain(|sid, needed_by| {
                let mut is_poisoned = false;
                needed_by.retain(|k| {
                    is_poisoned |= poison.contains(k);
                    !done.contains(k)
                });

                // Dependency failed to evaluate. Poison this value and continue.
                if is_poisoned {
                    done.insert(*sid);
                    poison.insert(*sid);
                    return false;
                }

                if !needed_by.is_empty() {
                    return true;
                }

                done.insert(*sid);
                if finalize_symbol(
                    gl,
                    arenas,
                    &lvl_symbols[sid],
                    *sid,
                    scope,
                    table,
                    diagnostics,
                )
                .is_err()
                {
                    error = true;
                    poison.insert(*sid);
                }
                false
            });
            if start_length == needed_by_adjacency_list.len() {
                // @TODO: Better error
                panic!("Infinite loop in module symbols");
            }
        }
    }

    if error { Err(()) } else { Ok(()) }
}

fn extend_module_or_generate_item_sids<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    id: AstId<ModuleOrGenerateItem>,
    scope: SymbolId,
    table: &mut VSymbolTable,
    lvl_symbols: &mut VgHashMap<SymbolId, InLevelSymbol>,
    next_levels: &mut VecDeque<(SymbolId, ElabLevel)>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    match arenas.get(id) {
        ModuleOrGenerateItem::ModuleOrGenerateItemDeclaration(id) => match arenas.get(*id) {
            ModuleOrGenerateItemDeclaration::Net(id) => todo!(),
            ModuleOrGenerateItemDeclaration::Reg(id) => todo!(),
            ModuleOrGenerateItemDeclaration::Integer(id) => {
                let IntegerDeclaration { variable_types } = arenas.get(*id);
                extend_variable_type_sids(
                    gl,
                    arenas,
                    *variable_types,
                    |var_type| InLevelSymbol::Integer(var_type),
                    scope,
                    table,
                    lvl_symbols,
                    diagnostics,
                )
            }
            ModuleOrGenerateItemDeclaration::Genvar(id) => todo!(),
            ModuleOrGenerateItemDeclaration::Task(id) => todo!(),
            ModuleOrGenerateItemDeclaration::Function(id) => todo!(),
        },
        ModuleOrGenerateItem::LocalParameterDeclaration(id) => {
            let LocalParameterDeclaration {
                typing,
                assignments,
            } = arenas.get(*id);
            extend_param_decl_idents_into_scope(
                arenas,
                scope,
                table,
                lvl_symbols,
                *typing,
                *assignments,
                diagnostics,
            )
        }
        ModuleOrGenerateItem::ParameterOverride => todo!(),
        ModuleOrGenerateItem::ContinuousAssign(_) | ModuleOrGenerateItem::GateInstantiation(_) => {
            Ok(())
        }
        ModuleOrGenerateItem::UdpInstantiation => todo!(),
        ModuleOrGenerateItem::ModuleInstantiation(id) => {
            todo!()
            // let ModuleInstantiation { module_identifier, parameter_value_assignment, module_instances } = arenas.get(*id);
            //     let sid = table.insert(name, parent, origin, content)
            //     next_levels.push_back(ElabLevel::ModuleInstantiation(*id));
            // Ok(())
        }
        ModuleOrGenerateItem::InitialConstruct(id) => {
            let InitialConstruct(id) = arenas.get(*id);
            extend_statements_sids(
                gl,
                arenas,
                AstIdRange::single(*id),
                scope,
                table,
                diagnostics,
            )
        }
        ModuleOrGenerateItem::AlwaysConstruct(id) => {
            let AlwaysConstruct(id) = arenas.get(*id);
            extend_statements_sids(
                gl,
                arenas,
                AstIdRange::single(*id),
                scope,
                table,
                diagnostics,
            )
        }
        ModuleOrGenerateItem::LoopGenerateConstruct(id) => {
            let sid = todo!();
            next_levels.push_back((ElabLevel::GenerateLoop(*id), sid));
        }
        ModuleOrGenerateItem::IfGenerateConstruct(id) => todo!(),
        ModuleOrGenerateItem::CaseGenerateConstruct(id) => todo!(),
    }
}

fn extend_statements_sids<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    stmts: AstIdRange<Statement>,
    scope: SymbolId,
    table: &mut VSymbolTable,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    todo!()
    // for id in stmts.iter() {
    //     match id {}
    // }
}

fn extend_variable_type_sids<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    var_types: AstIdRange<VariableType>,
    f: impl Fn(AstId<VariableType>) -> InLevelSymbol,
    scope: SymbolId,
    table: &mut VSymbolTable,
    lvl_symbols: &mut VgHashMap<SymbolId, InLevelSymbol>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let dummy_signal = gl.signals.insert(vogls_ir::Signal {
        name: "".to_string(),
        size: SCALAR_VSIZE,
        initialize: None,
        origin: TokenRange { start: 0, end: 0 },
    });
    let mut error = false;
    for var_type in var_types.iter() {
        let symbol = NetSymbol {
            ty: VType::SCALAR_NET,
            dims: Vec::new(),
            signal: dummy_signal,
            nba: None,
            port_idx: None,
        };
        let symbol = VSymbol::Net(symbol);

        let Ok(sid) = try_table_insert(
            arenas,
            table,
            scope,
            arenas.get(var_type).identifier,
            symbol,
            diagnostics,
        ) else {
            error = true;
            continue;
        };
        lvl_symbols.insert(sid, f(var_type));
    }
    gl.signals.remove(dummy_signal);
    if error { Ok(()) } else { Err(()) }
}

impl InLevelSymbol {
    pub fn extend_needed_by<'a>(
        &self,
        arenas: &'a AstArenas,
        sid: SymbolId,
        table: &VSymbolTable,
        seen: &mut VgHashSet<SymbolId>,
        needed_by_adjacency_list: &mut VgHashMap<SymbolId, Vec<SymbolId>>,
    ) {
        let scope = table[sid]
            .parent()
            .expect("in-level symbols should always have parents");

        match self {
            InLevelSymbol::Param(typing, expr) => {
                match arenas.get(*typing) {
                    ParameterDeclarationTyping::None(_, Some(range)) => {
                        let Range { msb, lsb } = arenas.get(*range);
                        for e in [*msb, *lsb] {
                            extend_constant_expr_symbol_needed_by(
                                arenas,
                                scope,
                                table,
                                sid,
                                seen,
                                needed_by_adjacency_list,
                                e,
                            );
                        }
                    }
                    ParameterDeclarationTyping::None(..)
                    | ParameterDeclarationTyping::Integer
                    | ParameterDeclarationTyping::Real
                    | ParameterDeclarationTyping::Realtime
                    | ParameterDeclarationTyping::Time => {}
                }

                let exprs: &[_] = match arenas.get(*expr) {
                    ConstantMinTypMaxExpression::Single(e) => &[*e],
                    ConstantMinTypMaxExpression::MinTypMax { min, typ, max } => &[*min, *typ, *max],
                };
                for e in exprs {
                    extend_constant_expr_symbol_needed_by(
                        arenas,
                        scope,
                        table,
                        sid,
                        seen,
                        needed_by_adjacency_list,
                        *e,
                    );
                }
            }
            InLevelSymbol::Integer(var_type) => match arenas.get(*var_type).variant {
                VariableTypeVariant::Dimensions(dims) => {
                    for dim in dims.iter() {
                        let Dimension { lhs, rhs } = arenas.get(dim);
                        for e in [*lhs, *rhs] {
                            extend_constant_expr_symbol_needed_by(
                                arenas,
                                scope,
                                table,
                                sid,
                                seen,
                                needed_by_adjacency_list,
                                e,
                            );
                        }
                    }
                }
                VariableTypeVariant::ConstantExpr(e) => {
                    extend_constant_expr_symbol_needed_by(
                        arenas,
                        scope,
                        table,
                        sid,
                        seen,
                        needed_by_adjacency_list,
                        e,
                    );
                }
            },
            InLevelSymbol::Port(decl, _) => {
                let range = match arenas.get(*decl) {
                    PortDeclaration::Inout(id) => arenas.get(*id).range,
                    PortDeclaration::Input(id) => arenas.get(*id).range,
                    PortDeclaration::Output(id) => arenas.get(*id).range,
                };

                if let Some(range) = range {
                    let Range { msb, lsb } = arenas.get(range);
                    for e in [*msb, *lsb] {
                        extend_constant_expr_symbol_needed_by(
                            arenas,
                            scope,
                            table,
                            sid,
                            seen,
                            needed_by_adjacency_list,
                            e,
                        );
                    }
                }
            }
        }
    }
}

pub fn extend_constant_expr_symbol_needed_by<'a>(
    arenas: &'a AstArenas,
    scope: SymbolId,
    table: &VSymbolTable,
    sid: SymbolId,
    seen: &mut VgHashSet<SymbolId>,
    needed_by_adjacency_list: &mut VgHashMap<SymbolId, Vec<SymbolId>>,
    expr: AstId<ConstantExpr>,
) {
    let expr = expr.into_expr();
    let mut dispatch_stack: Vec<AstId<Expr>> = Vec::new();

    dispatch_stack.push(expr);

    while let Some(item) = dispatch_stack.pop() {
        match arenas.get(item) {
            Expr::Unary(_, subexpr) => dispatch_stack.push(*subexpr),
            Expr::Binary(_, lhs, rhs) => dispatch_stack.extend([*lhs, *rhs]),
            Expr::Concatenation(exprs) => dispatch_stack.extend(exprs.iter()),
            Expr::Replication(replication) => {
                let Replication {
                    constant_expr,
                    exprs,
                } = replication;
                dispatch_stack.push(constant_expr.into_expr());
                dispatch_stack.extend(exprs.iter())
            }
            Expr::Ternary(condition, truthy, falsy) => {
                dispatch_stack.extend([*condition, *truthy, *falsy])
            }

            Expr::Ident(ident, exprs, bit_slice) => {
                dispatch_stack.extend(exprs.iter());
                match bit_slice {
                    None => {}
                    Some(BitSlice::MsbLsb(msb, lsb)) => {
                        dispatch_stack.extend([msb.into_expr(), lsb.into_expr()])
                    }
                    Some(BitSlice::PlusWidth(lsb, width)) => {
                        dispatch_stack.extend([*lsb, width.into_expr()])
                    }
                    Some(BitSlice::MinusWidth(msb, width)) => {
                        dispatch_stack.extend([*msb, width.into_expr()])
                    }
                }

                if let Some(ident_sid) = resolve_symbol_id_hier(scope, table, arenas, *ident)
                    && let Some(needed_by) = needed_by_adjacency_list.get_mut(&ident_sid)
                    && seen.insert(ident_sid)
                {
                    needed_by.push(sid);
                };
            }
            Expr::FunctionCall(ident, exprs) => {
                dispatch_stack.extend(exprs.iter());

                if let Some(ident_sid) = resolve_symbol_id_hier(scope, table, arenas, *ident)
                    && let Some(needed_by) = needed_by_adjacency_list.get_mut(&ident_sid)
                    && seen.insert(ident_sid)
                {
                    needed_by.push(sid);
                };
            }
            Expr::SystemFunctionCall(_, exprs) => {
                _ = exprs.map(|exprs| dispatch_stack.extend(exprs.iter()))
            }
            Expr::Decimal(..) | Expr::Sized(..) | Expr::String(..) => {}
        }
    }
}

pub fn finalize_symbol<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    symbol: &InLevelSymbol,
    sid: SymbolId,
    scope: SymbolId,
    table: &mut VSymbolTable,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    match symbol {
        InLevelSymbol::Param(typing, constant_expr) => {
            let (typing, constant_expr) = (*typing, *constant_expr);
            use ParameterDeclarationTyping as T;
            let (_, _, ty) = match arenas.get(typing) {
                T::None(signed, range) => match range {
                    None => (0, 0, None),
                    Some(ast_range) => {
                        let (msb, lsb, width) = super::eval_constant_range(
                            gl,
                            arenas,
                            scope,
                            table,
                            diagnostics,
                            *ast_range,
                        )?;
                        (msb, lsb, Some(VType::net(width, *signed)))
                    }
                },
                T::Integer => (31, 0, Some(VType::SignedNet(INTEGER_VSIZE))),
                T::Real | T::Realtime | T::Time => {
                    diagnostics.not_yet_implemented(
                        arenas.get_span(typing),
                        "real / realtime / time parameter",
                    );
                    return Err(());
                }
            };

            let mut value = match arenas.get(constant_expr) {
                ConstantMinTypMaxExpression::Single(id) => {
                    super::eval_constant_expr_elab(gl, arenas, scope, table, diagnostics, *id)?
                }
                ConstantMinTypMaxExpression::MinTypMax { .. } => todo!(),
            };

            let width = ty.map_or_else(|| value.ty().force_net_width(), |ty| ty.force_net_width());
            value = value.truncate_or_extend(width);

            *unwrap_get_param_mut(table, sid) = value;
        }
        InLevelSymbol::Integer(id) => {
            let VariableType {
                identifier,
                variant,
            } = arenas.get(*id);
            let dims = match variant {
                VariableTypeVariant::Dimensions(dimensions) => {
                    dims_to_array_elab(gl, arenas, parent, table, diagnostics, *dimensions)?
                }
                VariableTypeVariant::ConstantExpr(_) => Vec::new(),
            };
            let signal = new_signal(gl, arenas, &ty, &dims, *identifier);
            let (ty, _, _) = port_declaration_to_info(gl, arenas, id, scope, table, diagnostics)?;
            let net = unwrap_get_net_mut(table, sid);
            net.signal = super::new_signal(gl, arenas, &ty, &[], *ident);
            net.ty = ty;
        }
        InLevelSymbol::Port(id, ident) => {
            let id = *id;
            let (ty, _, _) = port_declaration_to_info(gl, arenas, id, scope, table, diagnostics)?;
            let net = unwrap_get_net_mut(table, sid);
            net.signal = super::new_signal(gl, arenas, &ty, &[], *ident);
            net.ty = ty;
        }
    }

    Ok(())
}

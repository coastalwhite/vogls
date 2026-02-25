use std::collections::VecDeque;
use std::sync::Arc;

use vogls_frontend::ident_table::{IdentId, IdentTable};
use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::token_range::TokenRange;
use vogls_ir::{ConnectionDirection, GlobalContext, INTEGER_VSIZE, SCALAR_VSIZE, SignalKey};
use vogls_utils::{IndexMap, VgHashMap, VgHashSet};

use crate::ast::constant_expr::{ConstantExpr, ConstantMinTypMaxExpression};
use crate::ast::expr::{BitSlice, Expr, Replication};
use crate::ast::module::{
    AlwaysConstruct, BlockItemDeclaration, CaseGenerateConstruct, CaseGenerateItem,
    CaseGeneratePattern, Dimension, FunctionDeclaration, FunctionRangeOrType, GenerateBlock,
    GenerateRegion, GenvarAssignment, GenvarDeclaration, IfGenerateConstruct, InitialConstruct,
    IntegerDeclaration, LocalParameterDeclaration, LoopGenerateConstruct, Module,
    ModuleInstantiation, ModuleItem, ModuleOrGenerateItem, ModuleOrGenerateItemContent,
    ModuleOrGenerateItemDeclaration, ModulePorts, NamedParameterAssignment, NetDeclAssignment,
    NetDeclaration, NetDeclarationNets, NetIdent, NetType, NonPortModuleItem, ParamAssignment,
    ParameterDeclaration, ParameterDeclarationTyping, ParameterValueAssignment, Port,
    PortDeclaration, PortExpression, PortReference, Range, RegDeclaration, TaskDeclaration,
    TaskPortItem, TaskPortItemContent, TfType, VariableType, VariableTypeVariant,
};
use crate::ast::statement::{
    Block, ConditionalStatement, SeqBlock, Statement, StatementContent, StatementOrNull,
};
use crate::ast::{AstId, AstIdRange, AstItem, Identifier};
use crate::elaborate::{FunctionSymbol, TaskSymbol};
use crate::lower::{
    Diagnostics, VType, VValue, resolve_symbol_id_hier, try_resolve_symbol_id, unwrap_get_module,
    unwrap_get_module_mut, unwrap_get_net_mut, unwrap_get_param_mut,
};
use crate::parser::AstArenas;

use super::{
    ModuleSymbol, NetSymbol, VSymbol, VSymbolTable, port_declaration_to_info, try_table_insert,
};

pub enum ElabLevel {
    GenerateIf(AstId<IfGenerateConstruct>),
    GenerateLoop(AstId<LoopGenerateConstruct>),
    GenerateCase(AstId<CaseGenerateConstruct>),
    GenerateRegion(GenerateRegion),
    GenerateBlock(AstIdRange<ModuleOrGenerateItem>),
    Module(AstId<Module>),
}

#[derive(Clone, Copy)]
pub enum InLevelSymbol {
    Param(
        AstId<ParameterDeclarationTyping>,
        AstId<ConstantMinTypMaxExpression>,
        Option<SymbolId>,
    ),
    ModuleInstance(Option<AstId<ParameterValueAssignment>>, AstId<Module>),
    Integer(AstId<VariableType>),
    Reg(bool, Option<AstId<Range>>, AstId<VariableType>),
    Net(
        AstId<NetDeclaration>,
        Option<AstIdRange<Dimension>>,
        AstItem<Identifier>,
    ),
    Port(AstId<PortDeclaration>, AstItem<Identifier>),
    Task(AstId<TaskDeclaration>),
    Function(AstId<FunctionDeclaration>),
}

pub struct ElaborationState<'a> {
    lvl_symbols: IndexMap<SymbolId, InLevelSymbol>,
    next_levels: VecDeque<(SymbolId, ElabLevel)>,
    marked: VgHashSet<SymbolId>,

    needs_adjacency_list: Vec<(SymbolId, usize, usize)>,
    needs_adjacency_list_items: Vec<SymbolId>,

    dummy_signal: SignalKey,

    /// Scratchpad for traversing the constant expressions.
    dispatch_stack: Vec<AstId<Expr>>,
    stmt_dispatch_stack: Vec<(SymbolId, AstIdRange<Statement>)>,

    module_lut: &'a VgHashMap<IdentId, AstId<Module>>,
}

impl ElaborationState<'_> {
    pub fn insert_lvl_symbol(&mut self, sid: SymbolId, symbol: InLevelSymbol) {
        assert!(self.lvl_symbols.insert(sid, symbol).is_none());
    }
}

/// Elaborate from the top-level module down.
///
/// This resolves symbols and fill the symbol table level-by-level as specified in the Verilog
/// Specification section on elaboration. This is done by keeping a list of next levels to process
/// and elaborating symbols at each level.
///
/// Per level, the level-symbols are enumerated three times.
/// 1. Walk the AST to get the identifier and type of all symbols.
/// 2. Order the symbols to create a dependency graph. For instance, if the size of a net `a`
///    depends on the value of a parameter `b`. Then, symbol `a` is dependent on symbol `b`.
/// 3. Walk dependency graph and finalize symbols of which all dependencies are ready.
///
/// This allows for symbols to be defined and used out-of-order in the AST, but still resolve
/// correctly. This also makes sure that functions can be used during the evaluation of constant
/// expressions.
pub fn elaborate<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    top_level: AstId<Module>,
    module_lut: &VgHashMap<IdentId, AstId<Module>>,
    diagnostics: &mut Diagnostics,
) -> Result<VSymbolTable, ()> {
    let dummy_signal = gl.signals.insert(vogls_ir::Signal {
        name: "".to_string(),
        size: SCALAR_VSIZE,
        initialize: None,
        origin: TokenRange { start: 0, end: 0 },
    });
    gl.signals.remove(dummy_signal);
    let mut st = ElaborationState {
        lvl_symbols: Default::default(),
        next_levels: VecDeque::new(),
        marked: VgHashSet::default(),
        needs_adjacency_list: Vec::new(),
        needs_adjacency_list_items: Vec::new(),
        dummy_signal,
        dispatch_stack: Vec::new(),
        stmt_dispatch_stack: Vec::new(),
        module_lut: &module_lut,
    };

    let mut table = VSymbolTable::default();
    let tlm_ident = arenas.get(top_level).module_identifier;
    let tlm = tlm_ident.item.0;
    let module_symbol = ModuleSymbol {
        module: tlm,
        ports: Vec::new(),
        parameters: Vec::new(),
        parameter_overrides: Arc::new(VgHashMap::default()),
        parameter_override_values: Arc::new(Vec::new()),
        contains_specify: false,
    };
    let tlm_sid = table
        .insert_root(
            tlm,
            arenas.get_item_span(tlm_ident),
            VSymbol::Module(module_symbol),
        )
        .expect("No collisions possible, first symbol.");

    st.next_levels
        .push_back((tlm_sid, ElabLevel::Module(top_level)));

    let mut error = false;
    while let Some((scope, lvl)) = st.next_levels.pop_front() {
        st.lvl_symbols.clear();
        st.needs_adjacency_list_items.clear();

        let mut lvl_error = false;
        match lvl {
            ElabLevel::GenerateIf(id) => {
                lvl_error |=
                    extend_generate_if_sids(gl, arenas, scope, &mut table, &mut st, id, diagnostics)
                        .is_err()
            }
            ElabLevel::GenerateLoop(id) => {
                lvl_error |= extend_generate_loop_sids(
                    gl,
                    arenas,
                    scope,
                    &mut table,
                    &mut st,
                    id,
                    diagnostics,
                )
                .is_err()
            }
            ElabLevel::GenerateCase(id) => {
                lvl_error |= extend_generate_case_sids(
                    gl,
                    arenas,
                    scope,
                    &mut table,
                    &mut st,
                    id,
                    diagnostics,
                )
                .is_err()
            }
            ElabLevel::GenerateRegion(region) => {
                for item in region.module_or_generate_item.iter() {
                    lvl_error |= extend_module_or_generate_item_sids(
                        arenas,
                        item,
                        scope,
                        &mut table,
                        &mut st,
                        diagnostics,
                    )
                    .is_err();
                }
            }
            ElabLevel::GenerateBlock(mod_or_gen_items) => {
                for item in mod_or_gen_items.iter() {
                    lvl_error |= extend_module_or_generate_item_sids(
                        arenas,
                        item,
                        scope,
                        &mut table,
                        &mut st,
                        diagnostics,
                    )
                    .is_err();
                }
            }
            ElabLevel::Module(module) => {
                lvl_error |=
                    elaborate_module(arenas, module, scope, &mut table, &mut st, diagnostics)
                        .is_err();
            }
        };

        if lvl_error {
            error = true;
            continue;
        }

        // Evaluate the level symbols in the topological order.
        {
            assert!(st.needs_adjacency_list.is_empty());
            st.needs_adjacency_list.reserve(st.lvl_symbols.len());

            st.marked.reserve(st.lvl_symbols.len());
            for i in 0..st.lvl_symbols.len() {
                let (&sid, &symbol) = st.lvl_symbols.at(i);
                st.needs_adjacency_list.push((
                    sid,
                    st.needs_adjacency_list_items.len(),
                    st.needs_adjacency_list_items.len(),
                ));
                symbol.extend_needs(arenas, sid, &table, &mut st);
                st.needs_adjacency_list.last_mut().unwrap().2 = st.needs_adjacency_list_items.len();
                st.marked.clear();
            }

            let mut poison = VgHashSet::<SymbolId>::default();
            while !st.needs_adjacency_list.is_empty() {
                let start_length = st.needs_adjacency_list.len();
                st.needs_adjacency_list.retain_mut(|(sid, start, end)| {
                    let mut is_poisoned = false;
                    let mut new_end = *start;
                    for i in *start..*end {
                        let k = st.needs_adjacency_list_items[i];
                        st.needs_adjacency_list_items[new_end] = k;
                        is_poisoned |= poison.contains(&k);
                        new_end += usize::from(!st.marked.contains(&k));
                    }
                    *end = new_end;

                    // Dependency failed to evaluate. Poison this value and continue.
                    if is_poisoned {
                        st.marked.insert(*sid);
                        poison.insert(*sid);
                        return false;
                    }

                    if start != end {
                        return true;
                    }

                    st.marked.insert(*sid);
                    let lvl_symbol = &st.lvl_symbols[*sid];
                    if finalize_symbol(
                        gl,
                        arenas,
                        &lvl_symbol,
                        *sid,
                        scope,
                        &mut table,
                        &mut st.next_levels,
                        diagnostics,
                    )
                    .is_err()
                    {
                        error = true;
                        poison.insert(*sid);
                    }
                    false
                });

                // If we are not able to resolve any symbol in this iteration, there is at least
                // one elaboration loop and we error out.
                if start_length == st.needs_adjacency_list.len() {
                    for (s, _, _) in &st.needs_adjacency_list {
                        diagnostics.not_yet_implemented(
                            table[*s].origin(),
                            "Symbol is involved in elaboration loop.",
                        );
                    }
                    return Err(());
                }
            }
        }
    }

    if error { Err(()) } else { Ok(table) }
}

fn extend_opt_generate_block_sids<'a>(
    arenas: &'a AstArenas,
    scope: SymbolId,
    table: &mut VSymbolTable,
    st: &mut ElaborationState,
    id: AstId<Option<GenerateBlock>>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let Some(blk) = arenas.get(id) else {
        return Ok(());
    };

    let items = blk.module_or_generate_items();
    let symbol = VSymbol::GenerateBlock(items);
    let sid = match blk.ident() {
        None => table.insert_unlinked(IdentTable::EMPTY_IDENT, scope, arenas.get_span(id), symbol),
        Some(ident) => try_table_insert(arenas, table, scope, ident, symbol, diagnostics)?,
    };

    let mut error = false;
    for item in items.iter() {
        error |=
            extend_module_or_generate_item_sids(arenas, item, sid, table, st, diagnostics).is_err();
    }
    if error { Err(()) } else { Ok(()) }
}

fn extend_generate_if_sids<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: SymbolId,
    table: &mut VSymbolTable,
    st: &mut ElaborationState,
    id: AstId<IfGenerateConstruct>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let IfGenerateConstruct {
        condition,
        truthy,
        falsy,
    } = arenas.get(id);
    let condition =
        super::eval_constant_expr_elab(gl, arenas, scope, &table, diagnostics, *condition)?;

    let blk = if condition.to_logical() {
        *truthy
    } else {
        match falsy {
            None => return Ok(()),
            Some(blk) => *blk,
        }
    };
    extend_opt_generate_block_sids(arenas, scope, table, st, blk, diagnostics)
}

fn extend_generate_loop_sids<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: SymbolId,
    table: &mut VSymbolTable,
    st: &mut ElaborationState,
    id: AstId<LoopGenerateConstruct>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let LoopGenerateConstruct {
        initialization,
        condition,
        iteration,
        block,
    } = arenas.get(id);

    let GenvarAssignment {
        ident: initialization_ident,
        expr: initialization,
    } = arenas.get(*initialization);
    let GenvarAssignment {
        ident: iteration_ident,
        expr: iteration,
    } = arenas.get(*iteration);

    if initialization_ident.item.0 != iteration_ident.item.0 {
        diagnostics.not_yet_implemented(
            arenas.get_span(*initialization),
            "initialization and iteration assignment identifier are different",
        );
        return Err(());
    }
    let genvar_sid =
        try_resolve_symbol_id(scope, table, arenas, *initialization_ident, diagnostics)?;
    let VSymbol::GenVar = &table[genvar_sid].content else {
        diagnostics.not_yet_implemented(
            arenas.get_span(*initialization),
            "non-genvar used as genvar",
        );
        return Err(());
    };

    let mut value =
        super::eval_constant_expr_elab(gl, arenas, scope, table, diagnostics, *initialization)?;

    let (mod_or_gen_items, block_ident_ast) = match arenas.get(*block) {
        GenerateBlock::ModuleOrGenerateItem(id) => (AstIdRange::single(*id), None),
        GenerateBlock::BeginEnd(ident, mod_or_gen_items) => (*mod_or_gen_items, *ident),
    };

    let loop_sid = match block_ident_ast {
        Some(block_ident) => try_table_insert(
            arenas,
            table,
            scope,
            block_ident,
            VSymbol::GenerateBlocks,
            diagnostics,
        )?,
        None => table.insert_unlinked(
            IdentTable::EMPTY_IDENT,
            scope,
            arenas.get_range_span(mod_or_gen_items),
            VSymbol::GenerateBlocks,
        ),
    };

    loop {
        let iter_sid = table.insert_unlinked(
            IdentTable::EMPTY_IDENT,
            loop_sid,
            table[loop_sid].origin(),
            VSymbol::GenerateBlock(mod_or_gen_items),
        );

        let genvar_constant = table
            .insert(
                initialization_ident.item.0,
                iter_sid,
                arenas.get_item_span(*initialization_ident),
                VSymbol::Parameter(value.clone()),
            )
            .expect("No other idents in this block yet");

        let c =
            super::eval_constant_expr_elab(gl, arenas, iter_sid, table, diagnostics, *condition)?;
        if !c.to_logical() {
            table.pop_last_inserted(genvar_constant);
            table.pop_last_inserted(iter_sid);
            break;
        }

        st.next_levels
            .push_back((iter_sid, ElabLevel::GenerateBlock(mod_or_gen_items)));

        value =
            super::eval_constant_expr_elab(gl, arenas, iter_sid, table, diagnostics, *iteration)?;
    }
    Ok(())
}

fn extend_generate_case_sids<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: SymbolId,
    table: &mut VSymbolTable,
    st: &mut ElaborationState,
    id: AstId<CaseGenerateConstruct>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let CaseGenerateConstruct { value, items } = arenas.get(id);
    let value = super::eval_constant_expr_elab(gl, arenas, scope, table, diagnostics, *value)?;

    for item in items.iter() {
        let CaseGenerateItem { pattern, block } = arenas.get(item);
        let mut is_selected = false;
        match pattern {
            CaseGeneratePattern::Default => is_selected = true,
            CaseGeneratePattern::Exprs(exprs) => {
                for expr in exprs.iter() {
                    let expr_value = super::eval_constant_expr_elab(
                        gl,
                        arenas,
                        scope,
                        table,
                        diagnostics,
                        expr,
                    )?;
                    let expr_value = expr_value.truncate_or_extend(value.ty().force_net_width());
                    if value.clone().logical_equal(expr_value) {
                        is_selected = true;
                    }
                }
            }
        };

        if is_selected {
            return extend_opt_generate_block_sids(arenas, scope, table, st, *block, diagnostics);
        }
    }

    Ok(())
}

fn extend_param_decl_idents_into_scope(
    arenas: &AstArenas,
    scope: SymbolId,
    table: &mut VSymbolTable,
    st: &mut ElaborationState,
    typing: AstId<ParameterDeclarationTyping>,
    assignments: AstIdRange<ParamAssignment>,
    parameter_type: ParameterType,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let module_sid = match parameter_type {
        ParameterType::Local => None,
        ParameterType::NonLocal => {
            let mut module_sid = scope;
            while !matches!(table[module_sid].content, VSymbol::Module(_)) {
                module_sid = table[module_sid].parent().unwrap();
            }
            Some(module_sid)
        }
    };
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
        if let Some(module_sid) = module_sid {
            unwrap_get_module_mut(table, module_sid)
                .parameters
                .push(sid);
        }
        st.insert_lvl_symbol(sid, InLevelSymbol::Param(typing, *constant, module_sid));
    }
    if error { Err(()) } else { Ok(()) }
}

pub enum ParameterType {
    NonLocal,
    Local,
}

fn elaborate_module<'a>(
    arenas: &'a AstArenas,
    module: AstId<Module>,
    scope: SymbolId,
    table: &mut VSymbolTable,
    st: &mut ElaborationState,
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

    // 1. Assign a SymbolId to each symbol.
    let mut error = false;
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
                st,
                *typing,
                *assignments,
                ParameterType::NonLocal,
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
                            signal: st.dummy_signal,
                            nba: None,
                            specify_proxy: None,
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
                        signal: st.dummy_signal,
                        nba: None,
                        specify_proxy: None,
                        port_idx: Some(port_idx),
                    };
                    let symbol = VSymbol::Net(symbol);
                    let Ok(sid) =
                        try_table_insert(arenas, table, scope, ident, symbol, diagnostics)
                    else {
                        error = true;
                        continue;
                    };

                    st.insert_lvl_symbol(sid, InLevelSymbol::Port(id, ident));
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
                        diagnostics
                            .not_yet_implemented(arenas.get_span(ident), "non-port used as port");
                        error = true;
                        continue;
                    };
                    let Some(port_idx) = net.port_idx else {
                        diagnostics
                            .not_yet_implemented(arenas.get_span(ident), "non-port used as port");
                        error = true;
                        continue;
                    };

                    st.insert_lvl_symbol(sid, InLevelSymbol::Port(id, arenas.to_item(ident)));
                    unwrap_get_module_mut(table, scope).ports[port_idx].1 = direction;
                }
            }
            ModuleItem::NonPortModuleItem(id) => match arenas.get(*id) {
                NonPortModuleItem::ModuleOrGenerateItem(id) => {
                    error |= extend_module_or_generate_item_sids(
                        arenas,
                        *id,
                        scope,
                        table,
                        st,
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
                    st.next_levels
                        .push_back((sid, ElabLevel::GenerateRegion(*region)));
                }
                NonPortModuleItem::ParameterDeclaration(id) => {
                    let ParameterDeclaration {
                        typing,
                        assignments,
                    } = arenas.get(*id);
                    error |= extend_param_decl_idents_into_scope(
                        arenas,
                        scope,
                        table,
                        st,
                        *typing,
                        *assignments,
                        ParameterType::NonLocal,
                        diagnostics,
                    )
                    .is_err();
                }
                NonPortModuleItem::SpecParamDeclaration => todo!(),

                // Specify blocks are ignored during elaboration and are expanded on after
                // elaboration.
                NonPortModuleItem::SpecifyBlock(_) => {
                    unwrap_get_module_mut(table, scope).contains_specify = true;
                }
            },
        }
    }

    if error { Err(()) } else { Ok(()) }
}

fn extend_module_or_generate_item_sids<'a>(
    arenas: &'a AstArenas,
    id: AstId<ModuleOrGenerateItem>,
    scope: SymbolId,
    table: &mut VSymbolTable,
    st: &mut ElaborationState,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    match arenas.get(id).content {
        ModuleOrGenerateItemContent::ModuleOrGenerateItemDeclaration(id) => match arenas.get(id) {
            ModuleOrGenerateItemDeclaration::Net(id) => {
                let NetDeclaration {
                    net_type,
                    signed: _,
                    range: _,
                    nets,
                } = arenas.get(*id);

                if !matches!(net_type.item, NetType::Wire) {
                    diagnostics.not_yet_implemented(
                        arenas.get_item_span(*net_type),
                        "this kind of net is not yet implemented",
                    );
                    return Err(());
                }

                let mut error = false;
                match nets {
                    NetDeclarationNets::Idents(idents) => {
                        for net_ident in idents.iter() {
                            let NetIdent { ident, dimension } = arenas.get(net_ident);

                            if let Some(sid) = table.resolve(scope, ident.item.0) {
                                // @Hack.
                                // Verilog allows shadowing ports with wires.
                                let InLevelSymbol::Port(..) = &mut st.lvl_symbols[sid] else {
                                    error = true;
                                    continue;
                                };

                                continue;
                            }

                            let symbol = NetSymbol {
                                ty: VType::SCALAR_NET,
                                dims: Vec::new(),
                                signal: st.dummy_signal,
                                nba: None,
                                specify_proxy: None,
                                port_idx: None,
                            };
                            let symbol = VSymbol::Net(symbol);

                            let Ok(sid) =
                                try_table_insert(arenas, table, scope, *ident, symbol, diagnostics)
                            else {
                                error = true;
                                continue;
                            };
                            st.insert_lvl_symbol(
                                sid,
                                InLevelSymbol::Net(*id, Some(*dimension), *ident),
                            );
                        }
                    }
                    NetDeclarationNets::Assignments(assignments) => {
                        for assignment in assignments.iter() {
                            let NetDeclAssignment { ident, expr: _ } = arenas.get(assignment);

                            if let Some(sid) = table.resolve(scope, ident.item.0) {
                                // @Hack.
                                // Verilog allows shadowing ports with wires.
                                let InLevelSymbol::Port(..) = &mut st.lvl_symbols[sid] else {
                                    error = true;
                                    continue;
                                };

                                continue;
                            }

                            let symbol = NetSymbol {
                                ty: VType::SCALAR_NET,
                                dims: Vec::new(),
                                signal: st.dummy_signal,
                                nba: None,
                                specify_proxy: None,
                                port_idx: None,
                            };
                            let symbol = VSymbol::Net(symbol);

                            let Ok(sid) =
                                try_table_insert(arenas, table, scope, *ident, symbol, diagnostics)
                            else {
                                error = true;
                                continue;
                            };
                            st.insert_lvl_symbol(sid, InLevelSymbol::Net(*id, None, *ident));
                        }
                    }
                }
                if error { Err(()) } else { Ok(()) }
            }
            ModuleOrGenerateItemDeclaration::Reg(id) => {
                let RegDeclaration {
                    signed,
                    range,
                    variable_types,
                } = arenas.get(*id);
                extend_variable_type_sids(
                    arenas,
                    *variable_types,
                    |var_type| InLevelSymbol::Reg(*signed, *range, var_type),
                    scope,
                    table,
                    st,
                    diagnostics,
                )
            }
            ModuleOrGenerateItemDeclaration::Integer(id) => {
                let IntegerDeclaration { variable_types } = arenas.get(*id);
                extend_variable_type_sids(
                    arenas,
                    *variable_types,
                    |var_type| InLevelSymbol::Integer(var_type),
                    scope,
                    table,
                    st,
                    diagnostics,
                )
            }
            ModuleOrGenerateItemDeclaration::Genvar(id) => {
                let GenvarDeclaration { identifiers } = arenas.get(*id);
                let mut error = false;
                for ident in identifiers.iter() {
                    error |= try_table_insert(
                        arenas,
                        table,
                        scope,
                        arenas.to_item(ident),
                        VSymbol::GenVar,
                        diagnostics,
                    )
                    .is_err();
                }
                if error { Err(()) } else { Ok(()) }
            }
            ModuleOrGenerateItemDeclaration::Task(id) => {
                let TaskDeclaration {
                    ident,
                    automatic: _,
                    task_ports: _,
                    block_item_decls,
                    statement_or_null,
                } = arenas.get(*id);
                let symbol = TaskSymbol {
                    ast_id: *id,
                    io: Vec::new(),
                    lowered: None,
                };
                let Ok(sid) = try_table_insert(
                    arenas,
                    table,
                    scope,
                    *ident,
                    VSymbol::Task(symbol),
                    diagnostics,
                ) else {
                    return Err(());
                };

                let mut error = false;
                for block_item_decl in block_item_decls.iter() {
                    error |= extend_block_item_decl_sid(
                        arenas,
                        sid,
                        table,
                        st,
                        block_item_decl,
                        diagnostics,
                    )
                    .is_err();
                }

                if let StatementOrNull::Statement(stmt) = arenas.get(*statement_or_null) {
                    error |= extend_statements_sids(
                        arenas,
                        AstIdRange::single(*stmt),
                        scope,
                        table,
                        st,
                        diagnostics,
                    )
                    .is_err();
                }
                st.insert_lvl_symbol(sid, InLevelSymbol::Task(*id));
                if error { Err(()) } else { Ok(()) }
            }
            ModuleOrGenerateItemDeclaration::Function(id) => {
                let FunctionDeclaration {
                    automatic: _,
                    range_or_type: _,
                    ident: _,
                    tf_input_decls: _,
                    block_item_decls,
                    statement,
                } = arenas.get(*id);

                let symbol = FunctionSymbol {
                    ast_id: *id,
                    inputs: Vec::new(),
                    output: st.dummy_signal,
                    output_ty: VType::SCALAR_NET,
                    lowered: None,
                };
                let Ok(sid) = try_table_insert(
                    arenas,
                    table,
                    scope,
                    arenas.get(*id).ident,
                    VSymbol::Function(symbol),
                    diagnostics,
                ) else {
                    return Err(());
                };

                let mut error = false;
                for block_item_decl in block_item_decls.iter() {
                    error |= extend_block_item_decl_sid(
                        arenas,
                        sid,
                        table,
                        st,
                        block_item_decl,
                        diagnostics,
                    )
                    .is_err();
                }

                error |= extend_statements_sids(
                    arenas,
                    AstIdRange::single(*statement),
                    scope,
                    table,
                    st,
                    diagnostics,
                )
                .is_err();

                st.insert_lvl_symbol(sid, InLevelSymbol::Function(*id));
                if error { Err(()) } else { Ok(()) }
            }
        },
        ModuleOrGenerateItemContent::LocalParameterDeclaration(id) => {
            let LocalParameterDeclaration {
                typing,
                assignments,
            } = arenas.get(id);
            extend_param_decl_idents_into_scope(
                arenas,
                scope,
                table,
                st,
                *typing,
                *assignments,
                ParameterType::Local,
                diagnostics,
            )
        }
        ModuleOrGenerateItemContent::ParameterOverride => todo!(),
        ModuleOrGenerateItemContent::ContinuousAssign(_)
        | ModuleOrGenerateItemContent::GateInstantiation(_)
        | ModuleOrGenerateItemContent::UdpInstantiation(_) => Ok(()),
        ModuleOrGenerateItemContent::ModuleInstantiation(id) => {
            let ModuleInstantiation {
                module_identifier,
                parameter_value_assignment,
                module_instances,
            } = arenas.get(id);

            let Some(module) = st.module_lut.get(&module_identifier.item.0) else {
                diagnostics.module_not_found(arenas, *module_identifier);
                return Err(());
            };

            let mut error = false;
            for module_instance in module_instances.iter() {
                let symbol = ModuleSymbol {
                    module: module_identifier.item.0,
                    ports: Vec::new(),
                    parameters: Vec::new(),
                    // @Performance: Remove these allocations.
                    parameter_overrides: Arc::new(VgHashMap::default()),
                    parameter_override_values: Arc::new(Vec::default()),
                    contains_specify: false,
                };
                let Ok(sid) = try_table_insert(
                    arenas,
                    table,
                    scope,
                    arenas.get(module_instance).name_of_module_instance,
                    VSymbol::Module(symbol),
                    diagnostics,
                ) else {
                    error = true;
                    continue;
                };
                st.insert_lvl_symbol(
                    sid,
                    InLevelSymbol::ModuleInstance(*parameter_value_assignment, *module),
                );
            }
            if error { Err(()) } else { Ok(()) }
        }
        ModuleOrGenerateItemContent::InitialConstruct(id) => {
            let InitialConstruct(id) = arenas.get(id);
            extend_statements_sids(
                arenas,
                AstIdRange::single(*id),
                scope,
                table,
                st,
                diagnostics,
            )
        }
        ModuleOrGenerateItemContent::AlwaysConstruct(id) => {
            let AlwaysConstruct(id) = arenas.get(id);
            extend_statements_sids(
                arenas,
                AstIdRange::single(*id),
                scope,
                table,
                st,
                diagnostics,
            )
        }
        ModuleOrGenerateItemContent::LoopGenerateConstruct(id) => {
            let lvl = ElabLevel::GenerateLoop(id);
            st.next_levels.push_back((scope, lvl));
            Ok(())
        }
        ModuleOrGenerateItemContent::IfGenerateConstruct(id) => {
            let lvl = ElabLevel::GenerateIf(id);
            st.next_levels.push_back((scope, lvl));
            Ok(())
        }
        ModuleOrGenerateItemContent::CaseGenerateConstruct(id) => {
            let lvl = ElabLevel::GenerateCase(id);
            st.next_levels.push_back((scope, lvl));
            Ok(())
        }
    }
}

fn extend_statements_sids<'a>(
    arenas: &'a AstArenas,
    stmts: AstIdRange<Statement>,
    scope: SymbolId,
    table: &mut VSymbolTable,
    st: &mut ElaborationState,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    assert!(st.stmt_dispatch_stack.is_empty());

    st.stmt_dispatch_stack.push((scope, stmts));

    let mut error = false;
    while let Some((scope, stmts)) = st.stmt_dispatch_stack.pop() {
        macro_rules! dispatch_stmt_or_null {
            ($stmt_or_null:expr) => {
                if let StatementOrNull::Statement(stmt) = arenas.get($stmt_or_null) {
                    st.stmt_dispatch_stack
                        .push((scope, AstIdRange::single(*stmt)));
                }
            };
        }

        for stmt in stmts.iter() {
            match arenas.get(stmt).content {
                StatementContent::SeqBlock(id) => {
                    let SeqBlock { block, statements } = arenas.get(id);

                    let mut scope = scope;
                    if let Some(block) = block {
                        let Block {
                            block_identifier,
                            block_item_decls,
                        } = arenas.get(*block);

                        let Ok(named_block_scope) = try_table_insert(
                            arenas,
                            table,
                            scope,
                            *block_identifier,
                            VSymbol::NamedBlock,
                            diagnostics,
                        ) else {
                            error = true;
                            continue;
                        };
                        scope = named_block_scope;

                        for block_item_decl in block_item_decls.iter() {
                            error |= extend_block_item_decl_sid(
                                arenas,
                                scope,
                                table,
                                st,
                                block_item_decl,
                                diagnostics,
                            )
                            .is_err();
                        }
                    }

                    st.stmt_dispatch_stack.push((scope, *statements));
                }

                StatementContent::CaseStatement(id) => {
                    for item in arenas.get(id).items {
                        dispatch_stmt_or_null!(arenas.get(item).statement_or_null)
                    }
                }
                StatementContent::ConditionalStatement(id) => {
                    let ConditionalStatement {
                        if_branch,
                        else_ifs,
                        else_branch,
                    } = arenas.get(id);

                    dispatch_stmt_or_null!(if_branch.statement);
                    for else_if in else_ifs.iter() {
                        dispatch_stmt_or_null!(arenas.get(else_if).statement);
                    }
                    if let Some(stmt_or_null) = else_branch {
                        dispatch_stmt_or_null!(*stmt_or_null);
                    }
                }
                StatementContent::LoopStatement(id) => {
                    st.stmt_dispatch_stack
                        .push((scope, AstIdRange::single(arenas.get(id).statement)));
                }

                StatementContent::ProceduralTimingControlStatement(id) => {
                    dispatch_stmt_or_null!(arenas.get(id).statement_or_null)
                }
                StatementContent::WaitStatement(id) => {
                    dispatch_stmt_or_null!(arenas.get(id).statement_or_null)
                }

                StatementContent::DisableStatement
                | StatementContent::EventTrigger
                | StatementContent::ParBlock
                | StatementContent::ProceduralContinuousAssignments
                | StatementContent::BlockingAssignment(_)
                | StatementContent::NonBlockingAssignment(_)
                | StatementContent::SystemTaskEnable(_)
                | StatementContent::TaskEnable(_) => {}
            }
        }
    }

    if error { Err(()) } else { Ok(()) }
}

fn extend_block_item_decl_sid<'a>(
    arenas: &'a AstArenas,
    scope: SymbolId,
    table: &mut VSymbolTable,
    st: &mut ElaborationState,
    block_item_decl: AstId<BlockItemDeclaration>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    match arenas.get(block_item_decl) {
        BlockItemDeclaration::Reg {
            signed,
            range,
            identifiers,
        } => extend_variable_type_sids(
            arenas,
            *identifiers,
            |var_type| InLevelSymbol::Reg(*signed, *range, var_type),
            scope,
            table,
            st,
            diagnostics,
        ),
        BlockItemDeclaration::Integer(var_types) => extend_variable_type_sids(
            arenas,
            *var_types,
            |var_type| InLevelSymbol::Integer(var_type),
            scope,
            table,
            st,
            diagnostics,
        ),
        BlockItemDeclaration::LocalParameterDeclaration(id) => {
            let LocalParameterDeclaration {
                typing,
                assignments,
            } = arenas.get(*id);
            extend_param_decl_idents_into_scope(
                arenas,
                scope,
                table,
                st,
                *typing,
                *assignments,
                ParameterType::Local,
                diagnostics,
            )
        }
        BlockItemDeclaration::ParameterDeclaration(id) => {
            let ParameterDeclaration {
                typing,
                assignments,
            } = arenas.get(*id);
            extend_param_decl_idents_into_scope(
                arenas,
                scope,
                table,
                st,
                *typing,
                *assignments,
                ParameterType::NonLocal,
                diagnostics,
            )
        }

        BlockItemDeclaration::Time
        | BlockItemDeclaration::Real
        | BlockItemDeclaration::Realtime
        | BlockItemDeclaration::Event => todo!(),
    }
}

fn extend_variable_type_sids<'a>(
    arenas: &'a AstArenas,
    var_types: AstIdRange<VariableType>,
    f: impl Fn(AstId<VariableType>) -> InLevelSymbol,
    scope: SymbolId,
    table: &mut VSymbolTable,
    st: &mut ElaborationState,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let mut error = false;
    for var_type in var_types.iter() {
        let symbol = NetSymbol {
            ty: VType::SCALAR_NET,
            dims: Vec::new(),
            signal: st.dummy_signal,
            nba: None,
            specify_proxy: None,
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
        st.insert_lvl_symbol(sid, f(var_type));
    }
    if error { Err(()) } else { Ok(()) }
}

impl InLevelSymbol {
    pub fn extend_needs<'a>(
        &self,
        arenas: &'a AstArenas,
        sid: SymbolId,
        table: &VSymbolTable,
        st: &mut ElaborationState,
    ) {
        let scope = table[sid]
            .parent()
            .expect("in-level symbols should always have parents");

        match self {
            InLevelSymbol::Param(typing, expr, module_sid) => {
                if let Some(module_sid) = module_sid {
                    let ident = table[sid].name();
                    if unwrap_get_module(table, *module_sid)
                        .parameter_overrides
                        .contains_key(&ident)
                    {
                        return;
                    }
                }
                match arenas.get(*typing) {
                    ParameterDeclarationTyping::None(_, Some(range)) => {
                        let Range { msb, lsb } = arenas.get(*range);
                        for e in [*msb, *lsb] {
                            extend_expr_needs(arenas, scope, table, st, e);
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
                    extend_expr_needs(arenas, scope, table, st, *e);
                }
            }
            InLevelSymbol::ModuleInstance(parameter_value_assignment, _) => {
                if let Some(parameter_value_assignment) = parameter_value_assignment {
                    match arenas.get(*parameter_value_assignment) {
                        ParameterValueAssignment::Ordered(exprs) => {
                            for e in exprs.iter() {
                                extend_expr_needs(arenas, scope, table, st, e);
                            }
                        }
                        ParameterValueAssignment::Named(named_exprs) => {
                            for named_expr in named_exprs.iter() {
                                let Some(expr) = arenas.get(named_expr).expression else {
                                    continue;
                                };
                                let exprs: &[_] = match arenas.get(expr) {
                                    ConstantMinTypMaxExpression::Single(e) => &[*e],
                                    ConstantMinTypMaxExpression::MinTypMax { min, typ, max } => {
                                        &[*min, *typ, *max]
                                    }
                                };
                                for e in exprs {
                                    extend_expr_needs(arenas, scope, table, st, *e);
                                }
                            }
                        }
                    }
                }
            }
            InLevelSymbol::Net(id, dimension, _) => {
                let NetDeclaration {
                    net_type: _,
                    signed: _,
                    range,
                    nets: _,
                } = arenas.get(*id);
                if let Some(range) = range {
                    let Range { msb, lsb } = arenas.get(*range);
                    for e in [*msb, *lsb] {
                        extend_expr_needs(arenas, scope, table, st, e);
                    }
                }
                if let Some(dims) = dimension {
                    for dim in dims.iter() {
                        let Dimension { lhs, rhs } = arenas.get(dim);
                        for e in [*lhs, *rhs] {
                            extend_expr_needs(arenas, scope, table, st, e);
                        }
                    }
                }
            }
            InLevelSymbol::Reg(_, range, var_type) => {
                if let Some(range) = range {
                    let Range { msb, lsb } = arenas.get(*range);
                    for e in [*msb, *lsb] {
                        extend_expr_needs(arenas, scope, table, st, e);
                    }
                }

                extend_var_type_needs(arenas, scope, table, st, *var_type)
            }
            InLevelSymbol::Integer(var_type) => {
                extend_var_type_needs(arenas, scope, table, st, *var_type)
            }
            InLevelSymbol::Port(decl, _) => {
                let range = match arenas.get(*decl) {
                    PortDeclaration::Inout(id) => arenas.get(*id).range,
                    PortDeclaration::Input(id) => arenas.get(*id).range,
                    PortDeclaration::Output(id) => arenas.get(*id).range,
                };

                if let Some(range) = range {
                    let Range { msb, lsb } = arenas.get(range);
                    for e in [*msb, *lsb] {
                        extend_expr_needs(arenas, scope, table, st, e);
                    }
                }
            }
            InLevelSymbol::Task(id) => {
                let TaskDeclaration {
                    ident: _,
                    automatic: _,
                    task_ports,
                    block_item_decls: _,
                    statement_or_null: _,
                } = arenas.get(*id);

                for task_port in task_ports.iter() {
                    let TaskPortItem {
                        attribute_instances: _,
                        content,
                    } = arenas.get(task_port);
                    let tf_type = match content {
                        TaskPortItemContent::Input(decl) => &decl.tf_type,
                        TaskPortItemContent::Output(decl) => &decl.tf_type,
                        TaskPortItemContent::Inout(decl) => &decl.tf_type,
                    };

                    extend_tf_type_needs(arenas, scope, table, st, tf_type);
                }
            }
            InLevelSymbol::Function(id) => {
                let FunctionDeclaration {
                    automatic: _,
                    range_or_type,
                    ident: _,
                    tf_input_decls,
                    block_item_decls: _,
                    statement: _,
                } = arenas.get(*id);

                if let FunctionRangeOrType::Signed(Some(range))
                | FunctionRangeOrType::Unsigned(Some(range)) = arenas.get(*range_or_type)
                {
                    let Range { msb, lsb } = arenas.get(*range);
                    for e in [*msb, *lsb] {
                        extend_expr_needs(arenas, scope, table, st, e);
                    }
                }
                for tf_input_decl in tf_input_decls.iter() {
                    extend_tf_type_needs(
                        arenas,
                        sid,
                        table,
                        st,
                        &arenas.get(tf_input_decl).tf_type,
                    );
                }
            }
        }
    }
}

fn extend_tf_type_needs<'a>(
    arenas: &'a AstArenas,
    scope: SymbolId,
    table: &VSymbolTable,
    st: &mut ElaborationState,
    tf_type: &TfType,
) {
    if let TfType::Net {
        reg: _,
        signed: _,
        range: Some(range),
    } = tf_type
    {
        let Range { msb, lsb } = arenas.get(*range);
        for e in [*msb, *lsb] {
            extend_expr_needs(arenas, scope, table, st, e);
        }
    }
}

pub fn extend_var_type_needs<'a>(
    arenas: &'a AstArenas,
    scope: SymbolId,
    table: &VSymbolTable,
    st: &mut ElaborationState,
    var_type: AstId<VariableType>,
) {
    match arenas.get(var_type).variant {
        VariableTypeVariant::Dimensions(dims) => {
            for dim in dims.iter() {
                let Dimension { lhs, rhs } = arenas.get(dim);
                for e in [*lhs, *rhs] {
                    extend_expr_needs(arenas, scope, table, st, e);
                }
            }
        }
        VariableTypeVariant::ConstantExpr(e) => {
            extend_expr_needs(arenas, scope, table, st, e);
        }
    }
}

pub fn extend_expr_needs<'a>(
    arenas: &'a AstArenas,
    scope: SymbolId,
    table: &VSymbolTable,
    st: &mut ElaborationState,
    expr: AstId<ConstantExpr>,
) {
    let expr = expr.into_expr();
    assert!(st.dispatch_stack.is_empty());

    let dispatch_stack = &mut st.dispatch_stack;
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
                    && st.marked.insert(ident_sid)
                    && st.lvl_symbols.contains_key(&ident_sid)
                {
                    st.needs_adjacency_list_items.push(ident_sid);
                };
            }
            Expr::FunctionCall(ident, exprs) => {
                dispatch_stack.extend(exprs.iter());

                if let Some(ident_sid) = resolve_symbol_id_hier(scope, table, arenas, *ident)
                    && st.marked.insert(ident_sid)
                    && st.lvl_symbols.contains_key(&ident_sid)
                {
                    st.needs_adjacency_list_items.push(ident_sid);
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
    next_levels: &mut VecDeque<(SymbolId, ElabLevel)>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    match symbol {
        InLevelSymbol::Param(typing, constant_expr, module_sid) => {
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
            let ident = table[sid].name();
            let value = if let Some(module_sid) = module_sid
                && let module = unwrap_get_module(table, *module_sid)
                && let Some(param_override_idx) = module.parameter_overrides.get(&ident)
            {
                module.parameter_override_values[*param_override_idx].clone()
            } else {
                match arenas.get(constant_expr) {
                    ConstantMinTypMaxExpression::Single(id) => {
                        super::eval_constant_expr_elab(gl, arenas, scope, table, diagnostics, *id)?
                    }
                    ConstantMinTypMaxExpression::MinTypMax { .. } => todo!(),
                }
            };

            let width = ty.map_or_else(|| value.ty().force_net_width(), |ty| ty.force_net_width());
            let value = value.truncate_or_extend(width);

            *unwrap_get_param_mut(table, sid) = value;
        }
        InLevelSymbol::Reg(signed, range, var_type) => {
            let VariableType {
                identifier,
                variant,
            } = arenas.get(*var_type);
            let ty = match range {
                None => VType::net(SCALAR_VSIZE, *signed),
                Some(range) => {
                    let (_, _, width) =
                        super::eval_constant_range(gl, arenas, scope, table, diagnostics, *range)?;
                    VType::net(width, *signed)
                }
            };
            let parent = table[sid].parent().unwrap();
            let dims = match variant {
                VariableTypeVariant::Dimensions(dimensions) => {
                    super::dims_to_array_elab(gl, arenas, parent, table, diagnostics, *dimensions)?
                }
                VariableTypeVariant::ConstantExpr(_) => Vec::new(),
            };
            let net = unwrap_get_net_mut(table, sid);
            net.signal = super::new_signal(gl, arenas, &ty, &dims, *identifier);
            net.dims = dims;
            net.ty = ty;
        }
        InLevelSymbol::Net(id, dims, ident) => {
            let NetDeclaration {
                signed,
                range,
                net_type: _,
                nets: _,
            } = arenas.get(*id);

            let ty = match range {
                None => VType::net(SCALAR_VSIZE, *signed),
                Some(range) => {
                    let (_, _, width) =
                        super::eval_constant_range(gl, arenas, scope, table, diagnostics, *range)?;
                    VType::net(width, *signed)
                }
            };
            let parent = table[sid].parent().unwrap();
            let dims = match dims {
                None => Vec::new(),
                Some(dims) => {
                    super::dims_to_array_elab(gl, arenas, parent, table, diagnostics, *dims)?
                }
            };
            let net = unwrap_get_net_mut(table, sid);
            net.signal = super::new_signal(gl, arenas, &ty, &dims, *ident);
            net.dims = dims;
            net.ty = ty;
        }
        InLevelSymbol::Integer(id) => {
            let VariableType {
                identifier,
                variant,
            } = arenas.get(*id);
            let parent = table[sid].parent().unwrap();
            let dims = match variant {
                VariableTypeVariant::Dimensions(dimensions) => {
                    super::dims_to_array_elab(gl, arenas, parent, table, diagnostics, *dimensions)?
                }
                VariableTypeVariant::ConstantExpr(_) => Vec::new(),
            };
            let net = unwrap_get_net_mut(table, sid);
            net.signal = super::new_signal(
                gl,
                arenas,
                &VType::SignedNet(INTEGER_VSIZE),
                &dims,
                *identifier,
            );
            net.dims = dims;
            net.ty = VType::SignedNet(INTEGER_VSIZE);
        }
        InLevelSymbol::Port(id, ident) => {
            let id = *id;
            let (ty, _, _) = port_declaration_to_info(gl, arenas, id, scope, table, diagnostics)?;
            let net = unwrap_get_net_mut(table, sid);
            net.signal = super::new_signal(gl, arenas, &ty, &[], *ident);
            net.ty = ty;
        }
        InLevelSymbol::Task(_) => {
            super::function::elaborate_task(gl, arenas, sid, table, diagnostics)?;
        }
        InLevelSymbol::Function(id) => {
            super::function::elaborate_fn(gl, arenas, sid, table, diagnostics)?;
            // @TODO: This should ignore errors with unresolved symbols.
            _ = crate::lower::module_or_generate_item::function::lower(
                gl,
                arenas,
                diagnostics,
                &mut crate::lower::Scope {
                    table: table,
                    key: sid,
                    udps: &VgHashMap::default(),
                    signal_map: &mut std::collections::HashMap::new(),
                },
                *id,
            )?;
        }
        InLevelSymbol::ModuleInstance(parameter_value_assignment, module) => {
            let (parameter_overrides, parameter_override_values) = match *parameter_value_assignment
            {
                None => Default::default(),
                Some(id) => match arenas.get(id) {
                    ParameterValueAssignment::Ordered(ids) => {
                        let mut params = Vec::new();
                        for id in ids.iter() {
                            let value = super::eval_constant_expr_elab(
                                gl,
                                arenas,
                                scope,
                                &table,
                                diagnostics,
                                id,
                            )?;
                            params.push(value);
                        }
                        (Default::default(), params)
                    }
                    ParameterValueAssignment::Named(named) => {
                        let mut params = VgHashMap::default();
                        let mut param_values = Vec::new();
                        for n in named.iter() {
                            let NamedParameterAssignment {
                                identifier,
                                expression,
                            } = arenas.get(n);
                            let Some(expression) = expression else {
                                diagnostics.not_yet_implemented(
                                    arenas.get_span(n),
                                    "null parameter assignment",
                                );
                                return Err(());
                            };
                            let ConstantMinTypMaxExpression::Single(expression) =
                                arenas.get(*expression)
                            else {
                                diagnostics.not_yet_implemented(
                                    arenas.get_span(n),
                                    "mintypmax parameter assignment",
                                );
                                return Err(());
                            };
                            let value = super::eval_constant_expr_elab(
                                gl,
                                arenas,
                                scope,
                                &table,
                                diagnostics,
                                *expression,
                            )?;
                            params.insert(identifier.item.0, param_values.len());
                            param_values.push(value);
                        }
                        (params, param_values)
                    }
                },
            };

            let module_symbol = unwrap_get_module_mut(table, sid);
            module_symbol.parameter_overrides = Arc::new(parameter_overrides);
            module_symbol.parameter_override_values = Arc::new(parameter_override_values);

            next_levels.push_back((sid, ElabLevel::Module(*module)));
        }
    }

    Ok(())
}

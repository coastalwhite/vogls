use std::collections::VecDeque;
use std::sync::Arc;

use vogls_frontend::ident_table::{IdentId, IdentTable};
use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::token_range::TokenRange;
use vogls_ir::{
    ConnectionDirection, GlobalContext, INTEGER_VSIZE, SCALAR_VSIZE, SignalFlags, SignalKey,
};
use vogls_utils::{IndexMap, VgHashMap, VgHashSet};

use crate::ast::constant_expr::{ConstantExpr, ConstantMinTypMaxExpression};
use crate::ast::expr::{Expr, Replication};
use crate::ast::module::{
    AlwaysConstruct, BlockItemDeclaration, CaseGenerateConstruct, CaseGenerateItem,
    CaseGeneratePattern, Dimension, FunctionDeclaration, FunctionRangeOrType, GenerateBlock,
    GenvarAssignment, GenvarDeclaration, IfGenerateConstruct, InitialConstruct, IntegerDeclaration,
    LocalParameterDeclaration, LoopGenerateConstruct, Module, ModuleInstantiation, ModuleItem,
    ModuleOrGenerateItem, ModuleOrGenerateItemContent, ModuleOrGenerateItemDeclaration,
    ModulePorts, NamedParameterAssignment, NetDeclAssignment, NetDeclaration, NetDeclarationNets,
    NetIdent, NetType, NonPortModuleItem, ParamAssignment, ParameterDeclaration,
    ParameterDeclarationTyping, ParameterValueAssignment, Port, PortDeclaration, PortExpression,
    PortReference, Range, RegDeclaration, TaskDeclaration, TaskPortItem, TaskPortItemContent,
    TfType, TimeScale, VariableType, VariableTypeVariant,
};
use crate::ast::statement::{
    Block, BlockingAssignment, CaseStatement, ConditionalStatement, LoopStatement,
    LoopStatementVariant, ParBlock, SeqBlock, Statement, StatementContent, StatementOrNull,
    WaitStatement,
};
use crate::ast::{AstId, AstIdRange, AstItem, Identifier};
use crate::elaborate::{FunctionSymbol, TaskSymbol};
use crate::lower::{
    Diagnostics, LowerContext, MutLowerContext, VType, VValue, eval_constant_expr, resolve_hident,
    try_resolve_hident, unwrap_get_module, unwrap_get_module_mut, unwrap_get_net_mut,
    unwrap_get_param_mut,
};

use super::{
    ModuleSymbol, Net, NetSymbol, VSymbol, VSymbolTable, VectorTransform, evaluate_net_msb_lsb,
    port_declaration_to_info, try_table_insert,
};

pub enum ElabLevel<'a> {
    GenerateIf(AstId<'a, IfGenerateConstruct<'a>>),
    GenerateLoop(AstId<'a, LoopGenerateConstruct<'a>>),
    GenerateCase(AstId<'a, CaseGenerateConstruct<'a>>),
    GenerateBlock(AstIdRange<'a, ModuleOrGenerateItem<'a>>),
    Module(AstId<'a, Module<'a>>),
    ModuleRange(AstId<'a, Module<'a>>, usize),
}

#[derive(Clone, Copy)]
pub enum InLevelSymbol<'a> {
    Param(
        AstId<'a, ParameterDeclarationTyping<'a>>,
        AstId<'a, ConstantMinTypMaxExpression<'a>>,
        Option<SymbolId>,
    ),
    ModuleInstance(
        Option<AstId<'a, ParameterValueAssignment<'a>>>,
        Option<AstId<'a, Range<'a>>>,
        AstId<'a, Module<'a>>,
    ),
    Integer(AstId<'a, VariableType<'a>>),
    Reg(RegInLevelSymbol<'a>),
    Net(NetInLevelSymbol<'a>),
    Port(PortInLevelSymbol<'a>),
    Task(AstId<'a, TaskDeclaration<'a>>),
    Function(AstId<'a, FunctionDeclaration<'a>>),
}

#[derive(Clone, Copy)]
pub struct RegInLevelSymbol<'a> {
    signed: bool,
    range: Option<AstId<'a, Range<'a>>>,
    variable_type: AstId<'a, VariableType<'a>>,
}
#[derive(Clone, Copy)]
pub struct NetInLevelSymbol<'a> {
    decl: AstId<'a, NetDeclaration<'a>>,
    dim: Option<AstIdRange<'a, Dimension<'a>>>,
    ident: AstItem<Identifier>,
}
#[derive(Clone, Copy)]
pub struct PortInLevelSymbol<'a> {
    decl: AstId<'a, PortDeclaration<'a>>,
    ident: AstItem<Identifier>,
}

pub struct ElaborationState<'a, 'b> {
    lvl_symbols: IndexMap<SymbolId, InLevelSymbol<'a>>,
    next_levels: VecDeque<(SymbolId, ElabLevel<'a>, TimeScale)>,
    marked: VgHashSet<SymbolId>,

    needs_adjacency_list: Vec<(SymbolId, usize, usize)>,
    needs_adjacency_list_items: Vec<SymbolId>,

    dummy_signal: SignalKey,

    /// Scratchpad for traversing the constant expressions.
    dispatch_stack: Vec<AstId<'a, Expr<'a>>>,
    stmt_dispatch_stack: Vec<(SymbolId, AstIdRange<'a, Statement<'a>>)>,

    module_lut: &'b VgHashMap<IdentId, AstId<'a, Module<'a>>>,
}

impl<'a, 'b> ElaborationState<'a, 'b> {
    pub fn insert_lvl_symbol(&mut self, sid: SymbolId, symbol: InLevelSymbol<'a>) {
        assert!(self.lvl_symbols.insert(sid, symbol).is_ok());
    }

    fn dummy_net(&self) -> Net {
        Net {
            width: SCALAR_VSIZE,
            specify: None,
            ba: self.dummy_signal,
            nba: None,
        }
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
    ctx: &mut LowerContext<'a, '_>,
    top_level: AstId<'a, Module<'a>>,
    module_lut: &VgHashMap<IdentId, AstId<'a, Module<'a>>>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let dummy_signal = gl.signals.insert(vogls_ir::Signal {
        name: "".to_string(),
        size: SCALAR_VSIZE,
        initialize: None,
        flags: SignalFlags::EMPTY,
        mode: vogls_ir::LogicMode::TwoValue,
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
        module_lut,
    };

    assert!(ctx.table.is_empty());
    let tlm_ident = top_level.module_identifier;
    let tlm = tlm_ident.item.0;
    let module_symbol = ModuleSymbol {
        module: tlm,
        ports: Vec::new(),
        parameters: Vec::new(),
        parameter_overrides: Arc::new(VgHashMap::default()),
        parameter_override_values: Arc::new(Vec::new()),
        contains_specify: false,
        time_scale: top_level.time_scale,
    };
    let tlm_sid = ctx
        .table
        .insert_root(
            tlm,
            ctx.arenas.get_item_span(tlm_ident),
            VSymbol::Module(module_symbol),
        )
        .expect("No collisions possible, first symbol.");

    st.next_levels
        .push_back((tlm_sid, ElabLevel::Module(top_level), top_level.time_scale));

    let mut error = false;
    while let Some((scope, lvl, time_scale)) = st.next_levels.pop_front() {
        ctx.time_scale = time_scale;

        st.lvl_symbols.clear();
        st.needs_adjacency_list_items.clear();

        let mut lvl_error = false;
        match lvl {
            ElabLevel::GenerateIf(id) => {
                lvl_error |=
                    extend_generate_if_sids(gl, scope, ctx, &mut st, id, diagnostics).is_err()
            }
            ElabLevel::GenerateLoop(id) => {
                lvl_error |=
                    extend_generate_loop_sids(gl, scope, ctx, &mut st, id, diagnostics).is_err()
            }
            ElabLevel::GenerateCase(id) => {
                lvl_error |=
                    extend_generate_case_sids(gl, scope, ctx, &mut st, id, diagnostics).is_err()
            }
            ElabLevel::GenerateBlock(mod_or_gen_items) => {
                for item in mod_or_gen_items.iter() {
                    lvl_error |=
                        extend_module_or_generate_item_sids(item, scope, ctx, &mut st, diagnostics)
                            .is_err();
                }
            }
            ElabLevel::Module(module) => {
                lvl_error |= elaborate_module(module, scope, ctx, &mut st, diagnostics).is_err();
            }
            ElabLevel::ModuleRange(module, num_instances) => {
                lvl_error |=
                    elaborate_module_range(module, scope, num_instances, ctx, &mut st, diagnostics)
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
                symbol.extend_needs(sid, &ctx.table, &mut st);
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
                        lvl_symbol,
                        *sid,
                        scope,
                        ctx,
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
                            ctx.table[*s].origin(),
                            "Symbol is involved in elaboration loop.",
                        );
                    }
                    return Err(());
                }
            }
        }
    }

    if error { Err(()) } else { Ok(()) }
}

fn extend_opt_generate_block_sids<'a, 'b>(
    scope: SymbolId,
    ctx: &mut LowerContext<'a, '_>,
    st: &mut ElaborationState<'a, 'b>,
    id: AstId<'a, Option<GenerateBlock<'a>>>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let Some(blk) = &*id else {
        return Ok(());
    };

    let items = blk.module_or_generate_items();
    let offset = ctx.table_ast_refs.gen_blocks.insert(items);
    let symbol = VSymbol::GenerateBlock(offset);
    let sid = match blk.ident() {
        None => ctx.table.insert_unlinked(
            IdentTable::EMPTY_IDENT,
            scope,
            ctx.arenas.get_span(id),
            symbol,
        ),
        Some(ident) => try_table_insert(
            ctx.arenas,
            &mut ctx.table,
            scope,
            ident,
            symbol,
            diagnostics,
        )?,
    };

    let mut error = false;
    for item in items.iter() {
        error |= extend_module_or_generate_item_sids(item, sid, ctx, st, diagnostics).is_err();
    }
    if error { Err(()) } else { Ok(()) }
}

fn extend_generate_if_sids<'a, 'b>(
    gl: &mut GlobalContext,
    scope: SymbolId,
    ctx: &mut LowerContext<'a, '_>,
    st: &mut ElaborationState<'a, 'b>,
    id: AstId<'a, IfGenerateConstruct<'a>>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let condition = eval_constant_expr(
        gl,
        ctx.arenas,
        &ctx.table,
        scope,
        diagnostics,
        id.condition,
        None,
    )?;

    let blk = if condition.to_logical() {
        id.truthy
    } else {
        match &id.falsy {
            None => return Ok(()),
            Some(blk) => *blk,
        }
    };
    extend_conditional_generate_sids(gl, scope, ctx, st, blk, diagnostics)
}

fn extend_conditional_generate_sids<'a, 'b>(
    gl: &mut GlobalContext,
    scope: SymbolId,
    ctx: &mut LowerContext<'a, '_>,
    st: &mut ElaborationState<'a, 'b>,
    blk: AstId<'a, Option<GenerateBlock<'a>>>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let Some(id) = &*blk else {
        return Ok(());
    };

    // @TODO: Remove recursive nature here.
    match id {
        GenerateBlock::ModuleOrGenerateItem(id) => match &id.content {
            ModuleOrGenerateItemContent::IfGenerateConstruct(id) => {
                return extend_generate_if_sids(gl, scope, ctx, st, *id, diagnostics);
            }
            ModuleOrGenerateItemContent::CaseGenerateConstruct(id) => {
                return extend_generate_case_sids(gl, scope, ctx, st, *id, diagnostics);
            }
            _ => {}
        },
        GenerateBlock::BeginEnd(..) => {}
    }

    extend_opt_generate_block_sids(scope, ctx, st, blk, diagnostics)
}

fn extend_generate_loop_sids<'a, 'b>(
    gl: &mut GlobalContext,
    scope: SymbolId,
    ctx: &mut LowerContext<'a, '_>,
    st: &mut ElaborationState<'a, 'b>,
    id: AstId<'a, LoopGenerateConstruct<'a>>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let LoopGenerateConstruct {
        initialization,
        condition,
        iteration,
        block,
    } = &*id;

    let GenvarAssignment {
        ident: initialization_ident,
        expr: initialization,
    } = &**initialization;
    let GenvarAssignment {
        ident: iteration_ident,
        expr: iteration,
    } = &**iteration;

    if initialization_ident.item.0 != iteration_ident.item.0 {
        diagnostics.not_yet_implemented(
            ctx.arenas.get_span(*initialization),
            "initialization and iteration assignment identifier are different",
        );
        return Err(());
    }
    let genvar_sid = try_resolve_hident(
        scope,
        &ctx.table,
        ctx.arenas,
        *initialization_ident,
        diagnostics,
    )?;
    let VSymbol::GenVar = &ctx.table[genvar_sid].content else {
        diagnostics.not_yet_implemented(
            ctx.arenas.get_span(*initialization),
            "non-genvar used as genvar",
        );
        return Err(());
    };

    let mut value = eval_constant_expr(
        gl,
        ctx.arenas,
        &ctx.table,
        scope,
        diagnostics,
        *initialization,
        Some(INTEGER_VSIZE),
    )?;

    let (mod_or_gen_items, block_ident_ast) = match &**block {
        GenerateBlock::ModuleOrGenerateItem(id) => (AstIdRange::single(*id), None),
        GenerateBlock::BeginEnd(ident, mod_or_gen_items) => (*mod_or_gen_items, *ident),
    };

    let loop_sid = match block_ident_ast {
        Some(block_ident) => try_table_insert(
            ctx.arenas,
            &mut ctx.table,
            scope,
            block_ident,
            VSymbol::GenerateBlocks,
            diagnostics,
        )?,
        None => ctx.table.insert_unlinked(
            IdentTable::EMPTY_IDENT,
            scope,
            ctx.arenas.get_range_span(mod_or_gen_items),
            VSymbol::GenerateBlocks,
        ),
    };

    let gen_block_ast_offset = ctx.table_ast_refs.gen_blocks.insert(mod_or_gen_items);
    loop {
        let iter_sid = ctx.table.insert_unlinked(
            IdentTable::EMPTY_IDENT,
            loop_sid,
            ctx.table[loop_sid].origin(),
            VSymbol::GenerateBlock(gen_block_ast_offset),
        );

        let genvar_constant = ctx
            .table
            .insert(
                initialization_ident.item.0,
                iter_sid,
                ctx.arenas.get_item_span(*initialization_ident),
                VSymbol::Parameter(value.clone()),
            )
            .expect("No other idents in this block yet");

        let c = eval_constant_expr(
            gl,
            ctx.arenas,
            &ctx.table,
            iter_sid,
            diagnostics,
            *condition,
            None,
        )?;
        if !c.to_logical() {
            ctx.table.pop_last_inserted(genvar_constant);
            ctx.table.pop_last_inserted(iter_sid);
            break;
        }

        st.next_levels.push_back((
            iter_sid,
            ElabLevel::GenerateBlock(mod_or_gen_items),
            ctx.time_scale,
        ));

        // @CONTEXTWIDTH
        value = eval_constant_expr(
            gl,
            ctx.arenas,
            &ctx.table,
            iter_sid,
            diagnostics,
            *iteration,
            None,
        )?;
    }
    Ok(())
}

fn extend_generate_case_sids<'a, 'b>(
    gl: &mut GlobalContext,

    scope: SymbolId,
    ctx: &mut LowerContext<'a, '_>,
    st: &mut ElaborationState<'a, 'b>,
    id: AstId<'a, CaseGenerateConstruct<'a>>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let CaseGenerateConstruct { value, items } = &*id;
    let value = eval_constant_expr(gl, ctx.arenas, &ctx.table, scope, diagnostics, *value, None)?;

    for item in items.iter() {
        let CaseGenerateItem { pattern, block } = &*item;
        let mut is_selected = false;
        match pattern {
            CaseGeneratePattern::Default => is_selected = true,
            CaseGeneratePattern::Exprs(exprs) => {
                for expr in exprs.iter() {
                    let expr_value = eval_constant_expr(
                        gl,
                        ctx.arenas,
                        &ctx.table,
                        scope,
                        diagnostics,
                        expr,
                        Some(value.ty().force_net_width()),
                    )?;
                    let expr_value = expr_value.truncate_or_extend(value.ty().force_net_width());
                    if value.clone().case_equal(expr_value) {
                        is_selected = true;
                    }
                }
            }
        };

        if is_selected {
            return extend_opt_generate_block_sids(scope, ctx, st, *block, diagnostics);
        }
    }

    Ok(())
}

fn extend_param_decl_idents_into_scope<'a, 'b>(
    scope: SymbolId,
    ctx: &mut LowerContext<'a, '_>,
    st: &mut ElaborationState<'a, 'b>,
    typing: AstId<'a, ParameterDeclarationTyping<'a>>,
    assignments: AstIdRange<'a, ParamAssignment<'a>>,
    parameter_type: ParameterType,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let module_sid = match parameter_type {
        ParameterType::Local => None,
        ParameterType::NonLocal => {
            let mut module_sid = scope;
            while !matches!(ctx.table[module_sid].content, VSymbol::Module(_)) {
                module_sid = ctx.table[module_sid].parent().unwrap();
            }
            Some(module_sid)
        }
    };
    let mut error = false;
    for assignment in assignments.iter() {
        let ParamAssignment { param, constant } = &*assignment;
        let Ok(sid) = try_table_insert(
            ctx.arenas,
            &mut ctx.table,
            scope,
            *param,
            super::VSymbol::Parameter(VValue::scalar_from_bool(false)),
            diagnostics,
        ) else {
            error = true;
            continue;
        };
        if let Some(module_sid) = module_sid {
            unwrap_get_module_mut(&mut ctx.table, module_sid)
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

fn elaborate_module_range<'a, 'b>(
    module: AstId<'a, Module<'a>>,
    scope: SymbolId,
    num_instances: usize,
    ctx: &mut LowerContext<'a, '_>,
    st: &mut ElaborationState<'a, 'b>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let module_range_symbol = &ctx.table[scope];
    let name = module_range_symbol.name();
    let origin = module_range_symbol.origin();
    let VSymbol::ModuleRange(s) = &module_range_symbol.content else {
        unreachable!();
    };

    // This is really inefficient as each element of a module range gets re-elaborated, but I don't
    // really know how to deal with nested module ranges otherwise.
    let symbol = s.clone();
    for _ in 0..num_instances {
        let module_sid =
            ctx.table
                .insert_unlinked(name, scope, origin, VSymbol::Module(symbol.clone()));
        elaborate_module(module, module_sid, ctx, st, diagnostics)?;
    }

    Ok(())
}

fn elaborate_module<'a, 'b>(
    module: AstId<'a, Module<'a>>,
    scope: SymbolId,
    ctx: &mut LowerContext<'a, '_>,
    st: &mut ElaborationState<'a, 'b>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let Module {
        attribute_instances: _,
        module_identifier: _,
        module_parameter_port_list,
        ports,
        module_items,
        default_nettype: _,
        time_scale: _,
    } = &*module;

    // 1. Assign a SymbolId to each symbol.
    let mut error = false;
    if let Some(module_parameter_port_list) = module_parameter_port_list {
        for id in module_parameter_port_list.iter() {
            let ParameterDeclaration {
                typing,
                assignments,
            } = &*id;
            error |= extend_param_decl_idents_into_scope(
                scope,
                ctx,
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
                match &*id {
                    Port::PortExpression(id) => {
                        let PortExpression { references } = &**id;
                        let PortReference { identifier, .. } = &**references;
                        let symbol = NetSymbol {
                            ty: VType::SCALAR_NET,
                            dims: Vec::new(),
                            net: st.dummy_net(),
                            transform: VectorTransform::default(),
                            port_idx: Some(port_idx),
                        };
                        let symbol = VSymbol::Net(symbol);
                        let Ok(sid) = try_table_insert(
                            ctx.arenas,
                            &mut ctx.table,
                            scope,
                            *identifier,
                            symbol,
                            diagnostics,
                        ) else {
                            error = true;
                            continue;
                        };

                        unwrap_get_module_mut(&mut ctx.table, scope)
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
                let (direction, identifiers) = match &*id {
                    PortDeclaration::Inout(id) => (D::Both, id.port_identifiers),
                    PortDeclaration::Input(id) => (D::In, id.port_identifiers),
                    PortDeclaration::Output(id) => (D::Out, id.identifiers),
                };

                for ident in identifiers.iter() {
                    let ident = ctx.arenas.to_item(ident);
                    let symbol = NetSymbol {
                        ty: VType::SCALAR_NET,
                        dims: Vec::new(),
                        net: st.dummy_net(),
                        transform: VectorTransform::default(),
                        port_idx: Some(port_idx),
                    };
                    let symbol = VSymbol::Net(symbol);
                    let Ok(sid) = try_table_insert(
                        ctx.arenas,
                        &mut ctx.table,
                        scope,
                        ident,
                        symbol,
                        diagnostics,
                    ) else {
                        error = true;
                        continue;
                    };

                    st.insert_lvl_symbol(
                        sid,
                        InLevelSymbol::Port(PortInLevelSymbol { decl: id, ident }),
                    );
                    unwrap_get_module_mut(&mut ctx.table, scope)
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
        match &*item {
            ModuleItem::PortDeclaration(id) => {
                let id = *id;

                use ConnectionDirection as D;
                let (direction, identifiers) = match &*id {
                    PortDeclaration::Inout(id) => (D::Both, id.port_identifiers),
                    PortDeclaration::Input(id) => (D::In, id.port_identifiers),
                    PortDeclaration::Output(id) => (D::Out, id.identifiers),
                };

                for ident in identifiers.iter() {
                    let Some(sid) = ctx.table.resolve(scope, ident.0) else {
                        diagnostics.var_not_found(ctx.arenas, ctx.arenas.to_item(ident));
                        error = true;
                        continue;
                    };
                    let VSymbol::Net(net) = &mut ctx.table[sid].content else {
                        diagnostics.not_yet_implemented(
                            ctx.arenas.get_span(ident),
                            "non-port used as port",
                        );
                        error = true;
                        continue;
                    };
                    let Some(port_idx) = net.port_idx else {
                        diagnostics.not_yet_implemented(
                            ctx.arenas.get_span(ident),
                            "non-port used as port",
                        );
                        error = true;
                        continue;
                    };

                    st.insert_lvl_symbol(
                        sid,
                        InLevelSymbol::Port(PortInLevelSymbol {
                            decl: id,
                            ident: ctx.arenas.to_item(ident),
                        }),
                    );
                    unwrap_get_module_mut(&mut ctx.table, scope).ports[port_idx].1 = direction;
                }
            }
            ModuleItem::NonPortModuleItem(id) => match &**id {
                NonPortModuleItem::ModuleOrGenerateItem(id) => {
                    error |= extend_module_or_generate_item_sids(*id, scope, ctx, st, diagnostics)
                        .is_err();
                }
                NonPortModuleItem::GenerateRegion(region) => {
                    for item in region.module_or_generate_item.iter() {
                        error |=
                            extend_module_or_generate_item_sids(item, scope, ctx, st, diagnostics)
                                .is_err();
                    }
                }
                NonPortModuleItem::ParameterDeclaration(id) => {
                    let ParameterDeclaration {
                        typing,
                        assignments,
                    } = &**id;
                    error |= extend_param_decl_idents_into_scope(
                        scope,
                        ctx,
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
                    unwrap_get_module_mut(&mut ctx.table, scope).contains_specify = true;
                }
            },
        }
    }

    if error { Err(()) } else { Ok(()) }
}

fn extend_module_or_generate_item_sids<'a, 'b>(
    id: AstId<'a, ModuleOrGenerateItem<'a>>,
    scope: SymbolId,
    ctx: &mut LowerContext<'a, '_>,
    st: &mut ElaborationState<'a, 'b>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    match id.content {
        ModuleOrGenerateItemContent::ModuleOrGenerateItemDeclaration(id) => match &*id {
            ModuleOrGenerateItemDeclaration::Net(id) => {
                let NetDeclaration {
                    net_type,
                    signed: _,
                    range: _,
                    nets,
                } = &**id;

                if !matches!(net_type.item, NetType::Wire) {
                    diagnostics.not_yet_implemented(
                        ctx.arenas.get_item_span(*net_type),
                        "this kind of net is not yet implemented",
                    );
                    return Err(());
                }

                let mut error = false;
                match nets {
                    NetDeclarationNets::Idents(idents) => {
                        for net_ident in idents.iter() {
                            let NetIdent { ident, dimension } = &*net_ident;

                            if let Some(sid) = ctx.table.resolve(scope, ident.item.0) {
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
                                net: st.dummy_net(),
                                transform: VectorTransform::default(),
                                port_idx: None,
                            };
                            let symbol = VSymbol::Net(symbol);

                            let Ok(sid) = try_table_insert(
                                ctx.arenas,
                                &mut ctx.table,
                                scope,
                                *ident,
                                symbol,
                                diagnostics,
                            ) else {
                                error = true;
                                continue;
                            };
                            st.insert_lvl_symbol(
                                sid,
                                InLevelSymbol::Net(NetInLevelSymbol {
                                    decl: *id,
                                    dim: Some(*dimension),
                                    ident: *ident,
                                }),
                            );
                        }
                    }
                    NetDeclarationNets::Assignments(assignments) => {
                        for assignment in assignments.iter() {
                            let NetDeclAssignment { ident, expr: _ } = &*assignment;

                            if let Some(sid) = ctx.table.resolve(scope, ident.item.0) {
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
                                net: st.dummy_net(),
                                transform: VectorTransform::default(),
                                port_idx: None,
                            };
                            let symbol = VSymbol::Net(symbol);

                            let Ok(sid) = try_table_insert(
                                ctx.arenas,
                                &mut ctx.table,
                                scope,
                                *ident,
                                symbol,
                                diagnostics,
                            ) else {
                                error = true;
                                continue;
                            };
                            st.insert_lvl_symbol(
                                sid,
                                InLevelSymbol::Net(NetInLevelSymbol {
                                    decl: *id,
                                    dim: None,
                                    ident: *ident,
                                }),
                            );
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
                } = &**id;
                extend_variable_type_sids(
                    *variable_types,
                    |var_type| {
                        InLevelSymbol::Reg(RegInLevelSymbol {
                            signed: *signed,
                            range: *range,
                            variable_type: var_type,
                        })
                    },
                    scope,
                    ctx,
                    st,
                    diagnostics,
                )
            }
            ModuleOrGenerateItemDeclaration::Integer(id) => {
                let IntegerDeclaration { variable_types } = &**id;
                extend_variable_type_sids(
                    *variable_types,
                    InLevelSymbol::Integer,
                    scope,
                    ctx,
                    st,
                    diagnostics,
                )
            }
            ModuleOrGenerateItemDeclaration::Genvar(id) => {
                let GenvarDeclaration { identifiers } = &**id;
                let mut error = false;
                for ident in identifiers.iter() {
                    error |= try_table_insert(
                        ctx.arenas,
                        &mut ctx.table,
                        scope,
                        ctx.arenas.to_item(ident),
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
                } = &**id;
                let offset = ctx.table_ast_refs.tasks.insert(*id);
                let symbol = TaskSymbol {
                    ast_id: offset,
                    io: Vec::new(),
                    lowered: None,
                };
                let Ok(sid) = try_table_insert(
                    ctx.arenas,
                    &mut ctx.table,
                    scope,
                    *ident,
                    VSymbol::Task(symbol),
                    diagnostics,
                ) else {
                    return Err(());
                };

                let mut error = false;
                for block_item_decl in block_item_decls.iter() {
                    error |= extend_block_item_decl_sid(sid, ctx, st, block_item_decl, diagnostics)
                        .is_err();
                }

                if let StatementOrNull::Statement(stmt) = &**statement_or_null {
                    error |= extend_statements_sids(
                        AstIdRange::single(*stmt),
                        scope,
                        ctx,
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
                } = &**id;

                let offset = ctx.table_ast_refs.fns.insert(*id);
                let symbol = FunctionSymbol {
                    ast_id: offset,
                    inputs: Vec::new(),
                    output: st.dummy_signal,
                    output_ty: VType::SCALAR_NET,
                    lowered: None,
                };
                let Ok(sid) = try_table_insert(
                    ctx.arenas,
                    &mut ctx.table,
                    scope,
                    id.ident,
                    VSymbol::Function(symbol),
                    diagnostics,
                ) else {
                    return Err(());
                };

                let mut error = false;
                for block_item_decl in block_item_decls.iter() {
                    error |= extend_block_item_decl_sid(sid, ctx, st, block_item_decl, diagnostics)
                        .is_err();
                }

                error |= extend_statements_sids(
                    AstIdRange::single(*statement),
                    sid,
                    ctx,
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
            } = &*id;
            extend_param_decl_idents_into_scope(
                scope,
                ctx,
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
            } = &*id;

            let Some(module) = st.module_lut.get(&module_identifier.item.0) else {
                diagnostics.module_not_found(ctx.arenas, *module_identifier);
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
                    time_scale: module.time_scale,
                };
                let symbol = match module_instance.range {
                    None => VSymbol::Module(symbol),
                    Some(_) => VSymbol::ModuleRange(symbol),
                };
                let Ok(sid) = try_table_insert(
                    ctx.arenas,
                    &mut ctx.table,
                    scope,
                    module_instance.name_of_module_instance,
                    symbol,
                    diagnostics,
                ) else {
                    error = true;
                    continue;
                };
                st.insert_lvl_symbol(
                    sid,
                    InLevelSymbol::ModuleInstance(
                        *parameter_value_assignment,
                        module_instance.range,
                        *module,
                    ),
                );
            }
            if error { Err(()) } else { Ok(()) }
        }
        ModuleOrGenerateItemContent::InitialConstruct(id) => {
            let InitialConstruct(id) = &*id;
            extend_statements_sids(AstIdRange::single(*id), scope, ctx, st, diagnostics)
        }
        ModuleOrGenerateItemContent::AlwaysConstruct(id) => {
            let AlwaysConstruct(id) = &*id;
            extend_statements_sids(AstIdRange::single(*id), scope, ctx, st, diagnostics)
        }
        ModuleOrGenerateItemContent::LoopGenerateConstruct(id) => {
            let lvl = ElabLevel::GenerateLoop(id);
            st.next_levels.push_back((scope, lvl, ctx.time_scale));
            Ok(())
        }
        ModuleOrGenerateItemContent::IfGenerateConstruct(id) => {
            let lvl = ElabLevel::GenerateIf(id);
            st.next_levels.push_back((scope, lvl, ctx.time_scale));
            Ok(())
        }
        ModuleOrGenerateItemContent::CaseGenerateConstruct(id) => {
            let lvl = ElabLevel::GenerateCase(id);
            st.next_levels.push_back((scope, lvl, ctx.time_scale));
            Ok(())
        }
    }
}

fn extend_statements_sids<'a, 'b>(
    stmts: AstIdRange<'a, Statement<'a>>,
    scope: SymbolId,
    ctx: &mut LowerContext<'a, '_>,
    st: &mut ElaborationState<'a, 'b>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    assert!(st.stmt_dispatch_stack.is_empty());

    st.stmt_dispatch_stack.push((scope, stmts));

    let mut error = false;
    while let Some((scope, stmts)) = st.stmt_dispatch_stack.pop() {
        macro_rules! dispatch_stmt_or_null {
            ($stmt_or_null:expr) => {
                if let StatementOrNull::Statement(stmt) = &*$stmt_or_null {
                    st.stmt_dispatch_stack
                        .push((scope, AstIdRange::single(*stmt)));
                }
            };
        }

        for stmt in stmts.iter() {
            match stmt.content {
                StatementContent::SeqBlock(id) => {
                    let SeqBlock { block, statements } = &*id;
                    error |= extend_block_sids(scope, ctx, st, diagnostics, *block, *statements)
                        .is_err();
                }
                StatementContent::ParBlock(id) => {
                    let ParBlock { block, statements } = &*id;
                    error |= extend_block_sids(scope, ctx, st, diagnostics, *block, *statements)
                        .is_err();
                }

                StatementContent::CaseStatement(id) => {
                    for item in id.items {
                        dispatch_stmt_or_null!(item.statement_or_null)
                    }
                }
                StatementContent::ConditionalStatement(id) => {
                    let ConditionalStatement {
                        if_branch,
                        else_ifs,
                        else_branch,
                    } = &*id;

                    dispatch_stmt_or_null!(if_branch.statement);
                    for else_if in else_ifs.iter() {
                        dispatch_stmt_or_null!(else_if.statement);
                    }
                    if let Some(stmt_or_null) = else_branch {
                        dispatch_stmt_or_null!(*stmt_or_null);
                    }
                }
                StatementContent::LoopStatement(id) => {
                    st.stmt_dispatch_stack
                        .push((scope, AstIdRange::single(id.statement)));
                }

                StatementContent::ProceduralTimingControlStatement(id) => {
                    dispatch_stmt_or_null!(id.statement_or_null)
                }
                StatementContent::WaitStatement(id) => {
                    dispatch_stmt_or_null!(id.statement_or_null)
                }

                StatementContent::DisableStatement
                | StatementContent::EventTrigger
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

fn extend_block_item_decl_sid<'a, 'b>(
    scope: SymbolId,
    ctx: &mut LowerContext<'a, '_>,
    st: &mut ElaborationState<'a, 'b>,
    block_item_decl: AstId<'a, BlockItemDeclaration<'a>>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    match &*block_item_decl {
        BlockItemDeclaration::Reg {
            signed,
            range,
            identifiers,
        } => extend_variable_type_sids(
            *identifiers,
            |var_type| {
                InLevelSymbol::Reg(RegInLevelSymbol {
                    signed: *signed,
                    range: *range,
                    variable_type: var_type,
                })
            },
            scope,
            ctx,
            st,
            diagnostics,
        ),
        BlockItemDeclaration::Integer(var_types) => extend_variable_type_sids(
            *var_types,
            InLevelSymbol::Integer,
            scope,
            ctx,
            st,
            diagnostics,
        ),
        BlockItemDeclaration::LocalParameterDeclaration(id) => {
            let LocalParameterDeclaration {
                typing,
                assignments,
            } = &**id;
            extend_param_decl_idents_into_scope(
                scope,
                ctx,
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
            } = &**id;
            extend_param_decl_idents_into_scope(
                scope,
                ctx,
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

fn extend_variable_type_sids<'a, 'b>(
    var_types: AstIdRange<'a, VariableType<'a>>,
    f: impl Fn(AstId<'a, VariableType<'a>>) -> InLevelSymbol,
    scope: SymbolId,
    ctx: &mut LowerContext<'a, '_>,
    st: &mut ElaborationState<'a, 'b>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let mut error = false;
    for var_type in var_types.iter() {
        let symbol = NetSymbol {
            ty: VType::SCALAR_NET,
            dims: Vec::new(),
            net: st.dummy_net(),
            transform: VectorTransform::default(),
            port_idx: None,
        };
        let symbol = VSymbol::Net(symbol);

        let Ok(sid) = try_table_insert(
            ctx.arenas,
            &mut ctx.table,
            scope,
            var_type.identifier,
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

impl<'a> InLevelSymbol<'a> {
    pub fn extend_needs<'b>(
        &self,
        sid: SymbolId,
        table: &VSymbolTable,
        st: &mut ElaborationState<'a, 'b>,
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
                match &**typing {
                    ParameterDeclarationTyping::None(_, Some(range)) => {
                        let Range { msb, lsb } = &**range;
                        for e in [*msb, *lsb] {
                            extend_expr_needs(scope, table, st, e);
                        }
                    }
                    ParameterDeclarationTyping::None(..)
                    | ParameterDeclarationTyping::Integer
                    | ParameterDeclarationTyping::Real
                    | ParameterDeclarationTyping::Realtime
                    | ParameterDeclarationTyping::Time => {}
                }

                let exprs: &[_] = match &**expr {
                    ConstantMinTypMaxExpression::Single(e) => &[*e],
                    ConstantMinTypMaxExpression::MinTypMax { min, typ, max } => &[*min, *typ, *max],
                };
                for e in exprs {
                    extend_expr_needs(scope, table, st, *e);
                }
            }
            InLevelSymbol::ModuleInstance(parameter_value_assignment, range, _) => {
                if let Some(parameter_value_assignment) = parameter_value_assignment {
                    match &**parameter_value_assignment {
                        ParameterValueAssignment::Ordered(exprs) => {
                            for e in exprs.iter() {
                                extend_expr_needs(scope, table, st, e);
                            }
                        }
                        ParameterValueAssignment::Named(named_exprs) => {
                            for named_expr in named_exprs.iter() {
                                let Some(expr) = named_expr.expression else {
                                    continue;
                                };
                                let exprs: &[_] = match &*expr {
                                    ConstantMinTypMaxExpression::Single(e) => &[*e],
                                    ConstantMinTypMaxExpression::MinTypMax { min, typ, max } => {
                                        &[*min, *typ, *max]
                                    }
                                };
                                for e in exprs {
                                    extend_expr_needs(scope, table, st, *e);
                                }
                            }
                        }
                    }
                }
                if let Some(range) = range {
                    extend_expr_needs(scope, table, st, range.msb);
                    extend_expr_needs(scope, table, st, range.lsb);
                }
            }
            InLevelSymbol::Net(NetInLevelSymbol {
                decl,
                dim,
                ident: _,
            }) => {
                let NetDeclaration {
                    net_type: _,
                    signed: _,
                    range,
                    nets: _,
                } = &**decl;
                if let Some(range) = range {
                    let Range { msb, lsb } = &**range;
                    for e in [*msb, *lsb] {
                        extend_expr_needs(scope, table, st, e);
                    }
                }
                if let Some(dims) = dim {
                    for dim in dims.iter() {
                        let Dimension { lhs, rhs } = &*dim;
                        for e in [*lhs, *rhs] {
                            extend_expr_needs(scope, table, st, e);
                        }
                    }
                }
            }
            InLevelSymbol::Reg(RegInLevelSymbol {
                signed: _,
                range,
                variable_type,
            }) => {
                if let Some(range) = range {
                    let Range { msb, lsb } = &**range;
                    for e in [*msb, *lsb] {
                        extend_expr_needs(scope, table, st, e);
                    }
                }

                extend_var_type_needs(scope, table, st, *variable_type)
            }
            InLevelSymbol::Integer(var_type) => extend_var_type_needs(scope, table, st, *var_type),
            InLevelSymbol::Port(PortInLevelSymbol { decl, ident: _ }) => {
                let range = match &**decl {
                    PortDeclaration::Inout(id) => id.range,
                    PortDeclaration::Input(id) => id.range,
                    PortDeclaration::Output(id) => id.range,
                };

                if let Some(range) = range {
                    let Range { msb, lsb } = &*range;
                    for e in [*msb, *lsb] {
                        extend_expr_needs(scope, table, st, e);
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
                } = &**id;

                for task_port in task_ports.iter() {
                    let TaskPortItem {
                        attribute_instances: _,
                        content,
                    } = &*task_port;
                    let tf_type = match content {
                        TaskPortItemContent::Input(decl) => &decl.tf_type,
                        TaskPortItemContent::Output(decl) => &decl.tf_type,
                        TaskPortItemContent::Inout(decl) => &decl.tf_type,
                    };

                    extend_tf_type_needs(scope, table, st, tf_type);
                }
            }
            InLevelSymbol::Function(id) => {
                let FunctionDeclaration {
                    automatic: _,
                    range_or_type,
                    ident: _,
                    tf_input_decls,
                    block_item_decls: _,
                    statement,
                } = &**id;

                if let FunctionRangeOrType::Signed(Some(range))
                | FunctionRangeOrType::Unsigned(Some(range)) = &**range_or_type
                {
                    let Range { msb, lsb } = &**range;
                    for e in [*msb, *lsb] {
                        extend_expr_needs(scope, table, st, e);
                    }
                }
                for tf_input_decl in tf_input_decls.iter() {
                    extend_tf_type_needs(sid, table, st, &tf_input_decl.tf_type);
                }
                extend_fn_statement_needs(sid, table, st, AstIdRange::single(*statement));
            }
        }
    }
}

fn extend_tf_type_needs<'a, 'b>(
    scope: SymbolId,
    table: &VSymbolTable,
    st: &mut ElaborationState<'a, 'b>,
    tf_type: &TfType<'a>,
) {
    if let TfType::Net {
        reg: _,
        signed: _,
        range: Some(range),
    } = tf_type
    {
        let Range { msb, lsb } = &**range;
        for e in [*msb, *lsb] {
            extend_expr_needs(scope, table, st, e);
        }
    }
}

pub fn extend_var_type_needs<'a, 'b>(
    scope: SymbolId,
    table: &VSymbolTable,
    st: &mut ElaborationState<'a, 'b>,
    var_type: AstId<'a, VariableType<'a>>,
) {
    match var_type.variant {
        VariableTypeVariant::Dimensions(dims) => {
            for dim in dims.iter() {
                let Dimension { lhs, rhs } = &*dim;
                for e in [*lhs, *rhs] {
                    extend_expr_needs(scope, table, st, e);
                }
            }
        }
        VariableTypeVariant::ConstantExpr(e) => {
            extend_expr_needs(scope, table, st, e);
        }
    }
}

pub fn extend_expr_needs<'a, 'b>(
    scope: SymbolId,
    table: &VSymbolTable,
    st: &mut ElaborationState<'a, 'b>,
    expr: AstId<'a, ConstantExpr<'a>>,
) {
    let expr = expr.into_expr();
    assert!(st.dispatch_stack.is_empty());

    let dispatch_stack = &mut st.dispatch_stack;
    dispatch_stack.push(expr);

    while let Some(item) = dispatch_stack.pop() {
        match &*item {
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
                if let Some(bit_slice) = bit_slice {
                    dispatch_stack.extend(bit_slice.exprs());
                }

                if let Some(ident_sid) = resolve_hident(scope, table, *ident)
                    && st.marked.insert(ident_sid)
                    && st.lvl_symbols.contains_key(&ident_sid)
                {
                    st.needs_adjacency_list_items.push(ident_sid);
                };
            }
            Expr::FunctionCall(ident, exprs) => {
                dispatch_stack.extend(exprs.iter());

                if let Some(ident_sid) = resolve_hident(scope, table, *ident)
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

fn extend_fn_statement_needs<'a, 'b>(
    scope: SymbolId,
    table: &VSymbolTable,
    st: &mut ElaborationState<'a, 'b>,
    stmts: AstIdRange<'a, Statement<'a>>,
) {
    assert!(st.stmt_dispatch_stack.is_empty());

    st.stmt_dispatch_stack.push((scope, stmts));

    while let Some((scope, stmts)) = st.stmt_dispatch_stack.pop() {
        macro_rules! dispatch_stmt_or_null {
            ($stmt_or_null:expr) => {
                if let StatementOrNull::Statement(stmt) = &*$stmt_or_null {
                    st.stmt_dispatch_stack
                        .push((scope, AstIdRange::single(*stmt)));
                }
            };
        }

        for stmt in stmts.iter() {
            match stmt.content {
                StatementContent::SeqBlock(id) => {
                    let SeqBlock {
                        block: _,
                        statements,
                    } = &*id;
                    extend_fn_statement_needs(scope, table, st, *statements);
                }
                StatementContent::ParBlock(id) => {
                    let ParBlock {
                        block: _,
                        statements,
                    } = &*id;
                    extend_fn_statement_needs(scope, table, st, *statements);
                }

                StatementContent::CaseStatement(id) => {
                    let CaseStatement {
                        variant: _,
                        expr,
                        items,
                    } = &*id;
                    extend_expr_needs(scope, table, st, (*expr).into_constant());
                    for item in items.iter() {
                        dispatch_stmt_or_null!(item.statement_or_null)
                    }
                }
                StatementContent::ConditionalStatement(id) => {
                    let ConditionalStatement {
                        if_branch,
                        else_ifs,
                        else_branch,
                    } = &*id;

                    extend_expr_needs(scope, table, st, if_branch.condition.into_constant());
                    dispatch_stmt_or_null!(if_branch.statement);
                    for else_if in else_ifs.iter() {
                        extend_expr_needs(scope, table, st, else_if.condition.into_constant());
                        dispatch_stmt_or_null!(else_if.statement);
                    }
                    if let Some(stmt_or_null) = else_branch {
                        dispatch_stmt_or_null!(*stmt_or_null);
                    }
                }
                StatementContent::LoopStatement(id) => {
                    let LoopStatement { variant, statement } = &*id;
                    match variant {
                        LoopStatementVariant::Forever => {}
                        LoopStatementVariant::Repeat(expr) | LoopStatementVariant::While(expr) => {
                            extend_expr_needs(scope, table, st, (*expr).into_constant())
                        }
                        LoopStatementVariant::For(_, _, _) => {
                            // @TODO: Actually use the expressions here.
                        }
                    }
                    st.stmt_dispatch_stack
                        .push((scope, AstIdRange::single(*statement)));
                }

                StatementContent::ProceduralTimingControlStatement(id) => {
                    dispatch_stmt_or_null!(id.statement_or_null)
                }
                StatementContent::WaitStatement(id) => {
                    let WaitStatement {
                        expression,
                        statement_or_null,
                    } = &*id;
                    extend_expr_needs(scope, table, st, (*expression).into_constant());
                    dispatch_stmt_or_null!(*statement_or_null)
                }

                // @TODO: Use the expressions here.
                StatementContent::BlockingAssignment(id) => {
                    let BlockingAssignment {
                        variable_lvalue: _,
                        delay_or_event_control: _,
                        expression,
                    } = &*id;
                    // @TODO: Use the expressions here.
                    extend_expr_needs(scope, table, st, (*expression).into_constant());
                }
                StatementContent::NonBlockingAssignment(_)
                | StatementContent::SystemTaskEnable(_)
                | StatementContent::TaskEnable(_) => {}

                StatementContent::DisableStatement
                | StatementContent::EventTrigger
                | StatementContent::ProceduralContinuousAssignments => {}
            }
        }
    }
}

pub fn finalize_symbol<'a>(
    gl: &mut GlobalContext,

    symbol: &InLevelSymbol<'a>,
    sid: SymbolId,
    scope: SymbolId,
    ctx: &mut LowerContext<'a, '_>,
    next_levels: &mut VecDeque<(SymbolId, ElabLevel<'a>, TimeScale)>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    match symbol {
        InLevelSymbol::Param(typing, constant_expr, module_sid) => {
            let (typing, constant_expr) = (*typing, *constant_expr);
            use ParameterDeclarationTyping as T;
            let (_, _, ty) = match &*typing {
                T::None(signed, range) => match range {
                    None => (0, 0, None),
                    Some(ast_range) => {
                        let (msb, lsb, width) = super::eval_constant_range(
                            gl,
                            ctx.arenas,
                            scope,
                            &ctx.table,
                            diagnostics,
                            *ast_range,
                        )?;
                        (msb, lsb, Some(VType::net(width, *signed)))
                    }
                },
                T::Integer => (31, 0, Some(VType::SignedNet(INTEGER_VSIZE))),
                T::Real | T::Realtime | T::Time => {
                    diagnostics.not_yet_implemented(
                        ctx.arenas.get_span(typing),
                        "real / realtime / time parameter",
                    );
                    return Err(());
                }
            };
            let ident = ctx.table[sid].name();
            let value = if let Some(module_sid) = module_sid
                && let module = unwrap_get_module(&ctx.table, *module_sid)
                && let Some(param_override_idx) = module.parameter_overrides.get(&ident)
            {
                module.parameter_override_values[*param_override_idx].clone()
            } else {
                match &*constant_expr {
                    ConstantMinTypMaxExpression::Single(id) => eval_constant_expr(
                        gl,
                        ctx.arenas,
                        &ctx.table,
                        scope,
                        diagnostics,
                        *id,
                        None,
                    )?,
                    ConstantMinTypMaxExpression::MinTypMax { .. } => todo!(),
                }
            };

            let ty = ty.unwrap_or_else(|| value.ty());
            let value = value.coerce(&ty);

            *unwrap_get_param_mut(&mut ctx.table, sid) = value;
        }
        InLevelSymbol::Reg(RegInLevelSymbol {
            signed,
            range,
            variable_type,
        }) => {
            let VariableType {
                identifier,
                variant,
            } = &**variable_type;
            let (ty, transform) = match range {
                None => (
                    VType::net(SCALAR_VSIZE, *signed),
                    VectorTransform::default(),
                ),
                Some(range) => {
                    let (transform, size) = evaluate_net_msb_lsb(
                        gl,
                        ctx.arenas,
                        *range,
                        scope,
                        &ctx.table,
                        diagnostics,
                    )?;
                    (VType::net(size, *signed), transform)
                }
            };
            let parent = ctx.table[sid].parent().unwrap();
            let (dims, initialize) = match variant {
                VariableTypeVariant::Dimensions(dimensions) => (
                    super::dims_to_array_elab(
                        gl,
                        ctx.arenas,
                        parent,
                        &ctx.table,
                        diagnostics,
                        *dimensions,
                    )?,
                    None,
                ),
                VariableTypeVariant::ConstantExpr(expr) => (
                    Vec::new(),
                    Some(
                        eval_constant_expr(
                            gl,
                            ctx.arenas,
                            &ctx.table,
                            scope,
                            diagnostics,
                            *expr,
                            Some(ty.force_net_width()),
                        )?
                        .coerce(&ty),
                    ),
                ),
            };
            let net = unwrap_get_net_mut(&mut ctx.table, sid);
            net.net = super::new_net(
                gl,
                ctx.logic_mode,
                ctx.arenas,
                &ty,
                &dims,
                *identifier,
                initialize,
            );
            net.dims = dims;
            net.transform = transform;
            net.ty = ty;
        }
        InLevelSymbol::Net(NetInLevelSymbol { decl, dim, ident }) => {
            let NetDeclaration {
                signed,
                range,
                net_type: _,
                nets: _,
            } = &**decl;

            let (ty, transform) = match range {
                None => (
                    VType::net(SCALAR_VSIZE, *signed),
                    VectorTransform::default(),
                ),
                Some(range) => {
                    let (transform, size) = evaluate_net_msb_lsb(
                        gl,
                        ctx.arenas,
                        *range,
                        scope,
                        &ctx.table,
                        diagnostics,
                    )?;
                    (VType::net(size, *signed), transform)
                }
            };
            let parent = ctx.table[sid].parent().unwrap();
            let dims = match dim {
                None => Vec::new(),
                Some(dims) => super::dims_to_array_elab(
                    gl,
                    ctx.arenas,
                    parent,
                    &ctx.table,
                    diagnostics,
                    *dims,
                )?,
            };
            let net = unwrap_get_net_mut(&mut ctx.table, sid);
            net.net = super::new_net(gl, ctx.logic_mode, ctx.arenas, &ty, &dims, *ident, None);
            net.dims = dims;
            net.transform = transform;
            net.ty = ty;
        }
        InLevelSymbol::Integer(id) => {
            let VariableType {
                identifier,
                variant,
            } = &**id;
            let parent = ctx.table[sid].parent().unwrap();
            let ty = VType::SignedNet(INTEGER_VSIZE);
            let (dims, initialize) = match variant {
                VariableTypeVariant::Dimensions(dimensions) => (
                    super::dims_to_array_elab(
                        gl,
                        ctx.arenas,
                        parent,
                        &ctx.table,
                        diagnostics,
                        *dimensions,
                    )?,
                    None,
                ),
                VariableTypeVariant::ConstantExpr(expr) => (
                    Vec::new(),
                    Some(
                        eval_constant_expr(
                            gl,
                            ctx.arenas,
                            &ctx.table,
                            scope,
                            diagnostics,
                            *expr,
                            Some(ty.force_net_width()),
                        )?
                        .coerce(&ty),
                    ),
                ),
            };
            let net = unwrap_get_net_mut(&mut ctx.table, sid);
            net.net = super::new_net(
                gl,
                ctx.logic_mode,
                ctx.arenas,
                &ty,
                &dims,
                *identifier,
                initialize,
            );
            net.dims = dims;
            net.ty = VType::SignedNet(INTEGER_VSIZE);
        }
        InLevelSymbol::Port(PortInLevelSymbol { decl, ident }) => {
            let (ty, transform, _, _) =
                port_declaration_to_info(gl, ctx.arenas, *decl, scope, &ctx.table, diagnostics)?;
            let net = unwrap_get_net_mut(&mut ctx.table, sid);
            net.net = super::new_net(gl, ctx.logic_mode, ctx.arenas, &ty, &[], *ident, None);
            net.transform = transform;
            net.ty = ty;
        }
        InLevelSymbol::Task(_) => {
            super::function::elaborate_task(gl, sid, ctx, diagnostics)?;
        }
        InLevelSymbol::Function(id) => {
            super::function::elaborate_fn(gl, sid, ctx, diagnostics)?;
            let mut mctx = MutLowerContext {
                gl: std::mem::take(gl),
                nbas: IndexMap::default(),
                diagnostics: std::mem::take(diagnostics),
                connections: Vec::new(),
                fuse_scratch: Vec::new(),
                has_vcd: false,
            };
            // @TODO: This should ignore errors with unresolved symbols.
            let res =
                crate::lower::module_or_generate_item::function::lower(ctx, &mut mctx, sid, *id);

            std::mem::swap(gl, &mut mctx.gl);
            std::mem::swap(diagnostics, &mut mctx.diagnostics);

            res?;
        }
        InLevelSymbol::ModuleInstance(parameter_value_assignment, range, module) => {
            let (parameter_overrides, parameter_override_values) = match *parameter_value_assignment
            {
                None => Default::default(),
                Some(id) => match &*id {
                    ParameterValueAssignment::Ordered(ids) => {
                        let mut params = Vec::new();
                        for id in ids.iter() {
                            let value = eval_constant_expr(
                                gl,
                                ctx.arenas,
                                &ctx.table,
                                scope,
                                diagnostics,
                                id,
                                None,
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
                            } = &*n;
                            let Some(expression) = expression else {
                                diagnostics.not_yet_implemented(
                                    ctx.arenas.get_span(n),
                                    "null parameter assignment",
                                );
                                return Err(());
                            };
                            let ConstantMinTypMaxExpression::Single(expression) = &**expression
                            else {
                                diagnostics.not_yet_implemented(
                                    ctx.arenas.get_span(n),
                                    "mintypmax parameter assignment",
                                );
                                return Err(());
                            };
                            let value = eval_constant_expr(
                                gl,
                                ctx.arenas,
                                &ctx.table,
                                scope,
                                diagnostics,
                                *expression,
                                None,
                            )?;
                            params.insert(identifier.item.0, param_values.len());
                            param_values.push(value);
                        }
                        (params, param_values)
                    }
                },
            };

            match range {
                None => {
                    let module_symbol = unwrap_get_module_mut(&mut ctx.table, sid);
                    module_symbol.parameter_overrides = Arc::new(parameter_overrides);
                    module_symbol.parameter_override_values = Arc::new(parameter_override_values);
                    next_levels.push_back((sid, ElabLevel::Module(*module), module.time_scale));
                }
                Some(range) => {
                    let msb = eval_constant_expr(
                        gl,
                        ctx.arenas,
                        &ctx.table,
                        scope,
                        diagnostics,
                        range.msb,
                        None,
                    )?;
                    let lsb = eval_constant_expr(
                        gl,
                        ctx.arenas,
                        &ctx.table,
                        scope,
                        diagnostics,
                        range.lsb,
                        None,
                    )?;

                    let VSymbol::ModuleRange(module_symbol) = &mut ctx.table[sid].content else {
                        unreachable!();
                    };
                    module_symbol.parameter_overrides = Arc::new(parameter_overrides);
                    module_symbol.parameter_override_values = Arc::new(parameter_override_values);

                    let (Some(msb), Some(lsb)) = (msb.as_integer(), lsb.as_integer()) else {
                        diagnostics.not_yet_implemented(ctx.arenas.get_span(*range), "not integer");
                        return Err(());
                    };

                    let num_instances = (msb.abs_diff(lsb) + 1) as usize;

                    // @TODO: Instead of re-elaborating each instance again. Elaborate once and
                    // deepcopy from there, we know the parameters are the same so the tree should
                    // be the same.
                    //
                    // We add this main expansion to allow for signal resolving against the main
                    // instance.
                    next_levels.push_back((
                        sid,
                        ElabLevel::ModuleRange(*module, num_instances),
                        module.time_scale,
                    ));
                }
            }
        }
    }

    Ok(())
}

fn extend_block_sids<'a, 'b>(
    scope: SymbolId,
    ctx: &mut LowerContext<'a, '_>,
    st: &mut ElaborationState<'a, 'b>,
    diagnostics: &mut Diagnostics,
    block: Option<AstId<'a, Block<'a>>>,
    stmts: AstIdRange<'a, Statement<'a>>,
) -> Result<(), ()> {
    let mut scope = scope;
    if let Some(block) = block {
        let Block {
            block_identifier,
            block_item_decls,
        } = &*block;

        let named_block_scope = try_table_insert(
            ctx.arenas,
            &mut ctx.table,
            scope,
            *block_identifier,
            VSymbol::NamedBlock,
            diagnostics,
        )?;
        scope = named_block_scope;

        let mut error = false;
        for block_item_decl in block_item_decls.iter() {
            error |=
                extend_block_item_decl_sid(scope, ctx, st, block_item_decl, diagnostics).is_err();
        }
        if error {
            return Err(());
        }
    }
    st.stmt_dispatch_stack.push((scope, stmts));
    Ok(())
}

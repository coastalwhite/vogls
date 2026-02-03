use core::fmt;
use std::num::NonZeroU32;
use std::sync::Arc;

use vogls_frontend::VgHashMap;
use vogls_frontend::ident_table::{IdentId, IdentTable};
use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{
    BasicBlockKey, ConnectionDirection, INTEGER_VSIZE, ProcessKey, SCALAR_VSIZE, Signal, SignalKey,
    VariableKey, VectorSize,
};

use crate::ast::constant_expr::{ConstantExpr, ConstantMinTypMaxExpression};
use crate::ast::module::{
    BlockItemDeclaration, CaseGenerateConstruct, CaseGenerateItem, CaseGeneratePattern, Dimension,
    FunctionDeclaration, GenerateBlock, GenvarAssignment, GenvarDeclaration, IfGenerateConstruct,
    IntegerDeclaration, LocalParameterDeclaration, LoopGenerateConstruct, Module, ModuleInstance,
    ModuleInstantiation, ModuleItem, ModuleOrGenerateItem, ModuleOrGenerateItemDeclaration,
    ModulePorts, NamedParameterAssignment, NetDeclAssignment, NetDeclaration, NetDeclarationNets,
    NetIdent, NetType, NonPortModuleItem, ParamAssignment, ParameterDeclaration,
    ParameterDeclarationTyping, ParameterValueAssignment, Port, PortDeclaration, PortExpression,
    PortReference, Range, RegDeclaration, TaskDeclaration, VariableType, VariableTypeVariant,
};
use crate::ast::statement::{
    Block, CaseItem, CaseStatement, ConditionalStatement, IfBranch, LoopStatement,
    ProceduralTimingControlStatement, SeqBlock, Statement, StatementContent, StatementOrNull,
    WaitStatement,
};
use crate::ast::{AstId, AstIdRange, AstItem, Identifier};
use crate::lower::{
    Diagnostics, EvalScope, VType, VValue, eval_constant_expr, unwrap_get_module_mut,
};
use crate::parser::AstArenas;

pub mod function;

pub type VSymbolTable = vogls_frontend::symbol_table::SymbolTable<VSymbol>;

pub enum VSymbol {
    Module(ModuleSymbol),
    Parameter(VValue),
    Net(NetSymbol),
    NamedBlock,
    GenerateBlock(AstIdRange<ModuleOrGenerateItem>),
    GenVar,
    Task(TaskSymbol),
    Function(FunctionSymbol),
}

impl fmt::Debug for VSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            VSymbol::Module(_) => "module",
            VSymbol::Parameter(_) => "param",
            VSymbol::Net(_) => "net",
            VSymbol::NamedBlock => "named_block",
            VSymbol::GenerateBlock(_) => "generate_block",
            VSymbol::GenVar => "genvar",
            VSymbol::Task(_) => "task",
            VSymbol::Function(_) => "function",
        })
    }
}

pub struct ModuleSymbol {
    pub module: IdentId,

    pub ports: Vec<(SymbolId, ConnectionDirection)>,
    pub parameters: Vec<SymbolId>,

    pub parameter_overrides: Arc<VgHashMap<IdentId, usize>>,
    pub parameter_override_values: Arc<Vec<VValue>>,
}

pub struct NetSymbol {
    pub ty: VType,
    pub dims: Vec<u32>,
    pub signal: vogls_ir::SignalKey,
    pub nba: Option<(ProcessKey, SignalKey, SignalKey)>,
    pub port_idx: Option<usize>,
}

pub struct FunctionSymbol {
    pub ast_id: AstId<FunctionDeclaration>,
    pub lowered: Option<LoweredFunction>,
}

pub struct TaskSymbol {
    pub ast_id: AstId<TaskDeclaration>,
    pub lowered: Option<LoweredTask>,
}

#[derive(Clone)]
pub struct LoweredFunction {
    pub entry: BasicBlockKey,
    pub input_vars: Vec<VariableKey>,
    pub input_types: Vec<VType>,
    pub output_var: VariableKey,
    pub output_ty: VType,
}

#[derive(Clone)]
pub struct LoweredTask {
    pub entry: BasicBlockKey,
    pub io_vars: Vec<VariableKey>,
    pub io_types: Vec<(ConnectionDirection, VType)>,
}

pub fn try_table_insert(
    arenas: &AstArenas,
    table: &mut VSymbolTable,
    parent: SymbolId,
    name: AstItem<Identifier>,
    content: VSymbol,
    diagnostics: &mut Diagnostics,
) -> Result<SymbolId, ()> {
    let Ok(symid) = table.insert(name.item.0, parent, arenas.get_item_span(name), content) else {
        diagnostics.duplicate_definition(arenas, name);
        return Err(());
    };

    Ok(symid)
}
pub fn table_recursive_resolve(
    table: &VSymbolTable,
    parent: SymbolId,
    name: IdentId,
) -> Option<SymbolId> {
    // @TODO: Actually recursively resolve
    table.resolve(parent, name)
}
pub fn try_table_resolve(
    arenas: &AstArenas,
    table: &VSymbolTable,
    parent: SymbolId,
    name: AstItem<Identifier>,
    diagnostics: &mut Diagnostics,
) -> Result<SymbolId, ()> {
    let Some(symid) = table_recursive_resolve(table, parent, name.item.0) else {
        diagnostics.var_not_found(arenas, name);
        return Err(());
    };
    Ok(symid)
}

pub fn elaborate_module<'a>(
    signals: &mut slotmap::SlotMap<SignalKey, Signal>,
    arenas: &'a AstArenas,
    module: AstId<Module>,
    module_symid: SymbolId,
    table: &mut VSymbolTable,
    module_instances_todo: &mut Vec<SymbolId>,
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

    let symbol = &table[module_symid];
    let VSymbol::Module(elab_module) = &symbol.content else {
        unreachable!("elaborated module is not a module");
    };
    let mut param_override_is_used = vec![false; elab_module.parameter_override_values.len()];

    if let Some(module_parameter_port_list) = module_parameter_port_list {
        for id in module_parameter_port_list.iter() {
            let ParameterDeclaration {
                typing,
                assignments,
            } = arenas.get(id);

            // @TODO:
            // We need to immediately exit here as a failed elaboration will have knock on effects
            // for future parameters.
            //
            // We should add the parameters into the scope, but mark them erroneous. When an
            // erroneous parameter is used later, it would then quietly ignore that elaboration and
            // continue.
            //
            // This way, you can get the broadest error messages.
            elaborate_parameter_declaration(
                arenas,
                *typing,
                *assignments,
                module_symid,
                table,
                diagnostics,
                Some(&mut param_override_is_used),
            )?;
        }
    }

    let mut error = false;
    match ports {
        ModulePorts::Ports(ports) => {
            for id in ports.iter() {
                match arenas.get(id) {
                    Port::PortExpression(id) => {
                        let PortExpression { references } = arenas.get(*id);
                        let PortReference { identifier } = arenas.get(*references);

                        let signal =
                            new_signal(signals, arenas, &VType::SCALAR_NET, &[], *identifier);

                        let symbol = &table[module_symid];
                        let VSymbol::Module(elab_module) = &symbol.content else {
                            unreachable!("elaborated module is not a module");
                        };
                        let port_idx = elab_module.ports.len();

                        let Ok(symid) = try_table_insert(
                            arenas,
                            table,
                            module_symid,
                            *identifier,
                            VSymbol::Net(NetSymbol {
                                ty: VType::SCALAR_NET,
                                dims: Vec::new(),
                                signal,
                                nba: None,
                                port_idx: Some(port_idx),
                            }),
                            diagnostics,
                        ) else {
                            error = true;
                            continue;
                        };

                        unwrap_get_module_mut(table, module_symid)
                            .ports
                            .push((symid, ConnectionDirection::Both));
                    }
                }
            }
        }
        ModulePorts::PortDeclarations(port_declarations) => {
            let mut error = false;
            for id in port_declarations.iter() {
                let Ok((ty, direction, identifiers)) =
                    port_declaration_to_info(arenas, id, module_symid, table, diagnostics)
                else {
                    error = true;
                    continue;
                };

                let symbol = &table[module_symid];
                let VSymbol::Module(module) = &symbol.content else {
                    unreachable!("non-local parameter can only be defined at module-level");
                };
                let mut port_idx = module.ports.len();

                for ident in identifiers.iter() {
                    let ident = arenas.to_item(ident);
                    let signal = new_signal(signals, arenas, &ty, &[], ident);
                    let Ok(symid) = try_table_insert(
                        arenas,
                        table,
                        module_symid,
                        ident,
                        VSymbol::Net(NetSymbol {
                            ty,
                            dims: Vec::new(),
                            signal,
                            nba: None,
                            port_idx: Some(port_idx),
                        }),
                        diagnostics,
                    ) else {
                        error = true;
                        continue;
                    };

                    let symbol = &mut table[module_symid];
                    let VSymbol::Module(module) = &mut symbol.content else {
                        unreachable!("non-local parameter can only be defined at module-level");
                    };

                    module.ports.push((symid, direction));
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
                let Ok((ty, direction, identifiers)) =
                    port_declaration_to_info(arenas, *id, module_symid, table, diagnostics)
                else {
                    error = true;
                    continue;
                };

                for ident in identifiers.iter() {
                    let Some(sid) = table.resolve(module_symid, arenas.get(ident).0) else {
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

                    signals[net.signal].size = ty.force_net_width();
                    net.ty = ty;
                    unwrap_get_module_mut(table, module_symid).ports[port_idx].1 = direction;
                }
            }
            ModuleItem::NonPortModuleItem(id) => match arenas.get(*id) {
                NonPortModuleItem::ModuleOrGenerateItem(id) => {
                    error |= elaborate_module_or_generate_item(
                        signals,
                        arenas,
                        *id,
                        module_symid,
                        table,
                        module_instances_todo,
                        diagnostics,
                    )
                    .is_err();
                }
                NonPortModuleItem::GenerateRegion(region) => {
                    for id in region.module_or_generate_item.iter() {
                        error |= elaborate_module_or_generate_item(
                            signals,
                            arenas,
                            id,
                            module_symid,
                            table,
                            module_instances_todo,
                            diagnostics,
                        )
                        .is_err();
                    }
                }
                NonPortModuleItem::SpecifyBlock => todo!(),
                NonPortModuleItem::ParameterDeclaration(id) => {
                    let ParameterDeclaration {
                        typing,
                        assignments,
                    } = arenas.get(*id);
                    elaborate_parameter_declaration(
                        arenas,
                        *typing,
                        *assignments,
                        module_symid,
                        table,
                        diagnostics,
                        Some(&mut param_override_is_used),
                    )?;
                }
                NonPortModuleItem::SpecParamDeclaration => todo!(),
            },
        }
    }

    if !param_override_is_used.iter().all(|v| *v) {
        diagnostics.not_yet_implemented(arenas.get_span(module), "unused parameter override");
        error = true;
    }

    if error {
        return Err(());
    }

    Ok(())
}

pub fn elaborate_parameter_declaration<'a>(
    arenas: &'a AstArenas,

    typing: AstId<ParameterDeclarationTyping>,
    assignments: AstIdRange<ParamAssignment>,

    parent: SymbolId,
    table: &mut VSymbolTable,
    diagnostics: &mut Diagnostics,
    mut param_override_is_used: Option<&mut [bool]>,
) -> Result<(), ()> {
    let (_, _, ty) = match arenas.get(typing) {
        ParameterDeclarationTyping::None(signed, range) => {
            let (msb, lsb, width) = match range {
                None => (0, 0, SCALAR_VSIZE),
                Some(ast_range) => {
                    eval_constant_range(arenas, parent, table, diagnostics, *ast_range)?
                }
            };
            (msb, lsb, VType::net(width, *signed))
        }
        ParameterDeclarationTyping::Integer => (31, 0, VType::SignedNet(INTEGER_VSIZE)),
        ParameterDeclarationTyping::Real
        | ParameterDeclarationTyping::Realtime
        | ParameterDeclarationTyping::Time => {
            diagnostics
                .not_yet_implemented(arenas.get_span(typing), "real / realtime / time parameter");
            return Err(());
        }
    };

    for assignment in assignments.iter() {
        let ParamAssignment { param, constant } = arenas.get(assignment);
        let name = param.item.0;
        let mut value = match arenas.get(*constant) {
            ConstantMinTypMaxExpression::Single(id) => {
                eval_constant_expr_elab(arenas, parent, table, diagnostics, *id)?
            }
            ConstantMinTypMaxExpression::MinTypMax { .. } => todo!(),
        };

        if let Some(param_override_is_used) = param_override_is_used.as_mut() {
            let symbol = &mut table[parent];
            let VSymbol::Module(module) = &mut symbol.content else {
                unreachable!("non-local parameter can only be defined at module-level");
            };

            let override_idx = if module.parameter_overrides.is_empty() {
                // Ordered parameter overrides.
                (module.parameters.len() < module.parameter_override_values.len())
                    .then_some(module.parameters.len())
            } else {
                // Named parameter overrides.
                module.parameter_overrides.get(&name).copied()
            };

            if let Some(override_idx) = override_idx {
                param_override_is_used[override_idx] = true;
                value = module.parameter_override_values[override_idx].clone();
            }
        }

        value = value.truncate_or_extend(ty.force_net_width());

        let param_symid = try_table_insert(
            arenas,
            table,
            parent,
            *param,
            VSymbol::Parameter(value),
            diagnostics,
        )?;

        if param_override_is_used.is_some() {
            let symbol = &mut table[parent];
            let VSymbol::Module(module) = &mut symbol.content else {
                unreachable!("non-local parameter can only be defined at module-level");
            };
            module.parameters.push(param_symid);
        }
    }

    Ok(())
}

pub fn port_declaration_to_info<'a>(
    arenas: &'a AstArenas,

    id: AstId<PortDeclaration>,

    parent: SymbolId,
    table: &mut VSymbolTable,
    diagnostics: &mut Diagnostics,
) -> Result<(VType, ConnectionDirection, AstIdRange<Identifier>), ()> {
    use ConnectionDirection as D;
    let (direction, range, signed, identifiers) = match arenas.get(id) {
        PortDeclaration::Inout(id) => {
            let inout = arenas.get(*id);
            (D::Both, inout.range, inout.signed, inout.port_identifiers)
        }
        PortDeclaration::Input(id) => {
            let input = arenas.get(*id);
            (D::In, input.range, input.signed, input.port_identifiers)
        }
        PortDeclaration::Output(id) => {
            let output = arenas.get(*id);
            (D::Out, output.range, output.signed, output.identifiers)
        }
    };

    let (_, _, size) = match range {
        None => (0, 0, SCALAR_VSIZE),
        Some(range) => eval_constant_range(arenas, parent, table, diagnostics, range)?,
    };
    let ty = VType::net(size, signed);
    Ok((ty, direction, identifiers))
}

pub fn elaborate_module_or_generate_item<'a>(
    signals: &mut slotmap::SlotMap<SignalKey, Signal>,
    arenas: &'a AstArenas,

    id: AstId<ModuleOrGenerateItem>,

    parent: SymbolId,
    table: &mut VSymbolTable,
    module_instances_todo: &mut Vec<SymbolId>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    match arenas.get(id) {
        ModuleOrGenerateItem::ModuleOrGenerateItemDeclaration(id) => {
            elaborate_module_or_generate_item_declaration(
                signals,
                arenas,
                *id,
                parent,
                table,
                diagnostics,
            )
        }
        ModuleOrGenerateItem::LocalParameterDeclaration(id) => {
            let LocalParameterDeclaration {
                typing,
                assignments,
            } = arenas.get(*id);
            elaborate_parameter_declaration(
                arenas,
                *typing,
                *assignments,
                parent,
                table,
                diagnostics,
                None,
            )
        }
        ModuleOrGenerateItem::ParameterOverride => todo!(),
        ModuleOrGenerateItem::ContinuousAssign(_) => Ok(()),

        // @TODO: This actually also needs to be elaborated somewhat. I am not 100% sure how or
        // what though.
        ModuleOrGenerateItem::GateInstantiation(_) => Ok(()),

        ModuleOrGenerateItem::UdpInstantiation => todo!(),
        ModuleOrGenerateItem::ModuleInstantiation(id) => {
            let ModuleInstantiation {
                module_identifier,
                parameter_value_assignment,
                module_instances,
            } = arenas.get(*id);

            let (parameter_overrides, parameter_override_values) = match parameter_value_assignment
            {
                None => Default::default(),
                Some(id) => match arenas.get(*id) {
                    ParameterValueAssignment::Ordered(ids) => {
                        let mut params = Vec::new();
                        for id in ids.iter() {
                            let value =
                                eval_constant_expr_elab(arenas, parent, table, diagnostics, id)?;
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
                            let value = eval_constant_expr_elab(
                                arenas,
                                parent,
                                table,
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

            let parameter_overrides = Arc::new(parameter_overrides);
            let parameter_override_values = Arc::new(parameter_override_values);

            for module_instance in module_instances.iter() {
                let ModuleInstance {
                    name_of_module_instance,
                    list_of_port_connections: _,
                } = arenas.get(module_instance);

                let symid = try_table_insert(
                    arenas,
                    table,
                    parent,
                    *name_of_module_instance,
                    VSymbol::Module(ModuleSymbol {
                        module: module_identifier.item.0,
                        ports: Vec::new(),
                        parameters: Vec::new(),
                        parameter_overrides: parameter_overrides.clone(),
                        parameter_override_values: parameter_override_values.clone(),
                    }),
                    diagnostics,
                )?;
                module_instances_todo.push(symid);
            }
            Ok(())
        }
        ModuleOrGenerateItem::InitialConstruct(id) => elaborate_statements(
            signals,
            arenas,
            parent,
            table,
            diagnostics,
            AstIdRange::single(arenas.get(*id).0),
        ),
        ModuleOrGenerateItem::AlwaysConstruct(id) => elaborate_statements(
            signals,
            arenas,
            parent,
            table,
            diagnostics,
            AstIdRange::single(arenas.get(*id).0),
        ),
        ModuleOrGenerateItem::LoopGenerateConstruct(id) => {
            let LoopGenerateConstruct {
                initialization,
                condition,
                iteration,
                block,
            } = arenas.get(*id);

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
            let symid =
                try_table_resolve(arenas, table, parent, *initialization_ident, diagnostics)?;
            let VSymbol::GenVar = &table[symid].content else {
                diagnostics.not_yet_implemented(
                    arenas.get_span(*initialization),
                    "non-genvar used as genvar",
                );
                return Err(());
            };

            let mut value =
                eval_constant_expr_elab(arenas, parent, table, diagnostics, *initialization)?;

            let (mod_or_gen_items, block_ident_ast) = match arenas.get(*block) {
                GenerateBlock::ModuleOrGenerateItem(id) => (AstIdRange::single(*id), None),
                GenerateBlock::BeginEnd(ident, mod_or_gen_items) => (*mod_or_gen_items, *ident),
            };

            loop {
                let symid = match block_ident_ast {
                    Some(block_ident) => try_table_insert(
                        arenas,
                        table,
                        parent,
                        block_ident,
                        VSymbol::GenerateBlock(mod_or_gen_items),
                        diagnostics,
                    )?,
                    None => table.insert_unlinked(
                        IdentTable::EMPTY_IDENT,
                        parent,
                        arenas.get_range_span(mod_or_gen_items),
                        VSymbol::GenerateBlock(mod_or_gen_items),
                    ),
                };

                let genvar_constant = table
                    .insert(
                        initialization_ident.item.0,
                        symid,
                        arenas.get_item_span(*initialization_ident),
                        VSymbol::Parameter(value.clone()),
                    )
                    .expect("No other idents in this block yet");

                let c = eval_constant_expr_elab(arenas, symid, table, diagnostics, *condition)?;
                if !c.to_logical() {
                    table.pop_last_inserted(genvar_constant);
                    table.pop_last_inserted(symid);
                    break;
                }

                let mut error = false;
                for id in mod_or_gen_items.iter() {
                    error |= elaborate_module_or_generate_item(
                        signals,
                        arenas,
                        id,
                        symid,
                        table,
                        module_instances_todo,
                        diagnostics,
                    )
                    .is_err();
                }
                if error {
                    return Err(());
                }

                value = eval_constant_expr_elab(arenas, symid, table, diagnostics, *iteration)?;
            }

            Ok(())
        }
        ModuleOrGenerateItem::IfGenerateConstruct(id) => {
            let IfGenerateConstruct {
                condition,
                truthy,
                falsy,
            } = arenas.get(*id);

            let condition =
                eval_constant_expr_elab(arenas, parent, table, diagnostics, *condition)?;

            let blk = if condition.to_logical() {
                Some(*truthy)
            } else {
                *falsy
            };
            if let Some(blk) = blk {
                elaborate_generate_block(
                    signals,
                    arenas,
                    parent,
                    table,
                    module_instances_todo,
                    diagnostics,
                    blk,
                )?;
            }

            Ok(())
        }
        ModuleOrGenerateItem::CaseGenerateConstruct(id) => {
            let CaseGenerateConstruct { value, items } = arenas.get(*id);
            let value = eval_constant_expr_elab(arenas, parent, table, diagnostics, *value)?;

            for item in items.iter() {
                let CaseGenerateItem { pattern, block } = arenas.get(item);
                let mut is_selected = false;
                match pattern {
                    CaseGeneratePattern::Default => is_selected = true,
                    CaseGeneratePattern::Exprs(exprs) => {
                        for expr in exprs.iter() {
                            let expr_value =
                                eval_constant_expr_elab(arenas, parent, table, diagnostics, expr)?;
                            let expr_value =
                                expr_value.truncate_or_extend(value.ty().force_net_width());
                            if value.clone().logical_equal(expr_value) {
                                is_selected = true;
                            }
                        }
                    }
                };

                if is_selected {
                    elaborate_generate_block(
                        signals,
                        arenas,
                        parent,
                        table,
                        module_instances_todo,
                        diagnostics,
                        *block,
                    )?;
                    break;
                }
            }

            Ok(())
        }
    }
}

pub fn elaborate_module_or_generate_item_declaration<'a>(
    signals: &mut slotmap::SlotMap<SignalKey, Signal>,
    arenas: &'a AstArenas,

    id: AstId<ModuleOrGenerateItemDeclaration>,

    parent: SymbolId,
    table: &mut VSymbolTable,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let mut error = false;
    match arenas.get(id) {
        ModuleOrGenerateItemDeclaration::Net(id) => {
            let NetDeclaration {
                net_type,
                signed,
                range,
                nets,
            } = arenas.get(*id);
            if !matches!(net_type.item, NetType::Wire) {
                diagnostics.not_yet_implemented(
                    arenas.get_item_span(*net_type),
                    "net type not yet supported",
                );
                return Err(());
            }

            let (_, _, width) = match range {
                None => (0, 0, SCALAR_VSIZE),
                Some(range) => eval_constant_range(arenas, parent, table, diagnostics, *range)?,
            };
            let ty = VType::net(width, *signed);
            match nets {
                NetDeclarationNets::Idents(idents) => {
                    for net_ident in idents.iter() {
                        let NetIdent { ident, dimension } = arenas.get(net_ident);
                        let dims =
                            dims_to_array_elab(arenas, parent, table, diagnostics, *dimension)?;

                        let signal = new_signal(signals, arenas, &ty, &dims, *ident);
                        try_table_insert(
                            arenas,
                            table,
                            parent,
                            *ident,
                            VSymbol::Net(NetSymbol {
                                ty,
                                dims,
                                signal,
                                nba: None,
                                port_idx: None,
                            }),
                            diagnostics,
                        )?;
                    }
                }
                NetDeclarationNets::Assignments(assignments) => {
                    for assignment in assignments.iter() {
                        let NetDeclAssignment { ident, expr: _ } = arenas.get(assignment);

                        let signal = new_signal(signals, arenas, &ty, &[], *ident);
                        try_table_insert(
                            arenas,
                            table,
                            parent,
                            *ident,
                            VSymbol::Net(NetSymbol {
                                ty,
                                dims: Vec::new(),
                                signal,
                                nba: None,
                                port_idx: None,
                            }),
                            diagnostics,
                        )?;
                    }
                }
            }
        }
        ModuleOrGenerateItemDeclaration::Reg(id) => {
            let RegDeclaration {
                signed,
                range,
                variable_types,
            } = arenas.get(*id);
            let (_, _, size) = match range {
                None => (0, 0, SCALAR_VSIZE),
                Some(range) => eval_constant_range(arenas, parent, table, diagnostics, *range)?,
            };

            let ty = VType::net(size, *signed);
            for variable_type in variable_types.iter() {
                error |= elaborate_variable_type(
                    signals,
                    arenas,
                    parent,
                    table,
                    diagnostics,
                    variable_type,
                    ty,
                )
                .is_err();
            }
        }
        ModuleOrGenerateItemDeclaration::Integer(id) => {
            let IntegerDeclaration { variable_types } = arenas.get(*id);
            let ty = VType::SignedNet(INTEGER_VSIZE);
            for variable_type in variable_types.iter() {
                error |= elaborate_variable_type(
                    signals,
                    arenas,
                    parent,
                    table,
                    diagnostics,
                    variable_type,
                    ty,
                )
                .is_err();
            }
        }
        ModuleOrGenerateItemDeclaration::Genvar(id) => {
            let GenvarDeclaration { identifiers } = arenas.get(*id);
            let mut error = false;
            for ast_ident in identifiers.iter() {
                let ast_ident = arenas.to_item(ast_ident);

                error |= try_table_insert(
                    arenas,
                    table,
                    parent,
                    ast_ident,
                    VSymbol::GenVar,
                    diagnostics,
                )
                .is_err();
            }
            if error {
                return Err(());
            }
        }
        ModuleOrGenerateItemDeclaration::Task(id) => {
            let TaskDeclaration {
                ident, automatic, ..
            } = arenas.get(*id);

            let symbol = try_table_insert(
                arenas,
                table,
                parent,
                *ident,
                VSymbol::Task(TaskSymbol {
                    ast_id: *id,
                    lowered: None,
                }),
                diagnostics,
            )?;
            function::elaborate_task(signals, arenas, symbol, table, diagnostics)?;
        }
        ModuleOrGenerateItemDeclaration::Function(id) => {
            let FunctionDeclaration {
                ident, automatic, ..
            } = arenas.get(*id);

            let symbol = try_table_insert(
                arenas,
                table,
                parent,
                *ident,
                VSymbol::Function(FunctionSymbol {
                    ast_id: *id,
                    lowered: None,
                }),
                diagnostics,
            )?;
            function::elaborate_fn(signals, arenas, symbol, table, diagnostics)?;
        }
    }

    if error { Err(()) } else { Ok(()) }
}

pub fn elaborate_variable_type<'a>(
    signals: &mut slotmap::SlotMap<SignalKey, Signal>,
    arenas: &'a AstArenas,
    parent: SymbolId,
    table: &mut VSymbolTable,
    diagnostics: &mut Diagnostics,
    variable_type: AstId<VariableType>,
    ty: VType,
) -> Result<(), ()> {
    let VariableType {
        identifier,
        variant,
    } = arenas.get(variable_type);

    let dims = match variant {
        VariableTypeVariant::Dimensions(dimensions) => {
            dims_to_array_elab(arenas, parent, table, diagnostics, *dimensions)?
        }
        VariableTypeVariant::ConstantExpr(_) => Vec::new(),
    };
    let signal = new_signal(signals, arenas, &ty, &dims, *identifier);

    try_table_insert(
        arenas,
        table,
        parent,
        *identifier,
        VSymbol::Net(NetSymbol {
            ty,
            dims,
            signal,
            nba: None,
            port_idx: None,
        }),
        diagnostics,
    )?;

    Ok(())
}

fn new_signal(
    signals: &mut slotmap::SlotMap<SignalKey, Signal>,
    arenas: &AstArenas,
    ty: &VType,
    dims: &[u32],
    name: AstItem<Identifier>,
) -> SignalKey {
    let mut size = ty.force_net_width();
    for dim in dims {
        size = size.checked_mul(NonZeroU32::new(*dim).unwrap()).unwrap();
    }
    let origin = arenas.get_item_span(name);
    let name = arenas.ident_table[name.item.0].to_string();
    signals.insert(Signal {
        name,
        size,
        initialize: None,
        origin,
    })
}

pub fn elaborate_statements<'a>(
    signals: &mut slotmap::SlotMap<SignalKey, Signal>,
    arenas: &'a AstArenas,
    parent: SymbolId,
    table: &mut VSymbolTable,
    diagnostics: &mut Diagnostics,
    stmts: AstIdRange<Statement>,
) -> Result<(), ()> {
    use StatementContent as S;
    let mut error = false;
    for stmt in stmts.iter() {
        let Statement {
            attr_instances: _,
            content,
        } = arenas.get(stmt);
        match content {
            S::CaseStatement(id) => {
                let CaseStatement {
                    variant: _,
                    expr: _,
                    items,
                } = arenas.get(*id);
                for item in items.iter() {
                    let CaseItem {
                        pattern: _,
                        statement_or_null,
                    } = arenas.get(item);
                    error |= elaborate_statement_or_null(
                        signals,
                        arenas,
                        parent,
                        table,
                        diagnostics,
                        *statement_or_null,
                    )
                    .is_err();
                }
            }
            S::ConditionalStatement(id) => {
                let ConditionalStatement {
                    if_branch,
                    else_ifs,
                    else_branch,
                } = arenas.get(*id);
                let IfBranch {
                    condition: _,
                    statement,
                } = if_branch;
                error |= elaborate_statement_or_null(
                    signals,
                    arenas,
                    parent,
                    table,
                    diagnostics,
                    *statement,
                )
                .is_err();
                for else_if in else_ifs.iter() {
                    let IfBranch {
                        condition: _,
                        statement,
                    } = arenas.get(else_if);
                    error |= elaborate_statement_or_null(
                        signals,
                        arenas,
                        parent,
                        table,
                        diagnostics,
                        *statement,
                    )
                    .is_err();
                }
                if let Some(statement) = else_branch {
                    error |= elaborate_statement_or_null(
                        signals,
                        arenas,
                        parent,
                        table,
                        diagnostics,
                        *statement,
                    )
                    .is_err();
                }
            }
            S::LoopStatement(id) => {
                let LoopStatement {
                    variant: _,
                    statement,
                } = arenas.get(*id);
                error |= elaborate_statements(
                    signals,
                    arenas,
                    parent,
                    table,
                    diagnostics,
                    AstIdRange::single(*statement),
                )
                .is_err();
            }
            S::DisableStatement => todo!(),
            S::EventTrigger => todo!(),
            S::ParBlock => todo!(),
            S::ProceduralContinuousAssignments => todo!(),
            S::ProceduralTimingControlStatement(id) => {
                let ProceduralTimingControlStatement {
                    procedural_timing_control: _,
                    statement_or_null,
                } = arenas.get(*id);
                error |= elaborate_statement_or_null(
                    signals,
                    arenas,
                    parent,
                    table,
                    diagnostics,
                    *statement_or_null,
                )
                .is_err();
            }
            S::SeqBlock(id) => {
                let SeqBlock { block, statements } = arenas.get(*id);
                match block {
                    Some(block) => {
                        let Block {
                            block_identifier,
                            block_item_decls,
                        } = arenas.get(*block);

                        let Ok(named_block_symid) = try_table_insert(
                            arenas,
                            table,
                            parent,
                            *block_identifier,
                            VSymbol::NamedBlock,
                            diagnostics,
                        ) else {
                            error = true;
                            continue;
                        };

                        for item_decl in block_item_decls.iter() {
                            use BlockItemDeclaration as B;
                            match arenas.get(item_decl) {
                                B::Reg {
                                    signed,
                                    range,
                                    identifiers,
                                } => {
                                    let (_, _, size) = match range {
                                        None => (0, 0, SCALAR_VSIZE),
                                        Some(range) => eval_constant_range(
                                            arenas,
                                            parent,
                                            table,
                                            diagnostics,
                                            *range,
                                        )?,
                                    };

                                    let ty = VType::net(size, *signed);
                                    for variable_type in identifiers.iter() {
                                        error |= elaborate_variable_type(
                                            signals,
                                            arenas,
                                            parent,
                                            table,
                                            diagnostics,
                                            variable_type,
                                            ty,
                                        )
                                        .is_err();
                                    }
                                }
                                B::Integer(var_types) => {
                                    let ty = VType::SignedNet(INTEGER_VSIZE);
                                    for variable_type in var_types.iter() {
                                        error |= elaborate_variable_type(
                                            signals,
                                            arenas,
                                            parent,
                                            table,
                                            diagnostics,
                                            variable_type,
                                            ty,
                                        )
                                        .is_err();
                                    }
                                }
                                B::Time | B::Real | B::Realtime | B::Event => todo!(),
                                B::LocalParameterDeclaration(ast_id) => {
                                    let LocalParameterDeclaration {
                                        typing,
                                        assignments,
                                    } = arenas.get(*ast_id);
                                    elaborate_parameter_declaration(
                                        arenas,
                                        *typing,
                                        *assignments,
                                        parent,
                                        table,
                                        diagnostics,
                                        None,
                                    )?;
                                }
                                B::ParameterDeclaration(ast_id) => {
                                    let ParameterDeclaration {
                                        typing,
                                        assignments,
                                    } = arenas.get(*ast_id);
                                    elaborate_parameter_declaration(
                                        arenas,
                                        *typing,
                                        *assignments,
                                        parent,
                                        table,
                                        diagnostics,
                                        None,
                                    )?;
                                }
                            }
                        }

                        error |= elaborate_statements(
                            signals,
                            arenas,
                            named_block_symid,
                            table,
                            diagnostics,
                            *statements,
                        )
                        .is_err();
                    }
                    None => {
                        error |= elaborate_statements(
                            signals,
                            arenas,
                            parent,
                            table,
                            diagnostics,
                            *statements,
                        )
                        .is_err();
                    }
                }
            }
            S::WaitStatement(id) => {
                let WaitStatement {
                    expression: _,
                    statement_or_null,
                } = arenas.get(*id);
                error |= elaborate_statement_or_null(
                    signals,
                    arenas,
                    parent,
                    table,
                    diagnostics,
                    *statement_or_null,
                )
                .is_err();
            }
            S::BlockingAssignment(_)
            | S::NonBlockingAssignment(_)
            | S::SystemTaskEnable(_)
            | S::TaskEnable(_) => {}
        }
    }
    if error { Err(()) } else { Ok(()) }
}

pub fn elaborate_statement_or_null<'a>(
    signals: &mut slotmap::SlotMap<SignalKey, Signal>,
    arenas: &'a AstArenas,
    parent: SymbolId,
    table: &mut VSymbolTable,
    diagnostics: &mut Diagnostics,
    stmt: AstId<StatementOrNull>,
) -> Result<(), ()> {
    match arenas.get(stmt) {
        StatementOrNull::Attribute(_) => Ok(()),
        StatementOrNull::Statement(id) => elaborate_statements(
            signals,
            arenas,
            parent,
            table,
            diagnostics,
            AstIdRange::single(*id),
        ),
    }
}

pub fn eval_constant_expr_elab<'a>(
    arenas: &'a AstArenas,
    scope: SymbolId,
    table: &VSymbolTable,
    diagnostics: &mut Diagnostics,
    expr: AstId<ConstantExpr>,
) -> Result<VValue, ()> {
    eval_constant_expr(arenas, EvalScope { table, key: scope }, diagnostics, expr)
}

pub fn eval_constant_range(
    arenas: &AstArenas,
    scope: SymbolId,
    table: &VSymbolTable,
    diagnostics: &mut Diagnostics,
    ast_range: AstId<Range>,
) -> Result<(i64, i64, VectorSize), ()> {
    let range = arenas.get(ast_range);
    let msb = eval_constant_expr_elab(arenas, scope, table, diagnostics, range.msb);
    let lsb = eval_constant_expr_elab(arenas, scope, table, diagnostics, range.lsb);

    let (Ok(VValue::SignedNet(msb)), Ok(VValue::SignedNet(lsb))) = (msb, lsb) else {
        return Err(());
    };
    let msb = msb.as_i64().unwrap();
    let lsb = lsb.as_i64().unwrap();
    let width = u32::try_from(msb.abs_diff(lsb)).ok();
    let width = width.and_then(|w| w.checked_add(1));
    let width = width.and_then(|w| VectorSize::new(w));
    let Some(width) = width else {
        let tr = arenas.get_span(range.msb) | arenas.get_span(range.lsb);
        diagnostics.net_width_overflow(tr);
        return Err(());
    };
    Ok((msb, lsb, width))
}

pub fn elaborate_generate_block<'a>(
    signals: &mut slotmap::SlotMap<SignalKey, Signal>,
    arenas: &'a AstArenas,
    parent: SymbolId,
    table: &mut VSymbolTable,
    module_instances_todo: &mut Vec<SymbolId>,
    diagnostics: &mut Diagnostics,
    blk: AstId<Option<GenerateBlock>>,
) -> Result<(), ()> {
    let Some(blk) = arenas.get(blk) else {
        return Ok(());
    };

    let (mod_or_gen_items, block_ident_ast) = match blk {
        GenerateBlock::ModuleOrGenerateItem(id) => (AstIdRange::single(*id), None),
        GenerateBlock::BeginEnd(ident, mod_or_gen_items) => (*mod_or_gen_items, *ident),
    };

    let symid = match block_ident_ast {
        None => table.insert_unlinked(
            IdentTable::EMPTY_IDENT,
            parent,
            arenas.get_range_span(mod_or_gen_items),
            VSymbol::GenerateBlock(mod_or_gen_items),
        ),
        Some(block_ident) => try_table_insert(
            arenas,
            table,
            parent,
            block_ident,
            VSymbol::GenerateBlock(mod_or_gen_items),
            diagnostics,
        )?,
    };

    let mut error = false;
    for id in mod_or_gen_items.iter() {
        error |= elaborate_module_or_generate_item(
            signals,
            arenas,
            id,
            symid,
            table,
            module_instances_todo,
            diagnostics,
        )
        .is_err();
    }

    if error { Err(()) } else { Ok(()) }
}

pub fn dims_to_array_elab<'a>(
    arenas: &'a AstArenas,
    scope: SymbolId,
    table: &VSymbolTable,
    diagnostics: &mut Diagnostics,
    dimensions: AstIdRange<Dimension>,
) -> Result<Vec<u32>, ()> {
    let mut dims = Vec::with_capacity(dimensions.len());
    for dim in dimensions.iter().rev() {
        let Dimension { lhs, rhs } = arenas.get(dim);
        let lhs = eval_constant_expr_elab(arenas, scope, table, diagnostics, *lhs);
        let rhs = eval_constant_expr_elab(arenas, scope, table, diagnostics, *rhs);

        let lhs = lhs?.into_bits().as_i64().unwrap();
        let rhs = rhs?.into_bits().as_i64().unwrap();

        dims.push((lhs.abs_diff(rhs) + 1) as u32);
    }
    Ok(dims)
}

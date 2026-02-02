use std::collections::HashMap;
use std::sync::Arc;

use vogls_frontend::VgHashMap;
use vogls_frontend::ident_table::{IdentId, IdentTable};
use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{ConnectionDirection, INTEGER_VSIZE, SCALAR_VSIZE, VectorSize};

use crate::ast::constant_expr::{ConstantExpr, ConstantMinTypMaxExpression};
use crate::ast::module::{
    CaseGenerateConstruct, CaseGenerateItem, CaseGeneratePattern, Dimension, FunctionDeclaration,
    GenerateBlock, GenvarAssignment, GenvarDeclaration, IfGenerateConstruct, IntegerDeclaration,
    LocalParameterDeclaration, LoopGenerateConstruct, Module, ModuleInstance, ModuleInstantiation,
    ModuleItem, ModuleOrGenerateItem, ModuleOrGenerateItemDeclaration, ModulePorts,
    NamedParameterAssignment, NetDeclAssignment, NetDeclaration, NetDeclarationNets, NetIdent,
    NetType, NonPortModuleItem, ParamAssignment, ParameterDeclaration, ParameterDeclarationTyping,
    ParameterValueAssignment, Port, PortDeclaration, PortExpression, PortReference, Range,
    RegDeclaration, TaskDeclaration, VariableType, VariableTypeVariant,
};
use crate::ast::statement::{
    Block, CaseItem, CaseStatement, ConditionalStatement, IfBranch, LoopStatement,
    ProceduralTimingControlStatement, SeqBlock, Statement, StatementContent, StatementOrNull,
    WaitStatement,
};
use crate::ast::{AstId, AstIdRange, AstItem, Identifier};
use crate::hierarchy::{
    HierarchyGenerateBlock, HierarchyItemRange, HierarchyModule, ParameterOverrides,
};
use crate::lower::expression::eval_constant_expr_f;
use crate::lower::{Diagnostics, VType, VValue};
use crate::parser::AstArenas;

pub mod function;

pub type ElabTable = vogls_frontend::symbol_table::SymbolTable<ElabSymbol>;

pub enum ElabSymbol {
    Module(ElabModule),
    Parameter(VValue),
    Net(VType, Vec<u32>),
    Reg(VType, Vec<u32>),
    NamedBlock,
    GenerateBlock,
    GenVar,
    Task(AstId<TaskDeclaration>),
    Function(AstId<FunctionDeclaration>),
}

pub struct ElabModule {
    pub module: IdentId,

    pub ports: Vec<SymbolId>,
    pub parameters: Vec<SymbolId>,

    pub parameter_overrides: Arc<VgHashMap<IdentId, usize>>,
    pub parameter_override_values: Arc<Vec<VValue>>,
}

pub fn try_table_insert(
    arenas: &AstArenas,
    table: &mut ElabTable,
    parent: SymbolId,
    name: AstItem<Identifier>,
    content: ElabSymbol,
    diagnostics: &mut Diagnostics,
) -> Result<SymbolId, ()> {
    let Ok(symid) = table.insert(name.item.0, parent, content) else {
        diagnostics.duplicate_definition(arenas, name);
        return Err(());
    };

    Ok(symid)
}
pub fn table_recursive_resolve(
    table: &ElabTable,
    parent: SymbolId,
    name: IdentId,
) -> Option<SymbolId> {
    // @TODO: Actually recursively resolve
    table.resolve(parent, name)
}
pub fn try_table_resolve(
    arenas: &AstArenas,
    table: &ElabTable,
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
    arenas: &'a AstArenas,
    module: AstId<Module>,
    module_symid: SymbolId,
    table: &mut ElabTable,
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
    let ElabSymbol::Module(elab_module) = &symbol.content else {
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

                        if table
                            .insert(
                                identifier.item.0,
                                module_symid,
                                ElabSymbol::Net(VType::SCALAR_NET, Vec::new()),
                            )
                            .is_err()
                        {
                            diagnostics.duplicate_definition(arenas, *identifier);
                            error = true;
                        }
                    }
                }
            }
        }
        ModulePorts::PortDeclarations(port_declarations) => {
            for id in port_declarations.iter() {
                error |= elaborate_port_declaration(arenas, id, module_symid, table, diagnostics)
                    .is_err();
            }
        }
    }

    for item in module_items.iter() {
        match arenas.get(item) {
            ModuleItem::PortDeclaration(id) => {
                error |= elaborate_port_declaration(arenas, *id, module_symid, table, diagnostics)
                    .is_err();
            }
            ModuleItem::NonPortModuleItem(id) => match arenas.get(*id) {
                NonPortModuleItem::ModuleOrGenerateItem(id) => {
                    error |= elaborate_module_or_generate_item(
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
    table: &mut ElabTable,
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
            let ElabSymbol::Module(module) = &mut symbol.content else {
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

        let Ok(param_symid) = table.insert(name, parent, ElabSymbol::Parameter(value)) else {
            diagnostics.duplicate_definition(arenas, *param);
            return Err(());
        };

        if param_override_is_used.is_some() {
            let symbol = &mut table[parent];
            let ElabSymbol::Module(module) = &mut symbol.content else {
                unreachable!("non-local parameter can only be defined at module-level");
            };
            module.parameters.push(param_symid);
        }
    }

    Ok(())
}

pub fn elaborate_port_declaration<'a>(
    arenas: &'a AstArenas,

    id: AstId<PortDeclaration>,

    parent: SymbolId,
    table: &mut ElabTable,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
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

    let mut error = false;
    for ident in identifiers.iter() {
        let origin = arenas.get_span(ident);
        let Ok(symid) = table.insert(arenas.get(ident).0, parent, ElabSymbol::Net(ty, Vec::new()))
        else {
            diagnostics.duplicate_definition(arenas, arenas.to_item(ident));
            error = true;
            continue;
        };

        let symbol = &mut table[parent];
        let ElabSymbol::Module(module) = &mut symbol.content else {
            unreachable!("non-local parameter can only be defined at module-level");
        };

        module.ports.push(symid);
    }

    if error {
        return Err(());
    }

    Ok(())
}

pub fn elaborate_module_or_generate_item<'a>(
    arenas: &'a AstArenas,

    id: AstId<ModuleOrGenerateItem>,

    parent: SymbolId,
    table: &mut ElabTable,
    module_instances_todo: &mut Vec<SymbolId>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    match arenas.get(id) {
        ModuleOrGenerateItem::ModuleOrGenerateItemDeclaration(id) => {
            elaborate_module_or_generate_item_declaration(arenas, *id, parent, table, diagnostics)
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
                    ElabSymbol::Module(ElabModule {
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
            arenas,
            parent,
            table,
            diagnostics,
            AstIdRange::single(arenas.get(*id).0),
        ),
        ModuleOrGenerateItem::AlwaysConstruct(id) => elaborate_statements(
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
            let ElabSymbol::GenVar = &table[symid].content else {
                diagnostics.not_yet_implemented(
                    arenas.get_span(*initialization),
                    "non-genvar used as genvar",
                );
                return Err(());
            };

            let mut value =
                eval_constant_expr_elab(arenas, parent, table, diagnostics, *initialization)?;

            // @GJB: Bit of a hack to allow the genvar to be used in the constant expression
            // evaluation.
            macro_rules! eval_constant_expr_with_genvar {
                ($expr:expr) => {{
                    eval_constant_expr_f(
                        arenas,
                        |ident| {
                            if ident == initialization_ident.item.0 {
                                return Some(value.clone());
                            }
                            let symid = table_recursive_resolve(table, parent, ident)?;
                            match &table[symid].content {
                                ElabSymbol::Parameter(value) => Some(value.clone()),
                                _ => None,
                            }
                        },
                        diagnostics,
                        $expr,
                    )
                }};
            }

            loop {
                let c = eval_constant_expr_with_genvar!(*condition)?;
                if !c.to_logical() {
                    break;
                }

                let (mod_or_gen_items, block_ident_ast) = match arenas.get(*block) {
                    GenerateBlock::ModuleOrGenerateItem(id) => (AstIdRange::single(*id), None),
                    GenerateBlock::BeginEnd(ident, mod_or_gen_items) => (*mod_or_gen_items, *ident),
                };

                let symid = match block_ident_ast {
                    Some(block_ident) => try_table_insert(
                        arenas,
                        table,
                        parent,
                        block_ident,
                        ElabSymbol::GenerateBlock,
                        diagnostics,
                    )?,
                    None => table.insert_unlinked(
                        IdentTable::EMPTY_IDENT,
                        parent,
                        ElabSymbol::GenerateBlock,
                    ),
                };

                assert!(
                    table
                        .insert(
                            initialization_ident.item.0,
                            symid,
                            ElabSymbol::Parameter(value.clone())
                        )
                        .is_ok()
                );

                let mut error = false;
                for id in mod_or_gen_items.iter() {
                    error |= elaborate_module_or_generate_item(
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

                value = eval_constant_expr_with_genvar!(*iteration)?;
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
            if condition.to_logical() {
                elaborate_generate_block(
                    arenas,
                    parent,
                    table,
                    module_instances_todo,
                    diagnostics,
                    *truthy,
                )?;
            } else if let Some(falsy) = falsy {
                elaborate_generate_block(
                    arenas,
                    parent,
                    table,
                    module_instances_todo,
                    diagnostics,
                    *falsy,
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
    arenas: &'a AstArenas,

    id: AstId<ModuleOrGenerateItemDeclaration>,

    parent: SymbolId,
    table: &mut ElabTable,
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

                        try_table_insert(
                            arenas,
                            table,
                            parent,
                            *ident,
                            ElabSymbol::Net(ty, dims),
                            diagnostics,
                        )?;
                    }
                }
                NetDeclarationNets::Assignments(assignments) => {
                    for assignment in assignments.iter() {
                        let NetDeclAssignment { ident, expr: _ } = arenas.get(assignment);
                        let origin = arenas.get_item_span(*ident);

                        try_table_insert(
                            arenas,
                            table,
                            parent,
                            *ident,
                            ElabSymbol::Net(ty, Vec::new()),
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
                error |=
                    elaborate_variable_type(arenas, parent, table, diagnostics, variable_type, ty)
                        .is_err();
            }
        }
        ModuleOrGenerateItemDeclaration::Integer(id) => {
            let IntegerDeclaration { variable_types } = arenas.get(*id);
            let ty = VType::SignedNet(INTEGER_VSIZE);
            for variable_type in variable_types.iter() {
                error |=
                    elaborate_variable_type(arenas, parent, table, diagnostics, variable_type, ty)
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
                    ElabSymbol::GenVar,
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

            error |= try_table_insert(
                arenas,
                table,
                parent,
                *ident,
                ElabSymbol::Task(*id),
                diagnostics,
            )
            .is_err();
        }
        ModuleOrGenerateItemDeclaration::Function(id) => {
            let FunctionDeclaration {
                ident, automatic, ..
            } = arenas.get(*id);

            error |= try_table_insert(
                arenas,
                table,
                parent,
                *ident,
                ElabSymbol::Function(*id),
                diagnostics,
            )
            .is_err();
        }
    }

    if error { Err(()) } else { Ok(()) }
}

pub fn elaborate_variable_type<'a>(
    arenas: &'a AstArenas,
    parent: SymbolId,
    table: &mut ElabTable,
    diagnostics: &mut Diagnostics,
    variable_type: AstId<VariableType>,
    ty: VType,
) -> Result<(), ()> {
    let VariableType {
        identifier,
        variant,
    } = arenas.get(variable_type);

    let origin = arenas.get_span(variable_type);
    let dims = match variant {
        VariableTypeVariant::Dimensions(dimensions) => {
            dims_to_array_elab(arenas, parent, table, diagnostics, *dimensions)?
        }
        VariableTypeVariant::ConstantExpr(_) => Vec::new(),
    };

    try_table_insert(
        arenas,
        table,
        parent,
        *identifier,
        ElabSymbol::Reg(ty, dims),
        diagnostics,
    )?;

    Ok(())
}

pub fn elaborate_statements<'a>(
    arenas: &'a AstArenas,
    parent: SymbolId,
    table: &mut ElabTable,
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
                error |=
                    elaborate_statement_or_null(arenas, parent, table, diagnostics, *statement)
                        .is_err();
                for else_if in else_ifs.iter() {
                    let IfBranch {
                        condition: _,
                        statement,
                    } = arenas.get(else_if);
                    error |=
                        elaborate_statement_or_null(arenas, parent, table, diagnostics, *statement)
                            .is_err();
                }
                if let Some(statement) = else_branch {
                    error |=
                        elaborate_statement_or_null(arenas, parent, table, diagnostics, *statement)
                            .is_err();
                }
            }
            S::LoopStatement(id) => {
                let LoopStatement {
                    variant: _,
                    statement,
                } = arenas.get(*id);
                error |= elaborate_statements(
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
                            block_item_decls: _,
                        } = arenas.get(*block);

                        let Ok(named_block_symid) =
                            table.insert(block_identifier.item.0, parent, ElabSymbol::NamedBlock)
                        else {
                            diagnostics.duplicate_definition(arenas, *block_identifier);
                            error = true;
                            continue;
                        };

                        error |= elaborate_statements(
                            arenas,
                            named_block_symid,
                            table,
                            diagnostics,
                            *statements,
                        )
                        .is_err();
                    }
                    None => {
                        error |=
                            elaborate_statements(arenas, parent, table, diagnostics, *statements)
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
    arenas: &'a AstArenas,
    parent: SymbolId,
    table: &mut ElabTable,
    diagnostics: &mut Diagnostics,
    stmt: AstId<StatementOrNull>,
) -> Result<(), ()> {
    match arenas.get(stmt) {
        StatementOrNull::Attribute(_) => Ok(()),
        StatementOrNull::Statement(id) => {
            elaborate_statements(arenas, parent, table, diagnostics, AstIdRange::single(*id))
        }
    }
}

pub fn eval_constant_expr_elab<'a>(
    arenas: &'a AstArenas,
    scope: SymbolId,
    table: &ElabTable,
    diagnostics: &mut Diagnostics,
    expr: AstId<ConstantExpr>,
) -> Result<VValue, ()> {
    eval_constant_expr_f(
        arenas,
        |ident| {
            let symid = table_recursive_resolve(table, scope, ident)?;
            match &table[symid].content {
                ElabSymbol::Parameter(value) => Some(value.clone()),
                _ => None,
            }
        },
        diagnostics,
        expr,
    )
}

pub fn eval_constant_range(
    arenas: &AstArenas,
    scope: SymbolId,
    table: &ElabTable,
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
    arenas: &'a AstArenas,
    parent: SymbolId,
    table: &mut ElabTable,
    module_instances_todo: &mut Vec<SymbolId>,
    diagnostics: &mut Diagnostics,
    blk: AstId<Option<GenerateBlock>>,
) -> Result<(), ()> {
    let (mod_or_gen_items, block_ident_ast) = match arenas.get(blk) {
        None => (AstIdRange::default(), None),
        Some(GenerateBlock::ModuleOrGenerateItem(id)) => (AstIdRange::single(*id), None),
        Some(GenerateBlock::BeginEnd(ident, mod_or_gen_items)) => (*mod_or_gen_items, *ident),
    };

    let symid = match block_ident_ast {
        None => table.insert_unlinked(IdentTable::EMPTY_IDENT, parent, ElabSymbol::GenerateBlock),
        Some(block_ident) => try_table_insert(
            arenas,
            table,
            parent,
            block_ident,
            ElabSymbol::GenerateBlock,
            diagnostics,
        )?,
    };

    let mut error = false;
    for id in mod_or_gen_items.iter() {
        error |= elaborate_module_or_generate_item(
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
    table: &ElabTable,
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

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use slotmap::SlotMap;
use vogls_ir::{ConnectionDirection, INTEGER_VSIZE, SCALAR_VSIZE, Signal, SignalKey, VectorSize};

use crate::ast::constant_expr::ConstantMinTypMaxExpression;
use crate::ast::module::{
    CaseGenerateConstruct, CaseGenerateItem, CaseGeneratePattern, FunctionDeclaration,
    GenerateBlock, GenvarAssignment, GenvarDeclaration, IfGenerateConstruct, IntegerDeclaration,
    LocalParameterDeclaration, LoopGenerateConstruct, Module, ModuleInstance, ModuleInstantiation,
    ModuleItem, ModuleOrGenerateItem, ModuleOrGenerateItemDeclaration, ModulePorts,
    NamedParameterAssignment, NetDeclAssignment, NetDeclaration, NetDeclarationNets, NetIdent,
    NetType, NonPortModuleItem, ParamAssignment, ParameterDeclaration, ParameterDeclarationTyping,
    ParameterValueAssignment, Port, PortDeclaration, PortExpression, PortReference, RegDeclaration,
    TaskDeclaration, VariableType, VariableTypeVariant,
};
use crate::ast::statement::{
    Block, CaseItem, CaseStatement, ConditionalStatement, IfBranch, LoopStatement,
    ProceduralTimingControlStatement, SeqBlock, Statement, StatementContent, StatementOrNull,
    WaitStatement,
};
use crate::ast::{AstId, AstIdRange};
use crate::hierarchy::{
    HierarchyFunction, HierarchyGenerateBlock, HierarchyItem, HierarchyItemRange, HierarchyModule,
    HierarchyNamedBlock, HierarchyNet, HierarchyParameter, HierarchyTask, ParameterOverrides,
    ScopeBuilder,
};
use crate::lower::{Diagnostics, VType, dims_to_array, eval_constant_expr, evaluate_range};
use crate::parser::AstArenas;

pub mod function;

pub fn elaborate_module<'a>(
    signals: &mut SlotMap<SignalKey, Signal>,
    arenas: &'a AstArenas,

    module: AstId<Module>,

    builder: &mut ScopeBuilder<'a>,
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

    let mut parameter_idx = 0;
    let mut gen_vars = HashMap::<String, bool>::new();

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
                builder,
                diagnostics,
                Some(&mut parameter_idx),
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

                        let name = arenas.ident_to_str(identifier.item.0);

                        // Insert the port as IO.
                        let HierarchyItem::Module(m) =
                            builder.hierarchy.items()[builder.key().as_idx()]
                        else {
                            unreachable!()
                        };
                        let m = &mut builder.hierarchy.modules[m];
                        m.lut.insert(name.to_string(), m.ports.len());
                        m.ports.push((usize::MAX, ConnectionDirection::Both));
                    }
                }
            }
        }
        ModulePorts::PortDeclarations(port_declarations) => {
            for id in port_declarations.iter() {
                error |=
                    elaborate_port_declaration(signals, arenas, id, builder, diagnostics).is_err();
            }
        }
    }

    for item in module_items.iter() {
        match arenas.get(item) {
            ModuleItem::PortDeclaration(id) => {
                error |=
                    elaborate_port_declaration(signals, arenas, *id, builder, diagnostics).is_err();
            }
            ModuleItem::NonPortModuleItem(id) => match arenas.get(*id) {
                NonPortModuleItem::ModuleOrGenerateItem(id) => {
                    error |= elaborate_module_or_generate_item(
                        signals,
                        arenas,
                        *id,
                        builder,
                        diagnostics,
                        &mut gen_vars,
                    )
                    .is_err();
                }
                NonPortModuleItem::GenerateRegion(region) => {
                    for id in region.module_or_generate_item.iter() {
                        error |= elaborate_module_or_generate_item(
                            signals,
                            arenas,
                            id,
                            builder,
                            diagnostics,
                            &mut gen_vars,
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
                        builder,
                        diagnostics,
                        Some(&mut parameter_idx),
                    )?;
                }
                NonPortModuleItem::SpecParamDeclaration => todo!(),
            },
        }
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

    builder: &mut ScopeBuilder<'a>,
    diagnostics: &mut Diagnostics,
    mut parameter_idx: Option<&mut usize>,
) -> Result<(), ()> {
    let _ty = parameter_typing_to_type(arenas, builder, diagnostics, typing)?;
    for assignment in assignments.iter() {
        let ParamAssignment { param, constant } = arenas.get(assignment);
        let name = arenas.ident_to_str(param.item.0);
        let mut value = match arenas.get(*constant) {
            ConstantMinTypMaxExpression::Single(id) => {
                eval_constant_expr(arenas, builder.eval_scope(), diagnostics, *id)?
            }
            ConstantMinTypMaxExpression::MinTypMax { .. } => todo!(),
        };

        if let Some(parameter_idx) = parameter_idx.as_deref_mut() {
            // Insert the parameter as a module parameter.
            let HierarchyItem::Module(m) = builder.hierarchy.items()[builder.key().as_idx()] else {
                unreachable!()
            };
            let idx = builder.hierarchy.parameters().len();
            let m = &mut builder.hierarchy.modules[m];
            m.parameter_lut.insert(name.to_string(), m.parameters.len());
            m.parameters.push(idx);

            if let Some(overrides) = m.parameter_overrides.as_ref() {
                match overrides {
                    ParameterOverrides::Ordered(values) => value = values[*parameter_idx].clone(),
                    ParameterOverrides::Named(map) => {
                        if let Some(override_value) = map.get(name) {
                            value = override_value.clone();
                        }
                    }
                }
            }

            *parameter_idx += 1;
        }

        if builder
            .insert_parameter(HierarchyParameter {
                name: name.to_string(),
                parent: builder.key(),
                value,
            })
            .is_some()
        {
            diagnostics.duplicate_definition(arenas, *param);
            return Err(());
        }
    }

    Ok(())
}

pub fn elaborate_port_declaration<'a>(
    signals: &mut SlotMap<SignalKey, Signal>,
    arenas: &'a AstArenas,

    id: AstId<PortDeclaration>,

    builder: &mut ScopeBuilder<'a>,
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
        Some(range) => evaluate_range(arenas, builder.eval_scope(), diagnostics, range)?,
    };
    let ty = VType::net(size, signed);

    let mut error = false;
    for ident in identifiers.iter() {
        let name = arenas.ident_to_str(arenas.get(ident).0);
        let origin = arenas.get_span(ident);
        let signal = signals.insert(Signal {
            name: name.to_string(),
            size: ty.force_net_width(),
            initialize: None,
            origin,
        });
        let net = HierarchyNet {
            name: name.to_string(),
            parent: builder.key(),
            signal,
            ty,
            dims: [].into(),
            nba: None,
        };
        let port_key = builder.hierarchy.net().len();
        if builder.insert_net(net).is_some() {
            diagnostics.duplicate_definition(arenas, arenas.to_item(ident));
            error = true;
            continue;
        }

        // Insert the port as IO.
        let HierarchyItem::Module(m) = builder.hierarchy.items()[builder.key().as_idx()] else {
            unreachable!()
        };
        let m = &mut builder.hierarchy.modules[m];
        match m.lut.entry(name.to_string()) {
            Entry::Vacant(v) => {
                m.ports.push((port_key, direction));
                v.insert(m.ports.len() - 1);
            }
            Entry::Occupied(idx) => m.ports[*idx.get()] = (port_key, direction),
        }
    }

    if error {
        return Err(());
    }

    Ok(())
}

pub fn elaborate_module_or_generate_item<'a>(
    signals: &mut SlotMap<SignalKey, Signal>,
    arenas: &'a AstArenas,

    id: AstId<ModuleOrGenerateItem>,

    builder: &mut ScopeBuilder<'a>,
    diagnostics: &mut Diagnostics,
    genvars: &mut HashMap<String, bool>,
) -> Result<(), ()> {
    match arenas.get(id) {
        ModuleOrGenerateItem::ModuleOrGenerateItemDeclaration(id) => {
            elaborate_module_or_generate_item_declaration(
                signals,
                arenas,
                *id,
                builder,
                diagnostics,
                genvars,
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
                builder,
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

            let parameter_overrides = match parameter_value_assignment {
                None => None,
                Some(id) => Some(match arenas.get(*id) {
                    ParameterValueAssignment::Ordered(ids) => {
                        let mut params = Vec::new();
                        for id in ids.iter() {
                            let value =
                                eval_constant_expr(arenas, builder.eval_scope(), diagnostics, id)?;
                            params.push(value);
                        }
                        ParameterOverrides::Ordered(params)
                    }
                    ParameterValueAssignment::Named(named) => {
                        let mut params = HashMap::new();
                        for n in named.iter() {
                            let NamedParameterAssignment {
                                identifier,
                                expression,
                            } = arenas.get(n);
                            let key = arenas.ident_to_str(identifier.item.0);
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
                            let value = eval_constant_expr(
                                arenas,
                                builder.eval_scope(),
                                diagnostics,
                                *expression,
                            )?;
                            params.insert(key.to_string(), value);
                        }
                        ParameterOverrides::Named(params)
                    }
                }),
            };

            let module_name = arenas.ident_to_str(module_identifier.item.0);
            for module_instance in module_instances.iter() {
                let ModuleInstance {
                    name_of_module_instance,
                    list_of_port_connections: _,
                } = arenas.get(module_instance);
                let instance_name = arenas.ident_to_str(name_of_module_instance.item.0);
                let module = HierarchyModule {
                    name: instance_name.to_string(),
                    module_name: module_name.to_string(),
                    children: Default::default(),
                    ast: Some(module_instance),
                    parent: Some(builder.key()),
                    lut: Default::default(),
                    ports: Default::default(),
                    parameter_lut: Default::default(),
                    parameters: Default::default(),
                    parameter_overrides: parameter_overrides.clone(),
                };
                if builder.insert_module(module).is_some() {
                    diagnostics.duplicate_definition(arenas, *name_of_module_instance);
                    return Err(());
                }
            }
            Ok(())
        }
        ModuleOrGenerateItem::InitialConstruct(id) => elaborate_statements(
            signals,
            arenas,
            builder,
            diagnostics,
            AstIdRange::single(arenas.get(*id).0),
        ),
        ModuleOrGenerateItem::AlwaysConstruct(id) => elaborate_statements(
            signals,
            arenas,
            builder,
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

            let GenvarAssignment { ident, expr } = arenas.get(*initialization);
            let GenvarAssignment {
                ident: iteration_ident,
                expr: iteration,
            } = arenas.get(*iteration);

            let genvar_ident = arenas.ident_to_str(ident.item.0);
            if arenas.ident_to_str(iteration_ident.item.0) != genvar_ident {
                diagnostics.not_yet_implemented(
                    arenas.get_span(*initialization),
                    "initialization and iteration assignment identifier are different",
                );
                return Err(());
            }
            let Some(genvar) = genvars.get_mut(genvar_ident) else {
                diagnostics.var_not_found(arenas, *ident);
                return Err(());
            };
            *genvar = true;

            let mut value = eval_constant_expr(arenas, builder.eval_scope(), diagnostics, *expr)?;

            // @GJB: Bit of a hack to add the GenVar as a localparameter and then delete it again.
            // This way it can be used in the condition and iteration.
            macro_rules! with_constant {
                ($scope:ident => $stmt:stmt) => {
                    if let Some(_overwritten) = builder.insert_parameter(HierarchyParameter {
                        name: genvar_ident.to_string(),
                        parent: builder.key,
                        value: value.clone(),
                    }) {
                        return Err(());
                    }

                    let $scope = builder.eval_scope();
                    $stmt
                    builder.hierarchy.symbols[builder.key.as_idx()]
                        .children_mut(builder.hierarchy)
                        .unwrap()
                        .end -= 1;
                    builder.hierarchy.lookup_table.remove(&(builder.key(), genvar_ident.to_string()));
                    builder.hierarchy.symbols.pop().unwrap();
                    builder.hierarchy.parameters.pop().unwrap();
                };
            }

            loop {
                with_constant!(
                    scope => let c = eval_constant_expr(arenas, scope, diagnostics, *condition)?
                );

                if !c.to_logical() {
                    break;
                }

                let (mod_or_gen_items, block_ident, block_ident_ast) = match arenas.get(*block) {
                    GenerateBlock::ModuleOrGenerateItem(id) => {
                        (AstIdRange::single(*id), None, None)
                    }
                    GenerateBlock::BeginEnd(ident, mod_or_gen_items) => (
                        *mod_or_gen_items,
                        ident.map(|i| arenas.ident_to_str(i.item.0)),
                        *ident,
                    ),
                };

                let name = block_ident.map(|i| i.to_string());
                if builder
                    .insert_generate_block(HierarchyGenerateBlock {
                        name,
                        ast: mod_or_gen_items,
                        children: HierarchyItemRange::default(),
                        parent: builder.key(),

                        genvar: Some((genvar_ident.to_string(), value.clone())),
                        genvars: genvars.clone(),
                    })
                    .is_some()
                {
                    diagnostics.duplicate_definition(arenas, block_ident_ast.unwrap());
                    return Err(());
                }

                with_constant!(
                    scope => value = eval_constant_expr(arenas, scope, diagnostics, *iteration)?
                );
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
                eval_constant_expr(arenas, builder.eval_scope(), diagnostics, *condition)?;
            if condition.to_logical() {
                elaborate_generate_block(arenas, builder, diagnostics, *truthy, genvars)?;
            } else if let Some(falsy) = falsy {
                elaborate_generate_block(arenas, builder, diagnostics, *falsy, genvars)?;
            }

            Ok(())
        }
        ModuleOrGenerateItem::CaseGenerateConstruct(id) => {
            let CaseGenerateConstruct { value, items } = arenas.get(*id);
            let value = eval_constant_expr(arenas, builder.eval_scope(), diagnostics, *value)?;

            for item in items.iter() {
                let CaseGenerateItem { pattern, block } = arenas.get(item);
                let mut is_selected = false;
                match pattern {
                    CaseGeneratePattern::Default => is_selected = true,
                    CaseGeneratePattern::Exprs(exprs) => {
                        for expr in exprs.iter() {
                            let expr_value = eval_constant_expr(
                                arenas,
                                builder.eval_scope(),
                                diagnostics,
                                expr,
                            )?;
                            let expr_value =
                                expr_value.truncate_or_extend(value.ty().force_net_width());
                            if value.clone().logical_equal(expr_value) {
                                is_selected = true;
                            }
                        }
                    }
                };

                if is_selected {
                    elaborate_generate_block(arenas, builder, diagnostics, *block, genvars)?;
                    break;
                }
            }

            Ok(())
        }
    }
}

pub fn elaborate_module_or_generate_item_declaration<'a>(
    signals: &mut SlotMap<SignalKey, Signal>,
    arenas: &'a AstArenas,

    id: AstId<ModuleOrGenerateItemDeclaration>,

    builder: &mut ScopeBuilder<'a>,
    diagnostics: &mut Diagnostics,
    genvars: &mut HashMap<String, bool>,
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
                Some(range) => evaluate_range(arenas, builder.eval_scope(), diagnostics, *range)?,
            };
            let ty = VType::net(width, *signed);
            match nets {
                NetDeclarationNets::Idents(idents) => {
                    for net_ident in idents.iter() {
                        let NetIdent { ident, dimension } = arenas.get(net_ident);
                        let origin = arenas.get_item_span(*ident);
                        let name = arenas.ident_to_str(ident.item.0);
                        let dims =
                            dims_to_array(arenas, builder.eval_scope(), diagnostics, *dimension)?;
                        let mut size = ty.force_net_width().get();
                        for dim in &dims {
                            size = size.checked_mul(*dim).ok_or_else(|| {
                                diagnostics.net_width_overflow(arenas.get_span(net_ident));
                                ()
                            })?;
                        }
                        let Some(size) = VectorSize::new(size) else {
                            diagnostics.zero_width_net(arenas.get_span(net_ident));
                            return Err(());
                        };

                        let signal = signals.insert(Signal {
                            name: name.to_string(),
                            size,
                            initialize: None,
                            origin,
                        });
                        let net = HierarchyNet {
                            name: name.to_string(),
                            parent: builder.key(),
                            signal,
                            ty,
                            dims: dims.into(),
                            nba: None,
                        };
                        if builder.insert_net(net).is_some() {
                            diagnostics.duplicate_definition(arenas, *ident);
                            return Err(());
                        }
                    }
                }
                NetDeclarationNets::Assignments(assignments) => {
                    for assignment in assignments.iter() {
                        let NetDeclAssignment { ident, expr: _ } = arenas.get(assignment);
                        let origin = arenas.get_item_span(*ident);
                        let name = arenas.ident_to_str(ident.item.0);
                        let size = ty.force_net_width();
                        let signal = signals.insert(Signal {
                            name: name.to_string(),
                            size,
                            initialize: None,
                            origin,
                        });
                        let net = HierarchyNet {
                            name: name.to_string(),
                            parent: builder.key(),
                            signal,
                            ty,
                            dims: [].into(),
                            nba: None,
                        };
                        if builder.insert_net(net).is_some() {
                            diagnostics.duplicate_definition(arenas, *ident);
                            return Err(());
                        }
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
                Some(range) => evaluate_range(arenas, builder.eval_scope(), diagnostics, *range)?,
            };

            let ty = VType::net(size, *signed);
            for variable_type in variable_types.iter() {
                error |= elaborate_variable_type(
                    signals,
                    arenas,
                    builder,
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
                    builder,
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
                let ident = arenas.ident_to_str(ast_ident.item.0);

                if genvars.insert(ident.to_string(), false).is_some() {
                    diagnostics.duplicate_definition(arenas, ast_ident);
                    error = true;
                }
            }
            if error {
                return Err(());
            }
        }
        ModuleOrGenerateItemDeclaration::Task(id) => {
            let TaskDeclaration {
                ident, automatic, ..
            } = arenas.get(*id);

            let name = arenas.ident_to_str(ident.item.0);
            let task = HierarchyTask {
                name: name.to_string(),
                ast: *id,
                children: HierarchyItemRange::default(),
                parent: builder.key(),
                automatic: *automatic,

                lower: None,
            };
            if builder.insert_task(task).is_some() {
                diagnostics.duplicate_definition(arenas, *ident);
                error = true;
            }
        }
        ModuleOrGenerateItemDeclaration::Function(id) => {
            let FunctionDeclaration {
                ident, automatic, ..
            } = arenas.get(*id);

            let name = arenas.ident_to_str(ident.item.0);
            let function = HierarchyFunction {
                name: name.to_string(),
                ast: *id,
                children: HierarchyItemRange::default(),
                parent: builder.key(),
                automatic: *automatic,
                lower: None,
            };
            if builder.insert_function(function).is_some() {
                diagnostics.duplicate_definition(arenas, *ident);
                error = true;
            }
        }
    }

    if error { Err(()) } else { Ok(()) }
}

pub fn elaborate_variable_type<'a>(
    signals: &mut SlotMap<SignalKey, Signal>,
    arenas: &'a AstArenas,
    builder: &mut ScopeBuilder<'a>,
    diagnostics: &mut Diagnostics,
    variable_type: AstId<VariableType>,
    ty: VType,
) -> Result<(), ()> {
    let VariableType {
        identifier,
        variant,
    } = arenas.get(variable_type);

    let origin = arenas.get_span(variable_type);
    let name = arenas.ident_to_str(identifier.item.0);

    let (dims, size) = match variant {
        VariableTypeVariant::Dimensions(dimensions) => {
            let dims = dims_to_array(arenas, builder.eval_scope(), diagnostics, *dimensions)?;
            let mut size = ty.force_net_width().get();
            for dim in &dims {
                size = size.checked_mul(*dim).ok_or_else(|| {
                    diagnostics.net_width_overflow(arenas.get_span(variable_type));
                    ()
                })?;
            }
            let Some(size) = VectorSize::new(size) else {
                diagnostics.zero_width_net(arenas.get_span(variable_type));
                return Err(());
            };

            Ok((dims.into(), size))
        }
        VariableTypeVariant::ConstantExpr(_) => Ok(([].into(), ty.force_net_width())),
    }?;

    let signal = signals.insert(Signal {
        name: name.to_string(),
        size,
        initialize: None,
        origin,
    });
    let net = HierarchyNet {
        name: name.to_string(),
        parent: builder.key(),
        signal,
        ty,
        dims,
        nba: None,
    };
    if builder.insert_net(net).is_some() {
        diagnostics.duplicate_definition(arenas, *identifier);
        return Err(());
    }

    Ok(())
}

pub fn parameter_typing_to_type<'a>(
    arenas: &'a AstArenas,
    builder: &mut ScopeBuilder<'a>,
    diagnostics: &mut Diagnostics,
    typing: AstId<ParameterDeclarationTyping>,
) -> Result<(i64, i64, VType), ()> {
    Ok(match arenas.get(typing) {
        ParameterDeclarationTyping::None(signed, range) => {
            let (msb, lsb, width) = match range {
                None => (0, 0, SCALAR_VSIZE),
                Some(range) => evaluate_range(arenas, builder.eval_scope(), diagnostics, *range)?,
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
    })
}

pub fn elaborate_statements<'a>(
    signals: &mut SlotMap<SignalKey, Signal>,
    arenas: &'a AstArenas,
    builder: &mut ScopeBuilder<'a>,
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
                        builder,
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
                    elaborate_statement_or_null(signals, arenas, builder, diagnostics, *statement)
                        .is_err();
                for else_if in else_ifs.iter() {
                    let IfBranch {
                        condition: _,
                        statement,
                    } = arenas.get(else_if);
                    error |= elaborate_statement_or_null(
                        signals,
                        arenas,
                        builder,
                        diagnostics,
                        *statement,
                    )
                    .is_err();
                }
                if let Some(statement) = else_branch {
                    error |= elaborate_statement_or_null(
                        signals,
                        arenas,
                        builder,
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
                    builder,
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
                    builder,
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
                        let name = arenas.ident_to_str(block_identifier.item.0);
                        let named_block = HierarchyNamedBlock {
                            name: name.to_string(),
                            ast: *id,
                            children: Default::default(),
                            parent: builder.key(),
                        };
                        builder.insert_named_block(named_block);
                    }
                    None => {
                        error |=
                            elaborate_statements(signals, arenas, builder, diagnostics, *statements)
                                .is_err()
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
                    builder,
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
    signals: &mut SlotMap<SignalKey, Signal>,
    arenas: &'a AstArenas,
    builder: &mut ScopeBuilder<'a>,
    diagnostics: &mut Diagnostics,
    stmt: AstId<StatementOrNull>,
) -> Result<(), ()> {
    match arenas.get(stmt) {
        StatementOrNull::Attribute(_) => Ok(()),
        StatementOrNull::Statement(id) => elaborate_statements(
            signals,
            arenas,
            builder,
            diagnostics,
            AstIdRange::single(*id),
        ),
    }
}

pub fn elaborate_generate_block<'a>(
    arenas: &'a AstArenas,
    builder: &mut ScopeBuilder<'a>,
    diagnostics: &mut Diagnostics,
    blk: AstId<Option<GenerateBlock>>,
    genvars: &HashMap<String, bool>,
) -> Result<(), ()> {
    let (mod_or_gen_items, block_ident, block_ident_ast) = match arenas.get(blk) {
        None => (AstIdRange::default(), None, None),
        Some(GenerateBlock::ModuleOrGenerateItem(id)) => (AstIdRange::single(*id), None, None),
        Some(GenerateBlock::BeginEnd(ident, mod_or_gen_items)) => (
            *mod_or_gen_items,
            ident.map(|i| arenas.ident_to_str(i.item.0)),
            *ident,
        ),
    };

    let name = block_ident.map(|i| i.to_string());
    if builder
        .insert_generate_block(HierarchyGenerateBlock {
            name,
            ast: mod_or_gen_items,
            children: HierarchyItemRange::default(),
            parent: builder.key(),

            genvar: None,
            genvars: genvars.clone(),
        })
        .is_some()
    {
        diagnostics.duplicate_definition(arenas, block_ident_ast.unwrap());
        return Err(());
    }
    Ok(())
}

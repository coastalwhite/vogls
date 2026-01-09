use slotmap::SlotMap;
use vogls_ir::{INTEGER_VSIZE, SCALAR_VSIZE, Signal, SignalKey, VectorSize};

use crate::ast::constant_expr::ConstantMinTypMaxExpression;
use crate::ast::module::{
    FunctionDeclaration, IntegerDeclaration, LocalParameterDeclaration, Module, ModuleInstance,
    ModuleInstantiation, ModuleItem, ModuleOrGenerateItem, ModuleOrGenerateItemDeclaration,
    ModulePorts, NetDeclAssignment, NetDeclaration, NetDeclarationNets, NetIdent,
    NonPortModuleItem, ParamAssignment, ParameterDeclaration, ParameterDeclarationTyping, Port,
    PortDeclaration, PortExpression, PortReference, RegDeclaration, TaskDeclaration, VariableType,
    VariableTypeVariant,
};
use crate::ast::statement::{
    Block, CaseItem, CaseStatement, ConditionalStatement, IfBranch, LoopStatement,
    ProceduralTimingControlStatement, SeqBlock, Statement, StatementContent, StatementOrNull,
    WaitStatement,
};
use crate::ast::{AstId, AstIdRange};
use crate::hierarchy::{
    HierarchyFunction, HierarchyItemRange, HierarchyModule, HierarchyNamedBlock, HierarchyNet,
    HierarchyParameter, HierarchyTask, ScopeBuilder,
};
use crate::lower::{Diagnostics, VType, dims_to_array, eval_constant_expr, evaluate_range};
use crate::parser::AstArenas;

pub enum ElaborationItem {
    ModuleInstance,
    NamedBlock,
}

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

    if let Some(module_parameter_port_list) = module_parameter_port_list {
        for id in module_parameter_port_list.iter() {
            // @TODO:
            // We need to immediately exit here as a failed elaboration will have knock on effects
            // for future parameters.
            //
            // We should add the parameters into the scope, but mark them erroneous. When an
            // erroneous parameter is used later, it would then quietly ignore that elaboration and
            // continue.
            //
            // This way, you can get the broadest error messages.
            elaborate_parameter_declaration(signals, arenas, id, builder, diagnostics)?;
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

                        let name = arenas.get_ident(identifier.item.0);
                        let origin = arenas.get_item_span(*identifier);

                        let ty = VType::UnsignedNet(SCALAR_VSIZE);
                        let signal = signals.insert(Signal {
                            name: name.to_string(),
                            size: ty.force_net_width(),
                            initialize: None,
                            origin,
                        });
                        let net = HierarchyNet {
                            name: name.to_string(),
                            signal,
                            ty,
                            dims: [].into(),
                        };

                        if builder.insert_net(net).is_some() {
                            diagnostics.duplicate_definition(arenas, *identifier);
                            error = true;
                            continue;
                        }
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
                    )
                    .is_err();
                }
                NonPortModuleItem::GenerateRegion(id) => {}
                NonPortModuleItem::SpecifyBlock => todo!(),
                NonPortModuleItem::ParameterDeclaration(id) => {
                    elaborate_parameter_declaration(signals, arenas, *id, builder, diagnostics)?
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
    signals: &mut SlotMap<SignalKey, Signal>,
    arenas: &'a AstArenas,

    id: AstId<ParameterDeclaration>,

    builder: &mut ScopeBuilder<'a>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let ParameterDeclaration {
        typing,
        assignments,
    } = arenas.get(id);

    let _ty = parameter_typing_to_type(arenas, builder, diagnostics, *typing)?;
    for assignment in assignments.iter() {
        let ParamAssignment { param, constant } = arenas.get(assignment);
        let name = arenas.get_ident(param.item.0);
        let value = match arenas.get(*constant) {
            ConstantMinTypMaxExpression::Single(id) => {
                eval_constant_expr(arenas, &builder.scope(), diagnostics, *id)?
            }
            ConstantMinTypMaxExpression::MinTypMax { .. } => todo!(),
        };

        if builder
            .insert_parameter(HierarchyParameter {
                name: name.to_string(),
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
    let (range, signed, identifiers) = match arenas.get(id) {
        PortDeclaration::Inout(id) => {
            let inout = arenas.get(*id);
            (inout.range, inout.signed, inout.port_identifiers)
        }
        PortDeclaration::Input(id) => {
            let input = arenas.get(*id);
            (input.range, input.signed, input.port_identifiers)
        }
        PortDeclaration::Output(id) => {
            let output = arenas.get(*id);
            (output.range, output.signed, output.identifiers)
        }
    };
    let (msb, lsb, size) = match range {
        None => (0, 0, SCALAR_VSIZE),
        Some(range) => evaluate_range(arenas, &builder.scope(), diagnostics, range)?,
    };
    let ty = VType::UnsignedNet(SCALAR_VSIZE);

    let mut error = false;
    for ident in identifiers.iter() {
        let name = arenas.get_ident(arenas.get(ident).0);
        let origin = arenas.get_span(ident);
        let signal = signals.insert(Signal {
            name: name.to_string(),
            size: ty.force_net_width(),
            initialize: None,
            origin,
        });
        let net = HierarchyNet {
            name: name.to_string(),
            signal,
            ty,
            dims: [].into(),
        };
        if builder.insert_net(net).is_some() {
            diagnostics.duplicate_definition(arenas, arenas.to_item(ident));
            error = true;
            continue;
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
) -> Result<(), ()> {
    match arenas.get(id) {
        ModuleOrGenerateItem::ModuleOrGenerateItemDeclaration(id) => {
            elaborate_module_or_generate_item_declaration(
                signals,
                arenas,
                *id,
                builder,
                diagnostics,
            )
        }
        ModuleOrGenerateItem::LocalParameterDeclaration(id) => {
            let LocalParameterDeclaration {
                typing,
                assignments,
            } = arenas.get(*id);

            let _ty = parameter_typing_to_type(arenas, builder, diagnostics, *typing)?;
            for assignment in assignments.iter() {
                let ParamAssignment { param, constant } = arenas.get(assignment);
                let name = arenas.get_ident(param.item.0);
                let value = match arenas.get(*constant) {
                    ConstantMinTypMaxExpression::Single(id) => {
                        eval_constant_expr(arenas, &builder.scope(), diagnostics, *id)?
                    }
                    ConstantMinTypMaxExpression::MinTypMax { .. } => todo!(),
                };

                if builder
                    .insert_parameter(HierarchyParameter {
                        name: name.to_string(),
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
        ModuleOrGenerateItem::ParameterOverride => todo!(),
        ModuleOrGenerateItem::ContinuousAssign(id) => todo!(),
        ModuleOrGenerateItem::GateInstantiation(id) => todo!(),
        ModuleOrGenerateItem::UdpInstantiation => todo!(),
        ModuleOrGenerateItem::ModuleInstantiation(id) => {
            let ModuleInstantiation {
                module_identifier,
                parameter_value_assignment,
                module_instances,
            } = arenas.get(*id);
            let module_name = arenas.get_ident(module_identifier.item.0);
            for module_instance in module_instances.iter() {
                let ModuleInstance {
                    name_of_module_instance,
                    list_of_port_connections,
                } = arenas.get(module_instance);
                let instance_name = arenas.get_ident(name_of_module_instance.item.0);
                let module = HierarchyModule {
                    name: instance_name.to_string(),
                    module_name: module_name.to_string(),
                    children: Default::default(),
                    ast: Some(module_instance),
                    parent: Some(builder.key()),
                    lut: Default::default(),
                    ports: Default::default(),
                };
                builder.insert_module(module);
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
        ModuleOrGenerateItem::LoopGenerateConstruct(id) => todo!(),
        ModuleOrGenerateItem::IfGenerateConstruct(id) => todo!(),
        ModuleOrGenerateItem::CaseGenerateConstruct(id) => todo!(),
    }
}

pub fn elaborate_module_or_generate_item_declaration<'a>(
    signals: &mut SlotMap<SignalKey, Signal>,
    arenas: &'a AstArenas,

    id: AstId<ModuleOrGenerateItemDeclaration>,

    builder: &mut ScopeBuilder<'a>,
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
            let (msb, lsb, width) = match range {
                None => (0, 0, SCALAR_VSIZE),
                Some(range) => evaluate_range(arenas, &builder.scope(), diagnostics, *range)?,
            };
            let ty = VType::SignedNet(INTEGER_VSIZE);
            match nets {
                NetDeclarationNets::Idents(idents) => {
                    for net_ident in idents.iter() {
                        let NetIdent { ident, dimension } = arenas.get(net_ident);
                        let origin = arenas.get_item_span(*ident);
                        let name = arenas.get_ident(ident.item.0);
                        let dims =
                            dims_to_array(arenas, &builder.scope(), diagnostics, *dimension)?;
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
                            signal,
                            ty,
                            dims: dims.into(),
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
                        let name = arenas.get_ident(ident.item.0);
                        let size = ty.force_net_width();
                        let signal = signals.insert(Signal {
                            name: name.to_string(),
                            size,
                            initialize: None,
                            origin,
                        });
                        let net = HierarchyNet {
                            name: name.to_string(),
                            signal,
                            ty,
                            dims: [].into(),
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
            let (msb, lsb, size) = match range {
                None => (0, 0, SCALAR_VSIZE),
                Some(range) => evaluate_range(arenas, &builder.scope(), diagnostics, *range)?,
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
        ModuleOrGenerateItemDeclaration::Genvar(id) => todo!(),
        ModuleOrGenerateItemDeclaration::Task(id) => {
            let TaskDeclaration {
                ident,
                automatic,
                statement_or_null: _,
            } = arenas.get(*id);

            let name = arenas.get_ident(ident.item.0);
            let task = HierarchyTask {
                name: name.to_string(),
                ast: *id,
                children: HierarchyItemRange::default(),
                parent: builder.key(),
            };
            if builder.insert_task(task).is_some() {
                diagnostics.duplicate_definition(arenas, *ident);
                error = true;
            }
        }
        ModuleOrGenerateItemDeclaration::Function(id) => {
            let FunctionDeclaration {
                ident,
                automatic,
                range_or_type,
                tf_input_decls,
                block_item_decls,
                statement,
            } = arenas.get(*id);

            let name = arenas.get_ident(ident.item.0);
            let function = HierarchyFunction {
                name: name.to_string(),
                ast: *id,
                children: HierarchyItemRange::default(),
                parent: builder.key(),
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
    let name = arenas.get_ident(identifier.item.0);

    let (dims, size) = match variant {
        VariableTypeVariant::Dimensions(dimensions) => {
            let dims = dims_to_array(arenas, &builder.scope(), diagnostics, *dimensions)?;
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
        signal,
        ty,
        dims,
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
                Some(range) => evaluate_range(arenas, &builder.scope(), diagnostics, *range)?,
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
                        let name = arenas.get_ident(block_identifier.item.0);
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

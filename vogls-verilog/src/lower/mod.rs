mod assign;
mod diagnostics;
mod expression;
mod module_or_generate_item;
mod parameter;
// mod scope;
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

pub struct Scope<'a> {
    pub hierarchy: &'a mut Hierarchy,
    pub key: HierarchyKey,
    pub signal_map: &'a mut HashMap<SignalKey, SignalKey>,
}

#[derive(Clone, Copy)]
pub struct EvalScope<'a> {
    pub hierarchy: &'a Hierarchy,
    pub key: HierarchyKey,
}

impl<'a> EvalScope<'a> {
    fn get(&self, ident: &str) -> Option<HierarchyKey> {
        if let Some(k) = self.hierarchy.lookup().get(&(self.key, ident.to_string())) {
            return Some(*k);
        }

        // @TODO: Hierarchical Path Identifiers.
        None
    }
}

impl<'a> Scope<'a> {
    fn get(&self, ident: &str) -> Option<HierarchyKey> {
        if let Some(k) = self.hierarchy.lookup().get(&(self.key, ident.to_string())) {
            return Some(*k);
        }

        // @TODO: Hierarchical Path Identifiers.
        None
    }

    fn get_unwrap_net(&self, ident: &str) -> Option<&HierarchyNet> {
        let symbol_key = self.get(ident)?;
        let HierarchyItem::Net(n) = self.hierarchy.items()[symbol_key.as_idx()] else {
            unreachable!("not a net");
        };
        Some(&self.hierarchy.net()[n])
    }

    pub fn eval<'b>(&'b self) -> EvalScope<'b> {
        EvalScope {
            hierarchy: &self.hierarchy,
            key: self.key,
        }
    }
}

use std::collections::HashMap;

use slotmap::{SecondaryMap, SparseSecondaryMap};
use vogls_ir::{
    BasicBlockKey, ConnectionDirection, GlobalContext, SCALAR_VSIZE, Signal, SignalKey, VectorSize,
    new_process,
};

use crate::ast::constant_expr::{ConstantExpr, ConstantMinTypMaxExpression};
use crate::ast::expr::{BitSlice, Expr};
use crate::ast::module::{
    GenerateRegion, Module, ModuleItem, NonPortModuleItem, ParamAssignment, ParameterDeclaration,
    Range,
};
use crate::ast::{AstId, Identifier};
use crate::hierarchy::{Hierarchy, HierarchyItem, HierarchyKey, HierarchyNet};
use crate::parser::AstArenas;

pub use self::expression::eval_constant_expr;
use self::expression::{lower_expr, truncate_or_extend};
pub use self::vtype::VType;
pub use self::vvalue::VValue;
pub use diagnostics::Diagnostics;
pub use module_or_generate_item::dims_to_array;

pub fn lower_module_to_ir<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    root: AstId<Module>,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let Module {
        attribute_instances: _,
        module_identifier: _,
        module_parameter_port_list: _,
        ports: _,
        module_items,
        default_nettype: _,
    } = arenas.get(root);

    for module_item in module_items.iter() {
        match arenas.get(module_item) {
            ModuleItem::PortDeclaration(_) => {}
            ModuleItem::NonPortModuleItem(p) => match arenas.get(*p) {
                NonPortModuleItem::ModuleOrGenerateItem(id) => {
                    module_or_generate_item::lower(gl, arenas, scope, *id, diagnostics)?
                }
                NonPortModuleItem::GenerateRegion(region) => {
                    let GenerateRegion {
                        module_or_generate_item,
                    } = region;
                    for id in module_or_generate_item.iter() {
                        module_or_generate_item::lower(gl, arenas, scope, id, diagnostics)?;
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

                        let _value = eval_constant_expr(arenas, scope.eval(), diagnostics, *constant)?;
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

fn lower_to_signal<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
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
        if let HierarchyItem::Net(s) = &scope.hierarchy.items()[symbol_key.as_idx()]
            && let s = &scope.hierarchy.net()[*s]
            && s.ty == ty
        {
            return Ok(s.signal);
        }
    }

    let signal = gl.signals.insert(Signal {
        name: "anon_port_assignment".to_string(),
        size: ty.force_net_width(),
        initialize: None,
        origin: arenas.get_span(expr),
    });

    let mut bb_builder = new_process(gl, "port_assignment".into(), arenas.get_span(expr));
    let bb_key = bb_builder.key();
    let (v, v_ty) = lower_expr(gl, arenas, scope, diagnostics, &mut bb_builder, expr)?;
    let v = expression::sign_or_zero_extend(gl, &mut bb_builder, v, v_ty, ty.force_net_width());

    bb_builder.drive(gl, signal, v);

    bb_builder.watch_for_ins_to(gl, bb_key);
    Ok(signal)
}

fn assign_port_output<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    expr: AstId<Expr>,
    output_net: usize,
    ty: VType,
) -> Result<(), ()> {
    if let Expr::Ident(ast_ident, exprs, range_expression) = arenas.get(expr)
        && exprs.is_empty()
        && range_expression.is_none()
    {
        let ident = arenas.get_ident(ast_ident.item.0);
        let Some(symbol_key) = scope.get(&ident) else {
            diagnostics.var_not_found(arenas, *ast_ident);
            return Err(());
        };
        if let HierarchyItem::Net(s) = scope.hierarchy.items()[symbol_key.as_idx()]
            && let s = &scope.hierarchy.nets[s]
            && s.ty == ty
        {
            let signal = s.signal;
            let old_signal =
                std::mem::replace(&mut scope.hierarchy.nets[output_net].signal, signal);
            scope.signal_map.insert(old_signal, signal);
            gl.signals.remove(old_signal);
            return Ok(());
        }
    }

    let mut driving: Vec<AstId<Expr>> = Vec::new();
    driving.push(expr);

    let signal = scope.hierarchy.net()[output_net].signal;

    let mut bb_builder = new_process(gl, "port_assignment".into(), arenas.get_span(expr));
    let bb_key = bb_builder.key();

    let probed = bb_builder.probe(gl, signal);

    let mut error = false;
    while let Some(expr) = driving.pop() {
        match arenas.get(expr) {
            Expr::Concatenation(_) => {
                todo!()
            }
            Expr::Ident(ast_ident, exprs, range_expression) => {
                let ident = arenas.get_ident(ast_ident.item.0);
                let Some(symbol_key) = scope.get(&ident) else {
                    diagnostics.var_not_found(arenas, *ast_ident);
                    error = true;
                    continue;
                };
                let HierarchyItem::Net(s) = &scope.hierarchy.items()[symbol_key.as_idx()] else {
                    diagnostics.output_expr_not_allowed(arenas.get_span(expr));
                    error = true;
                    continue;
                };
                let s = &scope.hierarchy.net()[*s];

                let (offset_dst, length_dst) = if range_expression.is_none() && exprs.is_empty() {
                    (bb_builder.constant_u32(gl, 0), Some(s.ty.force_net_width()))
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
                                msb_lsb_to_width(arenas, scope.eval(), diagnostics, *msb, *lsb)?;
                            let offset = bb_builder.constant_u32(gl, lsb as u32);
                            (offset, Some(width as VectorSize))
                        }
                        BitSlice::PlusWidth(base, width) => {
                            let offset =
                                lower_expr(gl, arenas, scope, diagnostics, &mut bb_builder, *base);
                            let width = eval_constant_expr(arenas, scope.eval(), diagnostics, *width);
                            let width = width?.as_integer().unwrap();
                            (offset?.0, Some(VectorSize::new(width as u32).unwrap()))
                        }
                        BitSlice::MinusWidth(base, width) => {
                            let offset =
                                lower_expr(gl, arenas, scope, diagnostics, &mut bb_builder, *base);
                            let width = eval_constant_expr(arenas, scope.eval(), diagnostics, *width)?;
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

                let length_dst = length_dst.unwrap_or(SCALAR_VSIZE);
                let src = probed;
                let src = truncate_or_extend(gl, &mut bb_builder, src, ty, length_dst);
                bb_builder.drive_partial(gl, s.signal, src, offset_dst, length_dst);
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

    if error {
        return Err(());
    }

    Ok(())
}

fn msb_lsb_to_width<'a>(
    arenas: &'a AstArenas,
    scope: EvalScope<'a>,
    diagnostics: &mut Diagnostics,
    ast_msb: AstId<ConstantExpr>,
    ast_lsb: AstId<ConstantExpr>,
) -> Result<(i64, i64, VectorSize), ()> {
    let msb = eval_constant_expr(arenas, scope, diagnostics, ast_msb);
    let lsb = eval_constant_expr(arenas, scope, diagnostics, ast_lsb);

    let (Ok(VValue::SignedNet(msb)), Ok(VValue::SignedNet(lsb))) = (msb, lsb) else {
        return Err(());
    };
    let msb = msb.as_i64().unwrap();
    let lsb = lsb.as_i64().unwrap();
    let width = u32::try_from(msb.abs_diff(lsb)).ok();
    let width = width.and_then(|w| w.checked_add(1));
    let width = width.and_then(|w| VectorSize::new(w));
    let Some(width) = width else {
        let tr = arenas.get_span(ast_msb) | arenas.get_span(ast_lsb);
        diagnostics.net_width_overflow(tr);
        return Err(());
    };
    Ok((msb, lsb, width))
}

pub fn evaluate_range<'a>(
    arenas: &'a AstArenas,
    scope: EvalScope<'a>,
    diagnostics: &mut Diagnostics,
    range: AstId<Range>,
) -> Result<(i64, i64, VectorSize), ()> {
    let range = arenas.get(range);
    msb_lsb_to_width(arenas, scope, diagnostics, range.msb, range.lsb)
}

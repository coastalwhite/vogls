use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::dyn_format_string::{Base, DynFormatArgument, DynFormatString, Padding};
use vogls_ir::{BasicBlockBuilder, IntrinsicOp, ReadMem, VariableKey};

use crate::ast::AstId;
use crate::ast::expr::Expr;
use crate::ast::statement::SystemTaskEnable;
use crate::elaborate::{VSymbol, determine_module_context};
use crate::lower::expression::{get_expr_type, lower_expr};
use crate::lower::{LowerContext, MutLowerContext, try_resolve_hident};
use crate::lower::{expression, hident_span, try_resolve_net};

pub fn lower_system_task_enable<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    mut builder: BasicBlockBuilder,
    system_task_enable: AstId<'a, SystemTaskEnable<'a>>,
) -> Result<BasicBlockBuilder, ()> {
    let SystemTaskEnable {
        system_task_identifier,
        expressions,
    } = &*system_task_enable;
    let system_task_ident = &ctx.arenas.ident_table[system_task_identifier.item.0];

    match system_task_ident {
        "display" => {
            let (mut format_string_content, format_string_arguments, format_string_args) =
                lower_write_arguments(ctx, mctx, scope, system_task_enable, &mut builder)?;
            use std::fmt::Write;
            writeln!(&mut format_string_content).unwrap();
            let format_str =
                DynFormatString::new(format_string_content.into(), format_string_arguments.into());
            builder.intrinsic(
                mctx.gl(),
                IntrinsicOp::Display(Box::new(format_str)),
                format_string_args.into(),
            );
        }
        "write" => {
            let (format_string_content, format_string_arguments, format_string_args) =
                lower_write_arguments(ctx, mctx, scope, system_task_enable, &mut builder)?;
            let format_str =
                DynFormatString::new(format_string_content.into(), format_string_arguments.into());
            builder.intrinsic(
                mctx.gl(),
                IntrinsicOp::Display(Box::new(format_str)),
                format_string_args.into(),
            );
        }
        "vogls_assert_eq" | "vogls_assert_ne" => {
            if expressions.len() != 2 {
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_span(system_task_enable),
                    "assertions requires two arguments",
                );
                return Err(());
            }

            let line_number =
                ctx.get_line_number(ctx.arenas.get_item_span(*system_task_identifier).start);

            let lhs = expressions.get(0);
            let rhs = expressions.get(1);

            let l_ty = get_expr_type(
                &mctx.gl,
                ctx.arenas,
                &ctx.table,
                scope,
                &mut mctx.diagnostics,
                lhs,
            )?;
            let r_ty = get_expr_type(
                &mctx.gl,
                ctx.arenas,
                &ctx.table,
                scope,
                &mut mctx.diagnostics,
                rhs,
            )?;

            let context_width = l_ty.force_net_width().max(r_ty.force_net_width());

            let (lhs, lhs_ty) =
                lower_expr(ctx, mctx, scope, &mut builder, lhs, Some(context_width))?;
            let (rhs, rhs_ty) =
                lower_expr(ctx, mctx, scope, &mut builder, rhs, Some(context_width))?;

            let (lhs, _, rhs, _) = expression::coerce_bin_arithmetic(
                mctx.gl(),
                &mut builder,
                lhs,
                lhs_ty,
                rhs,
                rhs_ty,
            );
            static FAILED_STR: &str = "Assertion failed on line .  != \n";
            let (condition, content) = if system_task_ident == "vogls_assert_eq" {
                (builder.case_equals(mctx.gl(), lhs, rhs), FAILED_STR)
            } else {
                (builder.not_case_equals(mctx.gl(), lhs, rhs), FAILED_STR)
            };
            let format_str = DynFormatString::new(
                content.into(),
                [
                    (
                        25,
                        DynFormatArgument {
                            padding: Padding::NoPadding,
                            base: Base::Decimal,
                            signed: false,
                            prefix: false,
                        },
                    ),
                    (27, DynFormatArgument::default()),
                    (31, DynFormatArgument::default()),
                ]
                .into(),
            );

            let line = builder.constant_u32(mctx.gl(), line_number as u32);
            builder.intrinsic(
                mctx.gl(),
                IntrinsicOp::Assert(Box::new(format_str)),
                [condition, line, lhs, rhs].into(),
            );
        }
        "finish" => _ = builder.intrinsic(mctx.gl(), IntrinsicOp::Finish, Default::default()),

        "printtimescale" => {
            if expressions.len() > 1 {
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_span(system_task_enable),
                    "only one identifier is allowed",
                );
                return Err(());
            }

            let (sid, module) = match expressions.first() {
                Some(expr) => {
                    let Expr::Ident(ident, exprs, range_expr) = &*expr else {
                        mctx.diagnostics.not_yet_implemented(
                            ctx.arenas.get_span(system_task_enable),
                            "invalid identifier",
                        );
                        return Err(());
                    };
                    if !exprs.is_empty() || range_expr.is_some() {
                        mctx.diagnostics.not_yet_implemented(
                            ctx.arenas.get_span(system_task_enable),
                            "array elements and ranges not supported yet",
                        );
                        return Err(());
                    }

                    let sid = try_resolve_hident(
                        scope,
                        &ctx.table,
                        ctx.arenas,
                        *ident,
                        &mut mctx.diagnostics,
                    )?;

                    let VSymbol::Module(n) = &ctx.table[sid].content else {
                        mctx.diagnostics.not_yet_implemented(
                            hident_span(&ctx.arenas, *ident),
                            "symbol is not a module",
                        );
                        return Err(());
                    };

                    (sid, n)
                }
                None => determine_module_context(scope, &ctx.table),
            };

            let name = &ctx.arenas.ident_table[ctx.table[sid].name()];
            let fmt = DynFormatString::from_string(format!(
                "Time scale of ({name}) is {}{} / {}{}\n",
                module.time_scale.time_unit_size.as_str(),
                module.time_scale.time_unit_unit.as_str(),
                module.time_scale.time_precision_size.as_str(),
                module.time_scale.time_precision_unit.as_str(),
            ));
            builder.intrinsic(mctx.gl(), IntrinsicOp::Display(Box::new(fmt)), [].into());
        }

        "dumpfile" => {
            assert!(expressions.len() <= 1);
            mctx.has_vcd = true;
            let path = match expressions.first().and_then(|e| e.into_str_literal()) {
                None => "dump.vcd".to_string(),
                Some(str_literal) => {
                    ctx.arenas.text[str_literal.0.start..str_literal.0.end].to_string()
                }
            };
            builder.intrinsic(mctx.gl(), IntrinsicOp::VcdOpenFile(path), [].into());
        }
        "dumpvars" => {
            if !expressions.is_empty() {
                mctx.diagnostics.warnings.push((
                    ctx.arenas.get_span(system_task_enable),
                    "not yet supported to select subset of hierarchy".to_string(),
                ));
            }
            mctx.has_vcd = true;
            builder.intrinsic(
                mctx.gl(),
                IntrinsicOp::VcdAppendModule(ctx.vcd_scope(scope, &ctx.arenas.ident_table)),
                [].into(),
            );
        }

        "readmemb" | "readmemh" => {
            assert!((2..=4).contains(&expressions.len()));
            let path = match &*expressions.get(0) {
                Expr::String(s) => ctx.arenas.text[s.0.start..s.0.end].to_string(),
                Expr::Ident(ident, exprs, range_exprs)
                    if exprs.is_empty() && range_exprs.is_none() =>
                {
                    let sid = try_resolve_hident(
                        scope,
                        &ctx.table,
                        ctx.arenas,
                        *ident,
                        &mut mctx.diagnostics,
                    )?;
                    let VSymbol::Parameter(v) = &ctx.table[sid].content else {
                        mctx.diagnostics.warnings.push((
                            ctx.arenas.get_span(system_task_enable),
                            "ignored: not yet supported path".to_string(),
                        ));
                        return Ok(builder);
                    };
                    if v.clone().into_bits().count_zeros() != v.ty().force_net_width().get() {
                        mctx.diagnostics.warnings.push((
                            ctx.arenas.get_span(system_task_enable),
                            "ignored: not yet supported path".to_string(),
                        ));
                        return Ok(builder);
                    }
                    String::new()
                }
                _ => {
                    mctx.diagnostics.warnings.push((
                        ctx.arenas.get_span(system_task_enable),
                        "ignored: not yet supported path".to_string(),
                    ));
                    return Ok(builder);
                }
            };
            let Expr::Ident(ident, exprs, range_expr) = &*expressions.get(1) else {
                mctx.diagnostics
                    .not_yet_implemented(ctx.arenas.get_span(system_task_enable), "invalid memory");
                return Err(());
            };
            if !exprs.is_empty() || range_expr.is_some() {
                mctx.diagnostics.not_yet_implemented(
                    ctx.arenas.get_span(system_task_enable),
                    "array elements and ranges not supported yet",
                );
                return Err(());
            }
            let net =
                try_resolve_net(scope, &ctx.table, ctx.arenas, *ident, &mut mctx.diagnostics)?;
            if net.dims.len() != 1 {
                mctx.diagnostics.not_yet_implemented(
                    hident_span(ctx.arenas, *ident),
                    "only single arrays are supported as memory at the moment",
                );
                return Err(());
            }

            if expressions.len() > 2 {
                mctx.diagnostics.warn_not_yet_implemented(
                    ctx.arenas.get_span(expressions.get(2)),
                    "ignored at the moment",
                );
            }
            if expressions.len() > 3 {
                mctx.diagnostics.warn_not_yet_implemented(
                    ctx.arenas.get_span(expressions.get(3)),
                    "ignored at the moment",
                );
            }

            let stride = net.ty.force_net_width();
            let offset = 0;
            let limit = net.dims[0].get();
            let signal = net.net.blocking_drive_signal();

            let binary = system_task_ident == "readmemb";
            let readmem = ReadMem {
                path,
                signal,
                offset,
                stride,
                limit,
                binary,
            };

            builder.intrinsic(
                mctx.gl(),
                IntrinsicOp::ReadMem(Box::new(readmem)),
                [].into(),
            );
        }

        // @Incomplete: Many variants here.
        _ => {
            mctx.diagnostics.not_yet_implemented(
                ctx.arenas.get_span(system_task_enable),
                "system task not yet implemented",
            );
            return Err(());
        }
    }
    Ok(builder)
}

#[expect(clippy::type_complexity)]
pub fn lower_write_arguments<'a>(
    ctx: &LowerContext<'a, '_>,
    mctx: &mut MutLowerContext,
    scope: SymbolId,
    system_task_enable: AstId<'a, SystemTaskEnable<'a>>,
    builder: &mut BasicBlockBuilder,
) -> Result<(String, Vec<(usize, DynFormatArgument)>, Vec<VariableKey>), ()> {
    let expressions = system_task_enable.expressions;
    let mut format_string_content = String::new();
    let mut format_string_arguments = Vec::new();
    let mut format_string_args = Vec::new();
    let mut required_arguments_left = 0;
    for expr in expressions.iter() {
        if let Some(str_literal) = expr.into_str_literal() {
            let str_literal = &ctx.arenas.text[str_literal.0.start..str_literal.0.end];

            let mut at = 0;
            while let Some(next) = str_literal[at..].find('%') {
                format_string_content.push_str(&str_literal[at..at + next]);

                let mut remaining = &str_literal[at + next + 1..];
                at += next + 1;

                if remaining.starts_with('%') {
                    format_string_content.push('%');
                    at += 1;
                    continue;
                }

                required_arguments_left += 1;

                let nums_start_with_zero = remaining.starts_with('0');
                let mut pad_size = None;
                while remaining.starts_with(|c: char| c.is_ascii_digit()) {
                    if pad_size.is_some() || !remaining.starts_with('0') {
                        let p = pad_size.get_or_insert(0);
                        *p *= 10u32;
                        *p += (remaining.as_bytes()[0] - b'0') as u32;
                    }

                    at += 1;
                    remaining = &remaining[1..];
                }

                let padding = if let Some(pad_size) = pad_size {
                    Padding::ZeroPaddedTo(pad_size)
                } else if nums_start_with_zero {
                    Padding::NoPadding
                } else {
                    Padding::ZeroPaddedToSize
                };

                let Some(b) = remaining.as_bytes().first() else {
                    format_string_arguments
                        .push((format_string_content.len(), DynFormatArgument::default()));
                    continue;
                };

                at += usize::from(matches!(
                    b,
                    b'h' | b'H' |
                            b'x' | b'X' | // @NOTE: Not in spec: but used by Icarus Verilog
                            b'd' | b'D' |
                            b'o' | b'O' |
                            b'b' | b'B' |
                            b'c' | b'C' |
                            b'l' | b'L' |
                            b'v' | b'V' |
                            b'm' | b'M' |
                            b's' | b'S' |
                            b't' | b'T' |
                            b'u' | b'U' |
                            b'z' | b'Z'
                ));

                // @TODO: Make this actually impact formatting.
                let base = match b {
                    b'h' | b'H' => Base::Hexadecimal,
                    b'x' | b'X' => Base::Hexadecimal, // @NOTE: Not in spec: but used by Icarus Verilog
                    b'd' | b'D' => Base::Decimal,
                    b'o' | b'O' => Base::Octal,
                    b'b' | b'B' => Base::Binary,
                    b't' | b'T' => {
                        mctx.diagnostics.warn_not_yet_implemented(
                            ctx.arenas.get_span(expr),
                            "format specifier redirect to Decimal",
                        );
                        Base::Decimal
                    }
                    b'c' | b'C' | b'l' | b'L' | b'v' | b'V' | b'm' | b'M' | b's' | b'S' | b'u'
                    | b'U' | b'z' | b'Z' => {
                        mctx.diagnostics.not_yet_implemented(
                            ctx.arenas.get_span(expr),
                            "format specifier not yet supported",
                        );
                        return Err(());
                    }
                    _ => Base::Decimal,
                };

                format_string_arguments.push((
                    format_string_content.len(),
                    DynFormatArgument {
                        padding,
                        base,
                        signed: false,
                        prefix: false,
                    },
                ));
            }
            format_string_content.push_str(&str_literal[at..]);
        } else {
            let (var, var_ty) = lower_expr(ctx, mctx, scope, builder, expr, None)?;
            if required_arguments_left == 0 {
                format_string_arguments.push((
                    format_string_content.len(),
                    DynFormatArgument {
                        padding: Padding::default(),
                        base: Base::Decimal,
                        signed: var_ty.is_signed(),
                        prefix: false,
                    },
                ));
            } else {
                required_arguments_left -= 1;
                let Some((_, dyn_fmt)) = format_string_arguments.get_mut(format_string_args.len())
                else {
                    mctx.diagnostics.not_yet_implemented(
                        ctx.arenas.get_span(system_task_enable),
                        "extra arguments",
                    );
                    return Err(());
                };
                dyn_fmt.signed = var_ty.is_signed();
            }
            format_string_args.push(var);
        }
    }
    if required_arguments_left > 0 {
        mctx.diagnostics.not_yet_implemented(
            ctx.arenas.get_span(system_task_enable),
            "missing or extra arguments",
        );
        return Err(());
    }
    Ok((
        format_string_content,
        format_string_arguments,
        format_string_args,
    ))
}

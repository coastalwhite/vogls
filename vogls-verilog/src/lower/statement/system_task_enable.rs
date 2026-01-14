use vogls_ir::dyn_format_string::{Base, DynFormatArgument, DynFormatString, Padding};
use vogls_ir::{BasicBlockBuilder, GlobalContext, IntrinsicOp};

use crate::ast::AstId;
use crate::ast::statement::SystemTaskEnable;
use crate::lower::Scope;
use crate::lower::expression::lower_expr;
use crate::lower::{Diagnostics, expression};
use crate::parser::AstArenas;

pub fn lower_system_task_enable<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,
    scope: &mut Scope<'a>,
    diagnostics: &mut Diagnostics,
    mut builder: BasicBlockBuilder,
    system_task_enable: AstId<SystemTaskEnable>,
) -> Result<BasicBlockBuilder, ()> {
    let SystemTaskEnable {
        system_task_identifier,
        expressions,
    } = arenas.get(system_task_enable);
    let ident = arenas.get_ident(system_task_identifier.item.0);

    match ident {
        "display" => {
            let mut format_string_content = String::new();
            let mut format_string_arguments = Vec::new();
            let mut format_string_args = Vec::new();
            let mut required_arguments_left = 0;
            for expr in expressions.iter() {
                if let Some(str_literal) = arenas.get(expr).into_str_literal() {
                    let str_literal = &arenas.text[str_literal.0.start..str_literal.0.end];

                    let mut at = 0;
                    while let Some(next) = str_literal[at..].find('%') {
                        format_string_content.push_str(&str_literal[at..at + next]);

                        let mut remaining = &str_literal[at + next + 1..];
                        at += next + 1;

                        if remaining.starts_with('%') {
                            format_string_content.push('%');
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
                            b'c' | b'C' | b'l' | b'L' | b'v' | b'V' | b'm' | b'M' | b's' | b'S'
                            | b't' | b'T' | b'u' | b'U' | b'z' | b'Z' => {
                                diagnostics.not_yet_implemented(
                                    arenas.get_span(expr),
                                    "format specifier not yet supported",
                                );
                                return Err(());
                            }
                            _ => Base::Decimal,
                        };

                        format_string_arguments.push((
                            format_string_content.len(),
                            DynFormatArgument { padding, base },
                        ));
                    }
                    format_string_content.push_str(&str_literal[at..]);
                } else {
                    let (var, _) = lower_expr(gl, arenas, scope, diagnostics, &mut builder, expr)?;
                    if required_arguments_left == 0 {
                        format_string_arguments
                            .push((format_string_content.len(), DynFormatArgument::default()));
                    } else {
                        required_arguments_left -= 1;
                    }
                    format_string_args.push(var);
                }
            }
            if required_arguments_left > 0 {
                diagnostics.not_yet_implemented(
                    arenas.get_span(system_task_enable),
                    "missing or extra arguments",
                );
                return Err(());
            }
            use std::fmt::Write;
            writeln!(&mut format_string_content).unwrap();
            let format_str =
                DynFormatString::new(format_string_content.into(), format_string_arguments.into());
            builder.intrinsic(
                gl,
                IntrinsicOp::Display(Box::new(format_str)),
                format_string_args.into(),
            );
        }
        "vogls_assert_eq" | "vogls_assert_ne" => {
            if expressions.len() != 2 {
                diagnostics.not_yet_implemented(
                    arenas.get_span(system_task_enable),
                    "assertions requires two arguments",
                );
                return Err(());
            }

            let lhs = expressions.get(0);
            let rhs = expressions.get(1);

            let (lhs, lhs_ty) = lower_expr(gl, arenas, scope, diagnostics, &mut builder, lhs)?;
            let (rhs, rhs_ty) = lower_expr(gl, arenas, scope, diagnostics, &mut builder, rhs)?;

            let (lhs, _, rhs, _) =
                expression::coerce_bin_arithmetic(gl, &mut builder, lhs, lhs_ty, rhs, rhs_ty);
            let (condition, content) = if ident == "vogls_assert_eq" {
                (builder.equals(gl, lhs, rhs), "Assertion failed.  != \n")
            } else {
                (builder.not_equals(gl, lhs, rhs), "Assertion failed.  == \n")
            };
            let format_str = DynFormatString::new(
                content.into(),
                [
                    (18, DynFormatArgument::default()),
                    (22, DynFormatArgument::default()),
                ]
                .into(),
            );

            builder.intrinsic(
                gl,
                IntrinsicOp::Assert(Box::new(format_str)),
                [condition, lhs, rhs].into(),
            );
        }
        "finish" => _ = builder.intrinsic(gl, IntrinsicOp::Finish, Default::default()),

        "dumpfile" => {
            assert!(expressions.is_empty());
            builder.intrinsic(gl, IntrinsicOp::VcdOpenFile("dump.vcd".into()), [].into());
        }
        "dumpvars" => {
            assert!(expressions.is_empty());
            builder.intrinsic(
                gl,
                IntrinsicOp::VcdAppendModule(scope.vcd_scope()),
                [].into(),
            );
        }

        // @Incomplete: Many variants here.
        _ => {
            diagnostics.not_yet_implemented(
                arenas.get_span(system_task_enable),
                "system task not yet implemented",
            );
            return Err(());
        }
    }
    Ok(builder)
}

use crate::span::Span;
use crate::tokenizer::Token;

use super::ParseErrorReason;

#[derive(Default)]
pub struct Diagnostics {
    pub errors: Vec<(Span, ParseErrorReason)>,
}

impl Diagnostics {
    pub fn incomplete(&mut self, span: Span, reason: &'static str) {
        self.errors
            .push((span, ParseErrorReason::Incomplete(reason)));
    }

    pub fn unexpected_token(&mut self, span: Span, kind: Token) {
        self.errors
            .push((span, ParseErrorReason::UnexpectedToken(kind)));
    }
}

pub fn report_error(
    reason: ParseErrorReason,
    location: Span,
    path: &str,
    content: &str,
    out: &mut String,
) -> std::fmt::Result {
    use std::fmt::Write;
    writeln!(out, "Failed to read file. Reason: {:?}", reason)?;
    let lines = lines_with_offset(&content);
    let start_line = match lines.binary_search_by_key(&location.start(), |(offset, _)| *offset) {
        Ok(v) => v,
        Err(v) => v - 1,
    };
    let end_line = match lines.binary_search_by_key(&location.end(), |(offset, _)| *offset) {
        Ok(v) => v,
        Err(v) => v - 1,
    };

    const CTX_LINES: usize = 2;
    let ctx_start_line = start_line.saturating_sub(CTX_LINES);
    let ctx_end_line = end_line.saturating_add(1 + CTX_LINES).min(lines.len());

    writeln!(out, "[{path}:{}]:", ctx_start_line + 1)?;
    for line in ctx_start_line..start_line {
        let (_, line) = lines[line];
        writeln!(out, "| {line}")?;
    }

    if start_line == end_line {
        let (offset, line) = lines[start_line];
        writeln!(out, "> {line}")?;
        writeln!(
            out,
            "  {:start_pad$}{:len$}",
            "",
            "^",
            start_pad = location.start() - offset,
            len = location.len()
        )?;
    } else {
        let (offset, line) = lines[start_line];
        writeln!(out, "> {line}")?;
        writeln!(
            out,
            "  {:start_pad$}{:len$}",
            "",
            "^",
            start_pad = location.start() - offset,
            len = line.len() - location.start() - offset,
        )?;

        for line in start_line + 1..end_line {
            let (_, line) = lines[line];
            writeln!(out, "> {line}")?;
            writeln!(out, "  {:len$}", "^", len = line.len(),)?;
        }

        let (offset, line) = lines[end_line];
        writeln!(out, "> {line}")?;
        writeln!(out, "  {:len$}", "^", len = location.end() - offset,)?;
    }

    for line in end_line.saturating_add(1).min(ctx_end_line)..ctx_end_line {
        let (_, line) = lines[line];
        writeln!(out, "| {line}")?;
    }
    Ok(())
}

fn lines_with_offset(mut s: &str) -> Vec<(usize, &str)> {
    let original_length = s.len();
    let mut vs = Vec::new();
    while let Some(p) = s.find(['\n', '\r']) {
        if s.as_bytes()[p] == b'\r' {
            todo!();
        }

        let offset = original_length - s.len();
        vs.push((offset, &s[..p]));
        s = &s[p + 1..];
    }

    if !s.is_empty() {
        let offset = original_length - s.len();
        vs.push((offset, s));
    }

    vs
}

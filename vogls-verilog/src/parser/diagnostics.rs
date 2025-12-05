use crate::span::Span;
use crate::tokenizer::{Token, Tokenized};

use super::ParseErrorReason;
use super::token_walker::{TokenLoc, TokenRange};

#[derive(Default)]
pub struct Diagnostics {
    pub errors: Vec<(TokenRange, ParseErrorReason)>,
}

impl Diagnostics {
    pub fn incomplete(&mut self, tr: usize, reason: &'static str) {
        self.errors
            .push((TokenRange::at(tr), ParseErrorReason::Incomplete(reason)));
    }

    pub fn unexpected_token(&mut self, tr: usize, kind: Token) {
        self.errors
            .push((TokenRange::at(tr), ParseErrorReason::UnexpectedToken(kind)));
    }
}

pub fn report_error(
    tokenized: &Tokenized,
    reason: ParseErrorReason,
    location: TokenRange,
    out: &mut String,
) -> std::fmt::Result {
    use std::fmt::Write;
    writeln!(out, "Failed to read file. Reason: {:?}", reason)?;
    if tokenized.file_idxs[location.start] != tokenized.file_idxs[location.end] {
        // @TODO
        return Ok(());
    }

    let content = tokenized.contents[tokenized.file_idxs[location.start] as usize].as_ref();

    // @Performance: Cache lines per file.
    let lines = lines_with_offset(&content);
    let tok_start = tokenized.spans[location.start];
    let start_line = match lines.binary_search_by_key(&tok_start.start(), |(offset, _)| *offset) {
        Ok(v) => v,
        Err(v) => v - 1,
    };
    let tok_end = tokenized.spans[location.end];
    let end_line = match lines.binary_search_by_key(&tok_end.end(), |(offset, _)| *offset) {
        Ok(v) => v,
        Err(v) => v - 1,
    };

    const CTX_LINES: usize = 2;
    let ctx_start_line = start_line.saturating_sub(CTX_LINES);
    let ctx_end_line = end_line.saturating_add(1 + CTX_LINES).min(lines.len());

    let path = &tokenized.paths[tokenized.file_idxs[location.start] as usize];
    let path = match path {
        None => "<unknown>".to_string(),
        Some(p) => p.as_ref().display().to_string(),
    };
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
            start_pad = tok_start.start() - offset,
            len = tok_end.end() - tok_start.start()
        )?;
    } else {
        let (offset, line) = lines[start_line];
        writeln!(out, "> {line}")?;
        writeln!(
            out,
            "  {:start_pad$}{:len$}",
            "",
            "^",
            start_pad = tok_start.start() - offset,
            len = line.len() - tok_start.start() - offset,
        )?;

        for line in start_line + 1..end_line {
            let (_, line) = lines[line];
            writeln!(out, "> {line}")?;
            writeln!(out, "  {:len$}", "^", len = line.len(),)?;
        }

        let (offset, line) = lines[end_line];
        writeln!(out, "> {line}")?;
        writeln!(out, "  {:len$}", "^", len = tok_end.end() - offset,)?;
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

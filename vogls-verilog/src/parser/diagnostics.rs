use std::fmt::{self, Write};

use crate::span::Span;
use crate::tokenizer::{Token, Tokenized};

use super::ParseErrorReason;
use super::token_walker::TokenRange;

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

pub fn display_width(mut s: &str, tab_width: usize) -> usize {
    // @TODO: use unicode_width
    let mut n = 0;
    while let Some(i) = s.find('\t') {
        n += i + tab_width;
        s = &s[i + 1..];
    }
    n + s.len()
}

pub fn display_with_tab_width(mut s: &str, f: &mut String, tab_width: usize) -> fmt::Result {
    // @TODO: This is horrible.
    while let Some(i) = s.find('\t') {
        f.write_str(&s[..i])?;
        for _ in 0..tab_width {
            f.write_char(' ')?;
        }
        s = &s[i + 1..];
    }
    f.write_str(&s)?;
    Ok(())
}

pub fn report_error(
    tokenized: &Tokenized,
    reason: ParseErrorReason,
    location: TokenRange,
    out: &mut String,
) -> std::fmt::Result {
    use std::fmt::Write;
    writeln!(out, "Failed to read file. Reason: {:?}", reason)?;
    if tokenized.file_idxs[location.start] != tokenized.file_idxs[location.end - 1] {
        // @TODO
        return Ok(());
    }

    let path = &tokenized.paths[tokenized.file_idxs[location.start] as usize];
    let content = tokenized.contents[tokenized.file_idxs[location.start] as usize].as_ref();
    let location = Span::new(
        tokenized.spans[location.start].start(),
        tokenized.spans[location.end - 1].end(),
    );

    // @Performance: Cache lines per file.
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
    const TAB_WIDTH: usize = 4;
    let ctx_start_line = start_line.saturating_sub(CTX_LINES);
    let ctx_end_line = end_line.saturating_add(1 + CTX_LINES).min(lines.len());

    let path = match path {
        None => "<unknown>".to_string(),
        Some(p) => p.as_ref().display().to_string(),
    };
    writeln!(out, "[{path}:{}]:", ctx_start_line + 1)?;
    for line in ctx_start_line..start_line {
        let (_, line) = lines[line];
        out.write_str("| ")?;
        display_with_tab_width(line, out, TAB_WIDTH)?;
        writeln!(out)?;
    }

    if start_line == end_line {
        let (offset, line) = lines[start_line];
        out.write_str("> ")?;
        display_with_tab_width(line, out, TAB_WIDTH)?;
        writeln!(out)?;
        writeln!(
            out,
            "  {:start_pad$}{:len$}",
            "",
            "^",
            start_pad = display_width(&line[..location.start() - offset], TAB_WIDTH),
            len = location.len()
        )?;
    } else {
        let (offset, line) = lines[start_line];
        out.write_str("> ")?;
        display_with_tab_width(line, out, TAB_WIDTH)?;
        writeln!(out)?;
        writeln!(
            out,
            "  {:start_pad$}{:len$}",
            "",
            "^",
            start_pad = display_width(&line[..location.start() - offset], TAB_WIDTH),
            len = line.len() - (location.start() - offset),
        )?;

        for line in start_line + 1..end_line {
            let (_, line) = lines[line];
            out.write_str("> ")?;
            display_with_tab_width(line, out, TAB_WIDTH)?;
            writeln!(out)?;
            writeln!(out, "  {:len$}", "^", len = display_width(line, TAB_WIDTH))?;
        }

        let (offset, line) = lines[end_line];
        out.write_str("> ")?;
        display_with_tab_width(line, out, TAB_WIDTH)?;
        writeln!(out)?;
        writeln!(
            out,
            "  {:len$}",
            "^",
            len = display_width(&line[..location.end() - offset], TAB_WIDTH)
        )?;
    }

    for line in end_line.saturating_add(1).min(ctx_end_line)..ctx_end_line {
        let (_, line) = lines[line];
        out.write_str("| ")?;
        display_with_tab_width(line, out, TAB_WIDTH)?;
        writeln!(out)?;
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

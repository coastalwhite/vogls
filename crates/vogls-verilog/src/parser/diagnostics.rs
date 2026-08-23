use std::fmt::{self, Write};

use vogls_ir::token_range::TokenRange;

use crate::span::Span;
use crate::tokenizer::{Token, Tokenized};

use super::ParseErrorReason;

#[derive(Default)]
pub struct Diagnostics {
    pub errors: Vec<(TokenRange, ParseErrorReason)>,
}

impl Diagnostics {
    pub fn missing_token(&mut self, tr: usize) {
        self.errors
            .push((TokenRange::at(tr), ParseErrorReason::MissingToken));
    }

    pub fn incomplete(&mut self, tr: usize, reason: &'static str) {
        self.errors
            .push((TokenRange::at(tr), ParseErrorReason::Incomplete(reason)));
    }

    pub fn unexpected_token(&mut self, tr: usize, kind: Token) {
        self.errors
            .push((TokenRange::at(tr), ParseErrorReason::UnexpectedToken(kind)));
    }

    pub fn no_corresponding(&mut self, tr: usize, kind: Token) {
        self.errors
            .push((TokenRange::at(tr), ParseErrorReason::NoCorresponding(kind)));
    }

    pub fn not_found(&mut self, tr: TokenRange, kind: Token) {
        self.errors.push((tr, ParseErrorReason::NotFound(kind)));
    }

    pub fn leftover_tokens(&mut self, tr: TokenRange) {
        self.errors.push((tr, ParseErrorReason::LeftoverTokens));
    }
}

pub fn report_error(
    tokenized: &Tokenized,
    reason: impl fmt::Debug,
    location: TokenRange,
    out: &mut String,
) -> std::fmt::Result {
    writeln!(out, "Failed to read file. Reason: {:?}", reason)?;
    report(tokenized, location, out)
}

pub fn report(tokenized: &Tokenized, location: TokenRange, out: &mut String) -> std::fmt::Result {
    use std::fmt::Write;
    if location.start == 0 && location.end == 0 {
        return Ok(());
    }

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
    let lines = lines_with_offset(content);
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

        let start_pad = display_width(&line[..location.start() - offset], TAB_WIDTH);
        let len = display_width(&content[location.as_range()], TAB_WIDTH);
        writeln!(out)?;
        writeln!(out, "  {:start_pad$}{:^>len$}", "", "")?;
    } else {
        let (offset, line) = lines[start_line];
        out.write_str("> ")?;
        display_with_tab_width(line, out, TAB_WIDTH)?;
        let start_pad = display_width(&line[..location.start() - offset], TAB_WIDTH);
        let len = line.len() - (location.start() - offset);
        writeln!(out)?;
        writeln!(out, "  {:start_pad$}{:^>len$}", "", "",)?;

        for line in start_line + 1..end_line {
            let (_, line) = lines[line];
            out.write_str("> ")?;
            display_with_tab_width(line, out, TAB_WIDTH)?;
            let start_pad = display_width(&line[..line.len() - line.trim_start().len()], TAB_WIDTH);
            let len = display_width(line, TAB_WIDTH) - start_pad;
            writeln!(out)?;
            writeln!(out, "  {:start_pad$}{:^>len$}", "", "")?;
        }

        let (offset, line) = lines[end_line];
        out.write_str("> ")?;
        display_with_tab_width(line, out, TAB_WIDTH)?;
        let start_pad = display_width(&line[..line.len() - line.trim_start().len()], TAB_WIDTH);
        let len = display_width(&line[..location.end() - offset], TAB_WIDTH) - start_pad;
        writeln!(out)?;
        writeln!(out, "  {:start_pad$}{:^>len$}", " ", " ")?;
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
    f.write_str(s)?;
    Ok(())
}

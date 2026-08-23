use std::fmt::{self, Write};
use std::path::Path;
use std::sync::Arc;

use vogls_ir::token_range::TokenRange;

use crate::span::Span;
use crate::tokenizer::Token;

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

pub struct SpanError<'a, T> {
    pub spans: &'a [Span],
    pub file_idxs: &'a [u32],

    pub paths: &'a [Option<Arc<Path>>],
    pub contents: &'a [Arc<str>],

    pub error: T,
    pub kind: ReportKind,
    pub code: Option<u32>,
    pub location: TokenRange,
}

#[derive(Clone, Copy)]
pub enum ReportKind {
    Error,
    Warning,
    Info,
}

const CTX_LINES: usize = 2;
const TAB_WIDTH: usize = 4;

impl<'a, T: fmt::Display> fmt::Display for SpanError<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file_idx = self.file_idxs[self.location.start];
        let path = self.paths[file_idx as usize].as_deref();
        let content = self.contents[file_idx as usize].as_ref();

        // @Performance: Cache lines per file.
        let lines = lines_with_offset(content);
        let start_line = match lines
            .binary_search_by_key(&self.spans[self.location.start].start(), |(offset, _)| {
                *offset
            }) {
            Ok(v) => v,
            Err(v) => v - 1,
        };

        let path = match path {
            None => "<unknown>".to_string(),
            Some(p) => p.display().to_string(),
        };
        let kind = match self.kind {
            ReportKind::Error => "error",
            ReportKind::Warning => "warning",
            ReportKind::Info => "info",
        };
        f.write_str(kind)?;
        if let Some(code) = self.code {
            write!(f, "[{code}]")?;
        }
        f.write_str(": ")?;
        writeln!(f.with_tab_width(), "{}", self.error)?;

        // TokenSpan across single file.
        if !self.location.is_empty() && file_idx == self.file_idxs[self.location.end - 1] {
            let file_span = Span::new(
                self.spans[self.location.start].start(),
                self.spans[self.location.end - 1].end(),
            );

            let end_line = match lines.binary_search_by_key(&file_span.end(), |(offset, _)| *offset)
            {
                Ok(v) => v,
                Err(v) => v - 1,
            };

            let ctx_start_line = start_line.saturating_sub(CTX_LINES);
            let ctx_end_line = end_line.saturating_add(1 + CTX_LINES).min(lines.len());

            let max_number_width = ceil_ilog10(ctx_end_line + 1) as usize;

            writeln!(f, "{:max_number_width$} --> {path}:{}", "", start_line + 1)?;
            for line_nr in ctx_start_line..start_line {
                let (_, line) = lines[line_nr];
                write!(f, " {:>max_number_width$}", line_nr + 1)?;
                f.write_str(" | ")?;
                f.with_tab_width().write_str(line)?;
                writeln!(f)?;
            }
            if start_line == end_line {
                let (offset, line) = lines[start_line];
                write!(f, " {:>max_number_width$}", start_line + 1)?;
                f.write_str(" > ")?;
                f.with_tab_width().write_str(line)?;
                writeln!(f)?;

                let start_pad = display_width(&line[..file_span.start() - offset], TAB_WIDTH);
                let len = display_width(&content[file_span.as_range()], TAB_WIDTH);
                writeln!(
                    f,
                    "{:max_number_width$}    {:start_pad$}{:^>len$}",
                    "", "", ""
                )?;
            } else {
                let (offset, line) = lines[start_line];
                write!(f, " {:>max_number_width$}", start_line + 1)?;
                f.write_str(" > ")?;
                f.with_tab_width().write_str(line)?;
                let start_pad = display_width(&line[..file_span.start() - offset], TAB_WIDTH);
                let len = line.len() - (file_span.start() - offset);
                writeln!(f)?;
                writeln!(
                    f,
                    "{:max_number_width$}    {:start_pad$}{:^>len$}",
                    "", "", "",
                )?;

                for line_nr in start_line + 1..end_line {
                    let (_, line) = lines[line_nr];
                    write!(f, " {:>max_number_width$}", line_nr + 1)?;
                    f.write_str(" > ")?;
                    f.with_tab_width().write_str(line)?;
                    let start_pad =
                        display_width(&line[..line.len() - line.trim_start().len()], TAB_WIDTH);
                    let len = display_width(line, TAB_WIDTH) - start_pad;
                    writeln!(f)?;
                    writeln!(
                        f,
                        "{:max_number_width$}    {:start_pad$}{:^>len$}",
                        "", "", ""
                    )?;
                }

                let (offset, line) = lines[end_line];
                write!(f, " {:>max_number_width$}", end_line + 1)?;
                f.write_str(" > ")?;
                f.with_tab_width().write_str(line)?;
                let start_pad =
                    display_width(&line[..line.len() - line.trim_start().len()], TAB_WIDTH);
                let len = display_width(&line[..file_span.end() - offset], TAB_WIDTH) - start_pad;
                writeln!(f)?;
                writeln!(
                    f,
                    "{:max_number_width$}    {:start_pad$}{:^>len$}",
                    "", "", ""
                )?;
            }

            for line_nr in end_line.saturating_add(1).min(ctx_end_line)..ctx_end_line {
                let (_, line) = lines[line_nr];
                write!(f, " {:>max_number_width$}", line_nr + 1)?;
                f.write_str(" | ")?;
                f.with_tab_width().write_str(line)?;
                writeln!(f)?;
            }
        } else {
            writeln!(f, "  --> {path}:{}", start_line + 1)?;
        }

        Ok(())
    }
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

trait IntoDisplay: Sized {
    fn with_tab_width<'a>(&'a mut self) -> DisplayWithTabWidth<'a, Self>;
}

impl<T: fmt::Write> IntoDisplay for T {
    fn with_tab_width<'a>(&'a mut self) -> DisplayWithTabWidth<'a, Self> {
        DisplayWithTabWidth {
            writer: self,
            tab_width: TAB_WIDTH,
        }
    }
}

struct DisplayWithTabWidth<'a, W> {
    writer: &'a mut W,
    tab_width: usize,
}

impl<'a, W: fmt::Write> fmt::Write for DisplayWithTabWidth<'a, W> {
    fn write_str(&mut self, mut s: &str) -> fmt::Result {
        while let Some(split) = s.split_once('\t') {
            let prefix;
            (prefix, s) = split;

            self.writer.write_str(prefix)?;
            for _ in 0..self.tab_width {
                self.writer.write_char(' ')?;
            }
        }
        self.writer.write_str(s)
    }
}

fn ceil_ilog10(n: usize) -> u32 {
    match n {
        0 => panic!("ceil_ilog10(0) is undefined"),
        1 => 0,
        n => (n - 1).ilog10() + 1,
    }
}

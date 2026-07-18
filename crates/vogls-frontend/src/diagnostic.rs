use std::borrow::Cow;
use std::fmt;
use std::ops::Range;
use std::path::Path;

pub struct Diagnostic<'a, T> {
    content: &'a str,
    file: Option<&'a Path>,
    line_offsets: Cow<'a, [usize]>,
    start: usize,
    end: usize,
    message: T,
}

impl<'a, T> Diagnostic<'a, T> {
    pub fn new_infer_lines(
        content: &'a str,
        file: Option<&'a Path>,
        message: T,
        span: Range<usize>,
    ) -> Self {
        let line_offsets = lines_with_offset(content);
        Self {
            content,
            file,
            line_offsets: line_offsets.into(),
            start: span.start,
            end: span.end,
            message,
        }
    }

    pub fn new(
        content: &'a str,
        file: Option<&'a Path>,
        line_offsets: &'a [usize],
        message: T,
        span: Range<usize>,
    ) -> Self {
        Self {
            content,
            file,
            line_offsets: Cow::Borrowed(line_offsets),
            start: span.start,
            end: span.end,
            message,
        }
    }
}

fn get_display_line<'a>(content: &'a str, line_offsets: &[usize], line: usize) -> &'a str {
    content[line_offsets[line]..line_offsets[line + 1]].trim_end()
}

impl<'a, T: fmt::Display> fmt::Display for Diagnostic<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        let start_line = match self.line_offsets.binary_search(&self.start) {
            Ok(v) => v,
            Err(v) => v - 1,
        };
        let end_line = match self.line_offsets.binary_search(&self.end) {
            Ok(v) => v,
            Err(v) => v - 1,
        };

        const CTX_LINES: usize = 2;
        const TAB_WIDTH: usize = 4;
        let ctx_start_line = start_line.saturating_sub(CTX_LINES);
        let ctx_end_line = end_line
            .saturating_add(1 + CTX_LINES)
            .min(self.line_offsets.len() - 1);

        f.write_str("[")?;
        match self.file {
            None => f.write_str("<unknown>"),
            Some(p) => p.display().fmt(f),
        }?;
        writeln!(f, ":{}]: {}", ctx_start_line + 1, self.message)?;
        for line in ctx_start_line..start_line {
            let line = get_display_line(self.content, &self.line_offsets, line);
            f.write_str("| ")?;
            display_with_tab_width(line, f, TAB_WIDTH)?;
            writeln!(f)?;
        }

        if start_line == end_line {
            let line = get_display_line(self.content, &self.line_offsets, start_line);
            f.write_str("> ")?;
            display_with_tab_width(line, f, TAB_WIDTH)?;

            let start_pad = display_width(
                &line[..self.start - self.line_offsets[start_line]],
                TAB_WIDTH,
            );
            let len = display_width(&self.content[self.start..self.end], TAB_WIDTH);
            writeln!(f)?;
            writeln!(f, "  {:start_pad$}{:^>len$}", " ", " ",)?;
        } else {
            let offset = self.line_offsets[start_line];
            let line = get_display_line(self.content, &self.line_offsets, start_line);
            f.write_str("> ")?;
            display_with_tab_width(line, f, TAB_WIDTH)?;
            let start_pad = display_width(&line[..self.start - offset], TAB_WIDTH);
            let len = line.len() - (self.start - offset);
            writeln!(f)?;
            writeln!(f, "  {:start_pad$}{:^>len$}", "", "^",)?;

            for line in start_line + 1..end_line {
                let line = get_display_line(self.content, &self.line_offsets, line);
                f.write_str("> ")?;
                display_with_tab_width(line, f, TAB_WIDTH)?;
                let start_pad =
                    display_width(&line[..line.len() - line.trim_start().len()], TAB_WIDTH);
                let len = display_width(line, TAB_WIDTH) - start_pad;
                writeln!(f)?;
                writeln!(f, "  {:start_pad$}{:^>len$}", "", "")?;
            }

            let offset = self.line_offsets[end_line];
            let line = get_display_line(self.content, &self.line_offsets, end_line);
            f.write_str("> ")?;
            display_with_tab_width(line, f, TAB_WIDTH)?;
            let start_pad = display_width(&line[..line.len() - line.trim_start().len()], TAB_WIDTH);
            let len = display_width(&line[..self.end - offset], TAB_WIDTH) - start_pad;
            writeln!(f)?;
            writeln!(f, "  {:start_pad$}{:^>len$}", " ", " ")?;
        }

        for line in end_line.saturating_add(1).min(ctx_end_line)..ctx_end_line {
            let line = get_display_line(self.content, &self.line_offsets, line);
            f.write_str("| ")?;
            display_with_tab_width(line, f, TAB_WIDTH)?;
            writeln!(f)?;
        }
        Ok(())
    }
}

fn lines_with_offset(mut s: &str) -> Vec<usize> {
    let original_length = s.len();
    let mut vs = Vec::new();

    vs.push(0);
    while let Some(p) = s.find(['\n']) {
        let offset = original_length - s.len();
        vs.push(offset);
        s = &s[p + 1..];
    }

    if !s.is_empty() {
        vs.push(original_length);
    }

    vs
}

fn display_width(mut s: &str, tab_width: usize) -> usize {
    // @TODO: use unicode_width
    let mut n = 0;
    while let Some(i) = s.find('\t') {
        n += i + tab_width;
        s = &s[i + 1..];
    }
    n + s.len()
}

fn display_with_tab_width(
    mut s: &str,
    f: &mut fmt::Formatter<'_>,
    tab_width: usize,
) -> fmt::Result {
    // @TODO: This is horrible.
    while let Some(i) = s.find('\t') {
        f.write_str(&s[..i])?;
        for _ in 0..tab_width {
            f.write_str(" ")?;
        }
        s = &s[i + 1..];
    }
    f.write_str(s)?;
    Ok(())
}

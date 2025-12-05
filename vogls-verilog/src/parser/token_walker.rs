use std::path::Path;
use std::rc::Rc;

use crate::span::Span;
use crate::tokenizer::{FileIdx, Token, Tokenized};

use super::{Diagnostics, ParseErrorKind, ParseErrorReason};

pub struct TokenWalker<'a> {
    tokens: &'a [Token],
    spans: &'a [Span],
    file_idxs: &'a [FileIdx],

    contents: &'a [Rc<str>],
    paths: &'a [Option<Rc<Path>>],

    /// Index of the next token.
    pub offset: usize,
}

#[derive(Debug)]
pub struct TokenLoc<'a> {
    pub kind: &'a Token,
    pub span: &'a Span,
    pub file: &'a FileIdx,
}

impl<'a> TokenWalker<'a> {
    pub fn new(buffer: &'a Tokenized) -> Self {
        Self {
            tokens: &buffer.tokens,
            spans: &buffer.spans,
            file_idxs: &buffer.file_idxs,
            contents: &buffer.contents,
            paths: &buffer.paths,
            offset: 0,
        }
    }

    pub fn content(&self, file: FileIdx) -> &str {
        &self.contents[file as usize]
    }

    pub fn path(&self, file: FileIdx) -> Option<&Path> {
        self.paths[file as usize].as_deref()
    }

    pub fn is_empty(&self) -> bool {
        self.offset >= self.tokens.len()
    }

    pub fn cursor_offset(&self) -> usize {
        if self.offset == 0 {
            0
        } else {
            self.spans[self.offset - 1].end()
        }
    }

    pub fn span_at_cursor(&self) -> Span {
        let cursor = self.cursor_offset();
        Span::new(cursor, cursor)
    }

    pub fn get(&self, i: usize) -> Option<TokenLoc<'_>> {
        if i >= self.tokens.len() {
            return None;
        }

        Some(TokenLoc {
            kind: &self.tokens[i],
            span: &self.spans[i],
            file: &self.file_idxs[i],
        })
    }

    pub fn try_get(
        &self,
        i: usize,
        diagnostics: Option<&mut Diagnostics>,
    ) -> Result<TokenLoc<'_>, ParseErrorKind> {
        match self.get(i) {
            Some(t) => Ok(t),
            None => {
                if let Some(diagnostics) = diagnostics {
                    diagnostics
                        .errors
                        .push((self.span_at_cursor(), ParseErrorReason::MissingToken));
                }
                Err(ParseErrorKind::MissingToken)
            }
        }
    }

    pub fn next_if_equals(&mut self, kind: Token) -> bool {
        let Some(next) = self.next() else {
            return false;
        };
        let next = *next.kind;
        self.offset -= usize::from(next != kind);
        next == kind
    }

    pub fn next(&mut self) -> Option<TokenLoc<'_>> {
        if self.is_empty() {
            return None;
        }

        self.offset += 1;
        self.get(self.offset - 1)
    }

    pub fn try_next(
        &mut self,
        diagnostics: Option<&mut Diagnostics>,
    ) -> Result<TokenLoc<'_>, ParseErrorKind> {
        if self.is_empty() {
            if let Some(diagnostics) = diagnostics {
                diagnostics
                    .errors
                    .push((self.span_at_cursor(), ParseErrorReason::MissingToken));
            }
            return Err(ParseErrorKind::MissingToken);
        }

        self.offset += 1;
        Ok(self.get(self.offset - 1).unwrap())
    }

    pub fn next_back(&mut self) -> Option<TokenLoc<'_>> {
        if self.offset == 0 {
            return None;
        }

        self.offset -= 1;
        self.get(self.offset)
    }

    pub fn next_expect(
        &mut self,
        kind: Token,
        mut diagnostics: Option<&mut Diagnostics>,
    ) -> Result<TokenLoc<'_>, ParseErrorKind> {
        let next = self.try_next(diagnostics.as_deref_mut())?;
        if *next.kind != kind {
            if let Some(diagnostics) = diagnostics {
                diagnostics
                    .errors
                    .push((*next.span, ParseErrorReason::UnexpectedToken(*next.kind)));
            }
            return Err(ParseErrorKind::UnexpectedToken);
        }
        Ok(next)
    }

    pub(crate) fn peek_content(&self) -> &str {
        let t = self.get(self.offset).unwrap();
        &self.content(*t.file)[t.span.start()..]
    }
}

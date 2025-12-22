use std::path::Path;
use std::rc::Rc;

use crate::span::Span;
use crate::tokenizer::{FileIdx, Token, Tokenized};

use super::{Diagnostics, ParseErrorReason};

#[derive(Clone)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TokenRange {
    pub start: usize,
    pub end: usize,
}
impl TokenRange {
    pub fn at(tr: usize) -> TokenRange {
        TokenRange {
            start: tr,
            end: tr + 1,
        }
    }
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
    ) -> Result<TokenLoc<'_>, ()> {
        match self.get(i) {
            Some(t) => Ok(t),
            None => {
                if let Some(diagnostics) = diagnostics {
                    diagnostics
                        .errors
                        .push((TokenRange::at(i), ParseErrorReason::MissingToken));
                }
                Err(())
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

    pub fn is_next_equal_to(&self, kind: Token) -> bool {
        let Some(next) = self.get(self.offset) else {
            return false;
        };
        let next = *next.kind;
        next == kind
    }
    pub fn is_next_nth_equal_to(&self, nth: usize, kind: Token) -> bool {
        let Some(next) = self.get(self.offset + nth) else {
            return false;
        };
        let next = *next.kind;
        next == kind
    }

    pub fn next(&mut self) -> Option<TokenLoc<'_>> {
        if self.is_empty() {
            return None;
        }

        self.offset += 1;
        self.get(self.offset - 1)
    }

    pub fn try_next(&mut self, diagnostics: Option<&mut Diagnostics>) -> Result<TokenLoc<'_>, ()> {
        if self.is_empty() {
            if let Some(diagnostics) = diagnostics {
                diagnostics
                    .errors
                    .push((TokenRange::at(self.offset), ParseErrorReason::MissingToken));
            }
            return Err(());
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
    ) -> Result<TokenLoc<'_>, ()> {
        let offset = self.offset;
        let next = self.try_next(diagnostics.as_deref_mut())?;
        if *next.kind != kind {
            if let Some(diagnostics) = diagnostics {
                diagnostics.errors.push((
                    TokenRange::at(offset),
                    ParseErrorReason::UnexpectedToken(*next.kind),
                ));
            }
            return Err(());
        }
        Ok(next)
    }

    pub fn peek_content(&self) -> &str {
        let t = self.get(self.offset).unwrap();
        &self.content(*t.file)
            [t.span.start()..(t.span.start() + 10).min(self.content(*t.file).len())]
    }

    pub fn try_find_corresponding(
        &self,
        token: Token,
        corresponding: usize,
        diagnostics: Option<&mut Diagnostics>,
    ) -> Result<usize, ()> {
        match self.find(token) {
            None => {
                if let Some(diagnostics) = diagnostics {
                    diagnostics.errors.push((
                        TokenRange::at(corresponding),
                        ParseErrorReason::NoCorresponding(token),
                    ));
                }
                return Err(());
            }
            Some(i) => Ok(i),
        }
    }

    pub fn find(&self, token: Token) -> Option<usize> {
        Some(
            self.offset
                + self.tokens[self.offset..]
                    .iter()
                    .position(|t| *t == token)?,
        )
    }

    pub fn end_at(&self, at: usize) -> TokenWalker<'a> {
        Self {
            tokens: &self.tokens[..at],
            spans: &self.spans[..at],
            file_idxs: &self.file_idxs[..at],
            contents: &self.contents,
            paths: &self.paths,
            offset: self.offset,
        }
    }

    pub fn try_find_corresponding_balanced(&self, offset: usize) -> usize {
        use Token as T;

        let corresponding = match self.get(offset).unwrap().kind {
            T::LeftParen => T::RightParen,
            T::LeftBrace => T::RightBrace,
            T::LeftBracket => T::RightBracket,
            _ => unreachable!(),
        };

        let mut lparens = 0;
        let mut lbraces = 0;
        let mut lbrackets = 0;

        for (i, t) in self.tokens.iter().enumerate().skip(offset) {
            match *t {
                T::LeftParen => lparens += 1,
                T::RightParen => {
                    lparens -= 1;
                    if corresponding == T::RightParen && lparens == 0 {
                        // @TODO: better error message
                        assert!(lparens == 0 && lbraces == 0 && lbrackets == 0);
                        return i;
                    }
                }
                T::LeftBrace => lbraces += 1,
                T::RightBrace => {
                    lbraces -= 1;
                    if corresponding == T::RightBrace && lbraces == 0 {
                        // @TODO: better error message
                        assert!(lparens == 0 && lbraces == 0 && lbrackets == 0);
                        return i;
                    }
                }
                T::LeftBracket => lbrackets += 1,
                T::RightBracket => {
                    lbrackets -= 1;
                    if corresponding == T::RightBracket && lbrackets == 0 {
                        // @TODO: better error message
                        assert!(lparens == 0 && lbraces == 0 && lbrackets == 0);
                        return i;
                    }
                }
                _ => {}
            }
        }

        // @TODO: better error message
        panic!("no matching enclosure");
    }

    pub fn find_next_same_depth(&self, token: Token) -> Option<usize> {
        use Token as T;

        let mut lparens = 0i32;
        let mut lbraces = 0i32;
        let mut lbrackets = 0i32;

        for (i, &t) in self.tokens.iter().enumerate().skip(self.offset) {
            if t == token && lparens == 0 && lbraces == 0 && lbrackets == 0 {
                return Some(i);
            }

            #[rustfmt::skip]
            {
                if t == T::LeftParen    { lparens   += 1; }
                if t == T::RightParen   { lparens   -= 1; }
                if t == T::LeftBrace    { lbraces   += 1; }
                if t == T::RightBrace   { lbraces   -= 1; }
                if t == T::LeftBracket  { lbrackets += 1; }
                if t == T::RightBracket { lbrackets -= 1; }
            };
        }

        None
    }

    pub fn find_next_one_of_same_depth(&self, tokens: &[Token]) -> Option<usize> {
        use Token as T;

        let mut lparens = 0i32;
        let mut lbraces = 0i32;
        let mut lbrackets = 0i32;

        for (i, &t) in self.tokens.iter().enumerate().skip(self.offset) {
            if tokens.contains(&t) && lparens == 0 && lbraces == 0 && lbrackets == 0 {
                return Some(i);
            }

            #[rustfmt::skip]
            {
                if t == T::LeftParen    { lparens   += 1; }
                if t == T::RightParen   { lparens   -= 1; }
                if t == T::LeftBrace    { lbraces   += 1; }
                if t == T::RightBrace   { lbraces   -= 1; }
                if t == T::LeftBracket  { lbrackets += 1; }
                if t == T::RightBracket { lbrackets -= 1; }
            };
        }

        None
    }
}

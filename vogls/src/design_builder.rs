use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use std::{fmt, io};

use vogls_verilog::arena::Arena;
use vogls_verilog::parser::{
    AstArenas, Diagnostics, ParseContext, ParserScratches, TokenWalker, parse_file,
};
use vogls_verilog::tokenizer::{Macro, TokenizeError, Tokenized};

use crate::{ParseError, ParsedDesign};

#[derive(Default)]
pub struct DesignBuilder {
    pub(crate) token_buffer: Tokenized,
    pub(crate) macros: HashMap<String, Macro>,
}

#[derive(Debug)]
pub enum DesignBuilderError {
    Io(io::Error),
    Tokenize(Box<TokenizeError>),
}

impl fmt::Display for DesignBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DesignBuilderError::Io(err) => err.fmt(f),
            DesignBuilderError::Tokenize(err) => err.fmt(f),
        }
    }
}
impl std::error::Error for DesignBuilderError {}

impl From<Box<TokenizeError>> for DesignBuilderError {
    fn from(value: Box<TokenizeError>) -> Self {
        Self::Tokenize(value)
    }
}
impl From<io::Error> for DesignBuilderError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl DesignBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_source(&mut self, path: impl AsRef<Path>) -> Result<&mut Self, DesignBuilderError> {
        let path = path.as_ref();
        let source = std::fs::read_to_string(path)?;
        self.add_source_str_with_name(source, path)?;
        Ok(self)
    }

    pub fn add_source_str(
        &mut self,
        source: impl Into<Rc<str>>,
    ) -> Result<&mut Self, Box<TokenizeError>> {
        self.add_source_str_with_opt_name(source, <Option<Rc<Path>>>::None)
    }

    pub fn add_source_str_with_name(
        &mut self,
        source: impl Into<Rc<str>>,
        name: impl Into<Rc<Path>>,
    ) -> Result<&mut Self, Box<TokenizeError>> {
        self.add_source_str_with_opt_name(source, Some(name))?;
        Ok(self)
    }

    pub fn add_source_str_with_opt_name(
        &mut self,
        source: impl Into<Rc<str>>,
        name: Option<impl Into<Rc<Path>>>,
    ) -> Result<&mut Self, Box<TokenizeError>> {
        self.token_buffer.append_tokenize_with_macros(
            source.into(),
            name.map(Into::into),
            &mut self.macros,
        )?;
        Ok(self)
    }

    pub fn define_macro(&mut self, name: impl Into<String>, value: Macro) -> &mut Self {
        self.macros.insert(name.into(), value);
        self
    }

    pub fn parse<'a>(self, arena: &'a mut Arena) -> Result<ParsedDesign<'a>, ParseError> {
        let mut tkw = TokenWalker::new(&self.token_buffer);
        let mut arenas = AstArenas::default();

        let mut diagnostics = Diagnostics::default();
        let ast = parse_file(
            &mut tkw,
            &mut ParserScratches::default(),
            Some(&mut diagnostics),
            &mut arenas,
            arena,
            &mut ParseContext::new(),
        );
        let Ok(ast) = ast else {
            return Err(ParseError {
                builder: self,
                diagnostics,
            });
        };

        Ok(ParsedDesign {
            ast,
            token_buffer: self.token_buffer,
            arenas,
        })
    }
}

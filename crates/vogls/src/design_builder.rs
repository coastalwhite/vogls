use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fmt, io};

use vogls_verilog::arena::Arena;
use vogls_verilog::parser::{
    AstArenas, Diagnostics, ParseContext, ParserScratches, TokenWalker, parse_file,
};
use vogls_verilog::tokenizer::{Macro, Macros, TokenizeError, Tokenized};
#[cfg(feature = "stdworld")]
use vogls_world::std::StdWorld;
use vogls_world::{World, WorldError};

use crate::{ParseError, ParsedDesign};

#[derive(Default, Clone)]
pub struct DesignBuilder {
    pub(crate) token_buffer: Tokenized,
    pub(crate) macros: Macros,
    pub(crate) include_dirs: Vec<PathBuf>,
}

#[derive(Debug)]
pub enum DesignBuilderError {
    Io(io::Error),
    Tokenize(Box<TokenizeError>),
    Unknown,
}

impl fmt::Display for DesignBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DesignBuilderError::Io(err) => err.fmt(f),
            DesignBuilderError::Tokenize(err) => err.fmt(f),
            DesignBuilderError::Unknown => f.write_str("unknown error"),
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
impl From<WorldError> for DesignBuilderError {
    fn from(value: WorldError) -> Self {
        match value {
            WorldError::RecloseFile => Self::Unknown,
            WorldError::Io(error) => Self::Io(error),
        }
    }
}

impl DesignBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    fn add_source_str_in_world_with_opt_name(
        &mut self,
        world: &mut dyn World,
        source: impl Into<Arc<str>>,
        name: Option<impl Into<Arc<Path>>>,
    ) -> Result<&mut Self, Box<TokenizeError>> {
        self.token_buffer.append_tokenize_with_macros(
            source.into(),
            name.map(Into::into),
            world,
            &mut self.macros,
            &self.include_dirs,
        )?;
        Ok(self)
    }

    pub fn add_source_in_world(
        &mut self,
        world: &mut dyn World,
        path: impl AsRef<Path>,
    ) -> Result<&mut Self, DesignBuilderError> {
        let path = path.as_ref();
        let source = world.read_to_string(path)?;
        Ok(self.add_source_str_in_world_with_opt_name(world, source, Some(path))?)
    }

    pub fn add_source_str_in_world(
        &mut self,
        world: &mut dyn World,
        source: impl Into<Arc<str>>,
    ) -> Result<&mut Self, Box<TokenizeError>> {
        self.add_source_str_in_world_with_opt_name(world, source, <Option<Arc<Path>>>::None)
    }

    pub fn add_source_str_in_world_with_name(
        &mut self,
        world: &mut dyn World,
        source: impl Into<Arc<str>>,
        name: impl Into<Arc<Path>>,
    ) -> Result<&mut Self, Box<TokenizeError>> {
        self.add_source_str_in_world_with_opt_name(world, source, Some(name))
    }

    #[cfg(feature = "stdworld")]
    pub fn add_source(&mut self, path: impl AsRef<Path>) -> Result<&mut Self, DesignBuilderError> {
        self.add_source_in_world(&mut StdWorld::new(), path)
    }

    #[cfg(feature = "stdworld")]
    pub fn add_source_str(
        &mut self,
        source: impl Into<Arc<str>>,
    ) -> Result<&mut Self, Box<TokenizeError>> {
        self.add_source_str_in_world(&mut StdWorld::new(), source)
    }

    #[cfg(feature = "stdworld")]
    pub fn add_source_str_with_name(
        &mut self,
        source: impl Into<Arc<str>>,
        name: impl Into<Arc<Path>>,
    ) -> Result<&mut Self, Box<TokenizeError>> {
        self.add_source_str_in_world_with_name(&mut StdWorld::new(), source, name)
    }

    pub fn define_macro(&mut self, name: impl AsRef<str>, value: Macro) -> &mut Self {
        self.macros.define(name.as_ref(), value);
        self
    }

    pub fn push_include_dir(&mut self, include_dir: impl Into<PathBuf>) -> &mut Self {
        self.include_dirs.push(include_dir.into());
        self
    }

    pub fn parse<'a>(self, arena: &'a Arena) -> Result<ParsedDesign<'a>, Box<ParseError>> {
        let mut tkw = TokenWalker::new(&self.token_buffer);
        let mut arenas = AstArenas::default();

        let mut diagnostics = Diagnostics::default();
        let mut ctx = ParseContext::new();
        let ast = parse_file(
            &mut tkw,
            &mut ParserScratches::default(),
            Some(&mut diagnostics),
            &mut arenas,
            arena,
            &mut ctx,
        );
        let Ok(ast) = ast else {
            return Err(Box::new(ParseError {
                builder: self,
                diagnostics,
            }));
        };

        Ok(ParsedDesign {
            ast,
            token_buffer: self.token_buffer,
            arenas,
            time_resolution: ctx.min_time_precision,
        })
    }
}

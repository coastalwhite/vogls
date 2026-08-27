use std::fmt;

use vogls_ir::token_range::TokenRange;

use crate::ast::{AstItem, Identifier};
use crate::elaborate::ModuleSymbol;
use crate::parser::{AstArenas, ReportKind, SpanError};
use crate::tokenizer::Tokenized;
use vogls_frontend::ident_table::IdentId;

use super::VType;

#[derive(Clone, Debug)]
pub enum LowerErrorReason {
    VariableNotFound(IdentId),
    PortNotFound(IdentId),
    PortNotDefined(IdentId),
    NotYetImplemented(&'static str),
    OutputExprNotAllowed,
    InvalidNumArguments(String),
    DuplicateDefinition(IdentId),
    ModuleNotFound(IdentId),
    UdpNotFound(IdentId),
    NetWidthOverflow,
    ZeroWidthNet,
}

#[derive(Debug, Default)]
pub struct Diagnostics {
    pub errors: Vec<(TokenRange, LowerErrorReason, Vec<String>)>,
    pub warnings: Vec<(TokenRange, String)>,
}

impl Diagnostics {
    pub fn var_not_found(&mut self, arenas: &AstArenas, ident: AstItem<Identifier>) {
        self.errors.push((
            arenas.get_item_span(ident),
            LowerErrorReason::VariableNotFound(ident.item.0),
            Vec::new(),
        ));
    }

    pub fn port_not_found(
        &mut self,
        arenas: &AstArenas,
        _io: &ModuleSymbol,
        ident: AstItem<Identifier>,
    ) {
        // @TODO
        // let context = format!(
        //     "available ports: {:?}",
        //     io.ports
        //         .iter()
        //         .map(|(i, _)| *s)
        //         .collect::<Vec<&str>>()
        // );
        self.errors.push((
            arenas.get_item_span(ident),
            LowerErrorReason::PortNotFound(ident.item.0),
            vec![],
        ));
    }

    pub fn not_yet_implemented(&mut self, tr: TokenRange, reason: &'static str) {
        self.errors
            .push((tr, LowerErrorReason::NotYetImplemented(reason), Vec::new()));
    }
    pub fn invalid_num_arguments(&mut self, tr: TokenRange, reason: impl Into<String>) {
        self.errors.push((
            tr,
            LowerErrorReason::InvalidNumArguments(reason.into()),
            Vec::new(),
        ));
    }

    pub fn warn_not_yet_implemented(&mut self, tr: TokenRange, reason: &'static str) {
        self.warnings
            .push((tr, format!("not yet implemented: {reason}")));
    }

    pub fn warn_assign_type_mismatch(&mut self, tr: TokenRange, dst: VType, src: VType) {
        self.warnings
            .push((tr, format!("assign type mismatch: {dst:?} <- {src:?}")));
    }

    pub fn output_expr_not_allowed(&mut self, tr: TokenRange) {
        self.errors
            .push((tr, LowerErrorReason::OutputExprNotAllowed, Vec::new()));
    }

    pub fn port_not_defined(&mut self, arenas: &AstArenas, port_ident: AstItem<Identifier>) {
        self.errors.push((
            arenas.get_item_span(port_ident),
            LowerErrorReason::PortNotDefined(port_ident.item.0),
            Vec::new(),
        ));
    }

    pub fn duplicate_definition(&mut self, arenas: &AstArenas, ident: AstItem<Identifier>) {
        self.errors.push((
            arenas.get_item_span(ident),
            LowerErrorReason::DuplicateDefinition(ident.item.0),
            Vec::new(),
        ));
    }

    pub fn module_not_found(&mut self, arenas: &AstArenas, ident: AstItem<Identifier>) {
        self.errors.push((
            arenas.get_item_span(ident),
            LowerErrorReason::ModuleNotFound(ident.item.0),
            Vec::new(),
        ));
    }

    pub fn udp_not_found(&mut self, arenas: &AstArenas, ident: AstItem<Identifier>) {
        self.errors.push((
            arenas.get_item_span(ident),
            LowerErrorReason::UdpNotFound(ident.item.0),
            Vec::new(),
        ));
    }

    pub fn net_width_overflow(&mut self, at: TokenRange) {
        self.errors
            .push((at, LowerErrorReason::NetWidthOverflow, Vec::new()));
    }
    pub fn zero_width_net(&mut self, at: TokenRange) {
        self.errors
            .push((at, LowerErrorReason::ZeroWidthNet, Vec::new()));
    }

    pub fn report<'a>(
        &'a self,
        tokens: &'a Tokenized,
        arenas: &'a AstArenas,
    ) -> DiagnosticsReport<'a> {
        DiagnosticsReport(self, tokens, arenas)
    }
}

pub struct DiagnosticsReport<'a>(&'a Diagnostics, &'a Tokenized, &'a AstArenas);

impl<'a> fmt::Display for DiagnosticsReport<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.0.warnings.is_empty() {
            for (location, warning) in &self.0.warnings {
                SpanError {
                    spans: &self.1.spans,
                    file_idxs: &self.1.file_idxs,
                    paths: &self.1.paths,
                    contents: &self.1.contents,
                    error: warning,
                    kind: ReportKind::Warning,
                    code: None,
                    location: *location,
                }
                .fmt(f)?;
            }
        }

        for (location, err, _context) in &self.0.errors {
            SpanError {
                spans: &self.1.spans,
                file_idxs: &self.1.file_idxs,
                paths: &self.1.paths,
                contents: &self.1.contents,
                error: LowerErrorDisplay {
                    error: err,
                    arenas: self.2,
                },
                kind: ReportKind::Error,
                code: None,
                location: *location,
            }
            .fmt(f)?;
        }

        Ok(())
    }
}

struct LowerErrorDisplay<'a> {
    error: &'a LowerErrorReason,
    arenas: &'a AstArenas,
}

impl<'a> fmt::Display for LowerErrorDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ident_table = &self.arenas.ident_table;
        match self.error {
            LowerErrorReason::VariableNotFound(iid) => {
                write!(f, "variable `{}` not found", &ident_table[*iid])
            }
            LowerErrorReason::PortNotFound(iid) => {
                write!(f, "port `{}` not found", &ident_table[*iid])
            }
            LowerErrorReason::PortNotDefined(iid) => {
                write!(f, "port `{}` is not defined", &ident_table[*iid])
            }
            LowerErrorReason::NotYetImplemented(reason) => {
                write!(f, "not yet implemented or no specialized error: {reason}")
            }
            LowerErrorReason::OutputExprNotAllowed => write!(f, "not allowed as output expression"),
            LowerErrorReason::InvalidNumArguments(_) => write!(f, "invalid number of arguments"),
            LowerErrorReason::DuplicateDefinition(iid) => {
                write!(f, "duplication definition of `{}`", &ident_table[*iid])
            }
            LowerErrorReason::ModuleNotFound(iid) => {
                write!(f, "module `{}` is not found", &ident_table[*iid])
            }
            LowerErrorReason::UdpNotFound(iid) => {
                write!(f, "UDP `{}` is not found", &ident_table[*iid])
            }
            LowerErrorReason::NetWidthOverflow => write!(f, "bit length overflow"),
            LowerErrorReason::ZeroWidthNet => write!(f, "zero bit length"),
        }
    }
}

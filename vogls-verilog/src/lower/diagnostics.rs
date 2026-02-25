use vogls_ir::token_range::TokenRange;

use crate::ast::{AstItem, Identifier};
use crate::elaborate::ModuleSymbol;
use crate::parser::AstArenas;
use vogls_frontend::ident_table::IdentId;

use super::VType;

#[derive(Clone, Debug)]
pub enum LowerErrorReason {
    VariableNotFound(IdentId),
    PortNotFound(IdentId),
    PortNotDefined(IdentId),
    NotYetImplemented(&'static str),
    OutputExprNotAllowed,
    DuplicateDefinition(IdentId),
    ModuleNotFound(IdentId),
    UdpNotFound(IdentId),
    NetWidthOverflow,
    ZeroWidthNet,
}

#[derive(Default)]
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

    pub fn port_not_found<'a>(
        &mut self,
        arenas: &'a AstArenas,
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
}

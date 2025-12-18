use vogls_ir::{GlobalContext, ModuleKey, Type};

use crate::ast::{AstItem, Identifier, TextRef};
use crate::parser::{AstArenas, TokenRange};

#[derive(Clone, Debug)]
pub enum LowerErrorReason {
    VariableNotFound(TextRef),
    PortNotFound(TextRef),
    NotYetImplemented(&'static str),
    OutputExprNotAllowed,
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

    pub fn port_not_found(
        &mut self,
        gl: &GlobalContext,
        arenas: &AstArenas,
        module: ModuleKey,
        ident: AstItem<Identifier>,
    ) {
        let context = format!(
            "available ports: {:?}",
            gl.modules[module].io.keys().collect::<Vec<&String>>()
        );
        self.errors.push((
            arenas.get_item_span(ident),
            LowerErrorReason::PortNotFound(ident.item.0),
            vec![context],
        ));
    }

    pub fn not_yet_implemented(&mut self, tr: TokenRange, reason: &'static str) {
        self.errors
            .push((tr, LowerErrorReason::NotYetImplemented(reason), Vec::new()));
    }

    pub fn warn_assign_type_mismatch(&mut self, tr: TokenRange, dst: Type, src: Type) {
        self.warnings
            .push((tr, format!("assign type mismatch: {dst:?} <- {src:?}")));
    }

    pub fn output_expr_not_allowed(&mut self, tr: TokenRange) {
        self.errors
            .push((tr, LowerErrorReason::OutputExprNotAllowed, Vec::new()));
    }
}

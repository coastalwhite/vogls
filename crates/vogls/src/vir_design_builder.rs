use vogls_frontend::ident_table::IdentTable;
use vogls_frontend::symbol_table::FrozenSymbolTable;
use vogls_ir::parse::ParseError;
use vogls_ir::{GlobalContext, LogicMode};
use vogls_verilog::tokenizer::Tokenized;

use crate::LoweredDesign;

pub struct VirDesignBuilder<'a> {
    content: &'a str,
    logic_mode: LogicMode,
}

impl<'a> VirDesignBuilder<'a> {
    pub fn new(content: &'a str) -> Self {
        Self {
            content,
            logic_mode: LogicMode::default(),
        }
    }

    pub fn with_logic_mode(&mut self, logic_mode: LogicMode) -> &mut Self {
        self.logic_mode = logic_mode;
        self
    }

    pub fn parse(&self) -> Result<LoweredDesign, Box<ParseError>> {
        let mut gl = GlobalContext::default();
        // @TODO: Fill ident table and symbol table.
        let ident_table = IdentTable::default();
        vogls_ir::parse::parse(self.content, &mut gl)?;
        Ok(LoweredDesign {
            table: FrozenSymbolTable::default(),
            gl,
            // @TODO: Add plugin support for VIR.
            plugins: vec![],
            vcd: None,
            has_vcd: false,
            ident_table,
            token_buffer: Tokenized::default(),
            itrace: false,
            emit_vm: false,
            stats: false,
            debug_symbols: false,
            output_source: None,
            print_vm_map: false,
        })
    }
}

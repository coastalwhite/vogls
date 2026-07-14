use std::fmt;

use hashbrown::HashSet;
use vogls_frontend::diagnostic::Diagnostic;
use vogls_frontend::ident_table::{IdentId, IdentTable};
use vogls_ir::{GlobalContext, LogicMode};
use vogls_utils::VgHashMap;
use vogls_verilog::ast::AstId;
use vogls_verilog::ast::module::{
    CaseGenerateConstruct, CaseGenerateItem, Description, GenerateBlock, IfGenerateConstruct,
    LoopGenerateConstruct, Module, ModuleItem, ModuleOrGenerateItem, ModuleOrGenerateItemContent,
    NonPortModuleItem, TimeScale,
};
use vogls_verilog::elaborate::{SymbolAstRefs, VSymbolTable};
use vogls_verilog::lower::{Diagnostics, LowerContext, MutLowerContext};
use vogls_verilog::parser::{Ast, AstArenas, Diagnostics as ParseDiagnostics};
use vogls_verilog::tokenizer::Tokenized;

use crate::elaborated_design::ElaborationErrorKind;
use crate::{DesignBuilder, ElaboratedDesign, ElaborationError};

#[derive(Clone)]
pub struct ParsedDesign<'a> {
    pub(crate) ast: Ast<'a>,
    pub(crate) token_buffer: Tokenized,
    pub(crate) arenas: AstArenas,
}

pub struct ParseError {
    pub builder: DesignBuilder,
    pub diagnostics: ParseDiagnostics,
}

impl std::error::Error for ParseError {}
impl fmt::Debug for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ParseError")
            .field(&format!("num_errors: {}", self.diagnostics.errors.len()))
            .finish_non_exhaustive()
    }
}

// @TODO: Wrap this in a stable API somehow.
pub use vogls_verilog::parser::ParseErrorReason;

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (tr, reason) in &self.diagnostics.errors {
            let start = self.builder.token_buffer.spans[tr.start].start();
            let end = self.builder.token_buffer.spans[tr.end].end();

            let file_idx = self.builder.token_buffer.file_idxs[tr.start];

            let span = if file_idx != self.builder.token_buffer.file_idxs[tr.end] {
                start..end
            } else {
                start..start
            };
            writeln!(
                f,
                "{}",
                Diagnostic::new(
                    &self.builder.token_buffer.contents[file_idx as usize],
                    self.builder.token_buffer.paths[file_idx as usize].as_deref(),
                    &self.builder.token_buffer.file_line_offsets[file_idx as usize],
                    reason,
                    span,
                )
            )?;
        }

        Ok(())
    }
}

impl<'a> ParsedDesign<'a> {
    pub fn infer_top_level_module(
        &self,
    ) -> Result<(AstId<'a, Module<'a>>, IdentId), Vec<(AstId<'a, Module<'a>>, IdentId)>> {
        let mut referenced = HashSet::new();
        for id in self.ast.descriptions {
            let Description::Module(module_id) = &*id else {
                continue;
            };

            let Module {
                attribute_instances: _,
                module_identifier: _,
                module_parameter_port_list: _,
                module_items,
                ports: _,
                default_nettype: _,
                time_scale: _,
            } = &**module_id;

            for module_item in module_items.iter() {
                let ModuleItem::NonPortModuleItem(p) = &*module_item else {
                    continue;
                };

                if let NonPortModuleItem::ModuleOrGenerateItem(module_item) = &**p {
                    append_referenced_modules(&self.arenas, *module_item, &mut referenced);
                }
            }
        }

        let mut top_level_modules = Vec::new();
        for id in self.ast.descriptions {
            let Description::Module(module_id) = &*id else {
                continue;
            };
            let Module {
                attribute_instances: _,
                module_identifier,
                module_parameter_port_list: _,
                module_items: _,
                ports: _,
                default_nettype: _,
                time_scale: _,
            } = &**module_id;
            let module_name = module_identifier.item.0;
            if referenced.contains(&module_name) {
                continue;
            }
            top_level_modules.push((*module_id, module_name));
        }

        if top_level_modules.len() == 1 {
            return Ok(top_level_modules[0]);
        }

        Err(top_level_modules)
    }

    pub fn elaborate(
        self,
        mode: LogicMode,
        top_level_module: Option<impl AsRef<str>>,
    ) -> Result<ElaboratedDesign<'a>, ElaborationError<'a>> {
        // @TODO: Verify that all modules are uniquely named.
        let module_lut = VgHashMap::<IdentId, AstId<Module>>::from_iter(
            self.ast.descriptions.iter().filter_map(|id| match &*id {
                Description::Module(id) => Some((id.module_identifier.item.0, *id)),
                Description::Udp(_) | Description::Config => None,
            }),
        );

        let top_level_module = match top_level_module {
            Some(name) => {
                let id = self
                    .arenas
                    .ident_table
                    .get(name.as_ref())
                    .and_then(|name| module_lut.get(&name).copied());
                match id {
                    None => {
                        return Err(ElaborationError {
                            design: self,
                            kind: ElaborationErrorKind::CannotFindTopLevelModule(
                                name.as_ref().to_string(),
                            ),
                        });
                    }
                    Some(id) => id,
                }
            }
            None => match self.infer_top_level_module() {
                Ok((m, _)) => m,
                Err(top_level_modules) => {
                    return Err(ElaborationError {
                        design: self,
                        kind: ElaborationErrorKind::AmbiguousTopLevelModule(top_level_modules),
                    });
                }
            },
        };

        let mut ctx = LowerContext {
            logic_mode: mode,
            table: VSymbolTable::default(),
            table_ast_refs: SymbolAstRefs::default(),
            udps: VgHashMap::default(),
            arenas: &self.arenas,
            tokenized: &self.token_buffer,
            time_scale: TimeScale::default(),
        };
        let mut mctx = MutLowerContext {
            gl: GlobalContext::default(),
            diagnostics: Diagnostics::default(),
            connections: Vec::new(),
            fuse_scratch: Vec::new(),
            has_vcd: false,
        };
        let Ok(()) = vogls_verilog::elaborate::next::elaborate(
            &mut mctx.gl,
            &mut ctx,
            top_level_module,
            &module_lut,
            &mut mctx.diagnostics,
        ) else {
            return Err(ElaborationError {
                design: self,
                kind: ElaborationErrorKind::Diagnostics(mctx.diagnostics),
            });
        };

        for description in self.ast.descriptions.iter() {
            let Description::Udp(udp_id) = &*description else {
                continue;
            };

            let udp_id = *udp_id;
            let ident = udp_id.identifier.item.0;

            ctx.udps.insert(ident, udp_id);
        }

        let table = ctx.table;
        let table_ast_refs = ctx.table_ast_refs;
        let udps = ctx.udps;
        Ok(ElaboratedDesign {
            ast: self.ast,
            token_buffer: self.token_buffer,
            arenas: self.arenas,

            logic_mode: mode,
            module_lut,
            table,
            table_ast_refs,
            udps,
            gl: mctx.gl,

            unoptimized_fgs: None,
            optimized_fgs: None,
        })
    }

    pub fn ident_table(&self) -> &IdentTable {
        &self.arenas.ident_table
    }
    pub fn token_buffer(&self) -> &Tokenized {
        &self.token_buffer
    }
}

fn append_referenced_modules_generate_block<'a>(
    arenas: &'a AstArenas,
    generate_block: AstId<'a, GenerateBlock<'a>>,
    referenced: &mut HashSet<IdentId>,
) {
    match &*generate_block {
        GenerateBlock::ModuleOrGenerateItem(id) => {
            append_referenced_modules(arenas, *id, referenced)
        }
        GenerateBlock::BeginEnd(_, ids) => {
            for id in ids.iter() {
                append_referenced_modules(arenas, id, referenced);
            }
        }
    }
}

fn append_referenced_modules_opt_generate_block<'a>(
    arenas: &'a AstArenas,
    generate_block: AstId<Option<GenerateBlock<'a>>>,
    referenced: &mut HashSet<IdentId>,
) {
    match &*generate_block {
        None => {}
        Some(GenerateBlock::ModuleOrGenerateItem(id)) => {
            append_referenced_modules(arenas, *id, referenced)
        }
        Some(GenerateBlock::BeginEnd(_, ids)) => {
            for id in ids.iter() {
                append_referenced_modules(arenas, id, referenced);
            }
        }
    }
}

fn append_referenced_modules<'a>(
    arenas: &'a AstArenas,
    module_or_generate_item: AstId<'a, ModuleOrGenerateItem<'a>>,
    referenced: &mut HashSet<IdentId>,
) {
    match module_or_generate_item.content {
        ModuleOrGenerateItemContent::ModuleInstantiation(module_instantiation) => {
            let module_instantiation = &*module_instantiation;
            let module_name = module_instantiation.module_identifier.item.0;
            referenced.insert(module_name);
        }
        ModuleOrGenerateItemContent::ModuleOrGenerateItemDeclaration(_) => {}
        ModuleOrGenerateItemContent::LocalParameterDeclaration(_) => {}
        ModuleOrGenerateItemContent::ParameterOverride => {}
        ModuleOrGenerateItemContent::ContinuousAssign(_) => {}
        ModuleOrGenerateItemContent::GateInstantiation(_) => {}
        ModuleOrGenerateItemContent::UdpInstantiation(_) => {}
        ModuleOrGenerateItemContent::InitialConstruct(_) => {}
        ModuleOrGenerateItemContent::AlwaysConstruct(_) => {}
        ModuleOrGenerateItemContent::LoopGenerateConstruct(loop_generate_construct) => {
            let LoopGenerateConstruct {
                initialization: _,
                condition: _,
                iteration: _,
                block,
            } = &*loop_generate_construct;
            append_referenced_modules_generate_block(arenas, *block, referenced);
        }
        ModuleOrGenerateItemContent::IfGenerateConstruct(if_generate_construct) => {
            let IfGenerateConstruct {
                condition: _,
                truthy,
                falsy,
            } = &*if_generate_construct;
            append_referenced_modules_opt_generate_block(arenas, *truthy, referenced);
            if let Some(falsy) = falsy {
                append_referenced_modules_opt_generate_block(arenas, *falsy, referenced);
            }
        }
        ModuleOrGenerateItemContent::CaseGenerateConstruct(case_generate_construct) => {
            let CaseGenerateConstruct { value: _, items } = &*case_generate_construct;
            for item in items.iter() {
                let CaseGenerateItem { pattern: _, block } = &*item;
                append_referenced_modules_opt_generate_block(arenas, *block, referenced);
            }
        }
    }
}

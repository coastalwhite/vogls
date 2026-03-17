use core::fmt;
use std::num::NonZeroU32;
use std::sync::Arc;

use vogls_frontend::ident_table::IdentId;
use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{
    BasicBlockBuilder, BasicBlockKey, Bits, ConnectionDirection, GlobalContext, INTEGER_VSIZE,
    ProcessKey, SCALAR_VSIZE, Signal, SignalKey, VariableKey, VectorSize,
};
use vogls_utils::{new_table_key, Table, VgHashMap};

use crate::ast::constant_expr::ConstantExpr;
use crate::ast::module::{
    Dimension, FunctionDeclaration, ModuleOrGenerateItem, PortDeclaration, Range, TaskDeclaration,
};
use crate::ast::{AstId, AstIdRange, AstItem, Identifier};
use crate::lower::{Diagnostics, VType, VValue, eval_constant_expr};
use crate::parser::AstArenas;

pub mod function;
pub mod next;

pub type VSymbolTable = vogls_frontend::symbol_table::SymbolTable<VSymbol>;

new_table_key! { pub struct AstGenBlockKey; }
new_table_key! { pub struct AstFnDeclKey; }
new_table_key! { pub struct AstTaskDeclKey; }

#[derive(Default)]
pub struct SymbolAstRefs<'a> {
    pub gen_blocks: Table<AstGenBlockKey, AstIdRange<'a, ModuleOrGenerateItem<'a>>>,
    pub fns: Table<AstFnDeclKey, AstId<'a, FunctionDeclaration<'a>>>,
    pub tasks: Table<AstTaskDeclKey, AstId<'a, TaskDeclaration<'a>>>,
}

pub enum VSymbol {
    Module(ModuleSymbol),
    Parameter(VValue),
    Net(NetSymbol),
    NamedBlock,
    GenerateBlock(AstGenBlockKey),
    GenVar,
    Task(TaskSymbol),
    Function(FunctionSymbol),

    GenerateBlocks,
}

impl fmt::Debug for VSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            VSymbol::Module(_) => "module",
            VSymbol::Parameter(_) => "param",
            VSymbol::Net(_) => "net",
            VSymbol::NamedBlock => "named_block",
            VSymbol::GenerateBlock(_) => "generate_block",
            VSymbol::GenVar => "genvar",
            VSymbol::Task(_) => "task",
            VSymbol::Function(_) => "function",
            VSymbol::GenerateBlocks => "generate_blocks",
        })
    }
}

pub struct ModuleSymbol {
    pub module: IdentId,

    pub ports: Vec<(SymbolId, ConnectionDirection)>,
    pub parameters: Vec<SymbolId>,

    pub parameter_overrides: Arc<VgHashMap<IdentId, usize>>,
    pub parameter_override_values: Arc<Vec<VValue>>,

    pub contains_specify: bool,
}

pub struct Net {
    width: VectorSize,
    pub offset: Option<u32>,

    specify: Option<SignalKey>,
    ba: SignalKey,
    pub nba: Option<(ProcessKey, SignalKey, SignalKey)>,
}

impl Net {
    pub fn set_specify(&mut self, value: SignalKey) -> Option<SignalKey> {
        self.specify.replace(value)
    }

    pub fn probe_signal(&self) -> SignalKey {
        self.ba
    }

    pub fn blocking_drive_signal(&self) -> SignalKey {
        self.specify.unwrap_or(self.ba)
    }

    pub fn non_blocking_drive_signal(&self) -> (SignalKey, SignalKey) {
        let (_, value, mask) = self.nba.unwrap();
        (value, mask)
    }

    pub fn probe(&self, gl: &mut GlobalContext, bbb: &mut BasicBlockBuilder) -> VariableKey {
        let v = bbb.probe(gl, self.probe_signal());
        match self.offset {
            None => v,
            Some(offset) => bbb.slice_constant(gl, v, offset, self.width),
        }
    }

    pub fn drive_blocking(
        &self,
        gl: &mut GlobalContext,
        bbb: &mut BasicBlockBuilder,
        src: VariableKey,
        partial: Option<(VariableKey, VectorSize)>,
    ) {
        let signal = self.blocking_drive_signal();
        match (self.offset, partial) {
            (None, _) => bbb.drive_opt_partial(gl, signal, src, partial),
            (Some(offset), None) => {
                bbb.drive_partial_constant(gl, signal, src, offset, self.width);
            }
            (Some(net_offset), Some((offset, length))) => {
                let offset =
                    bbb.plus_constant(gl, offset, Bits::from_u64(INTEGER_VSIZE, net_offset as u64));
                bbb.drive_partial(gl, signal, src, offset, length);
            }
        }
    }
    pub fn drive_non_blocking(
        &self,
        gl: &mut GlobalContext,
        bbb: &mut BasicBlockBuilder,
        src: VariableKey,
        partial: Option<(VariableKey, VectorSize)>,
    ) {
        let (mask, value) = self.non_blocking_drive_signal();
        let size = gl.vars[src].size;
        let mask_value = bbb.constant(gl, Bits::new_ones(size));
        match (self.offset, partial) {
            (None, _) => {
                bbb.drive_opt_partial(gl, mask, mask_value, partial);
                bbb.drive_opt_partial(gl, value, src, partial);
            }
            (Some(offset), None) => {
                bbb.drive_partial_constant(gl, mask, mask_value, offset, self.width);
                bbb.drive_partial_constant(gl, value, src, offset, self.width);
            }
            (Some(net_offset), Some((offset, length))) => {
                let offset =
                    bbb.plus_constant(gl, offset, Bits::from_u64(INTEGER_VSIZE, net_offset as u64));
                bbb.drive_partial(gl, mask, mask_value, offset, length);
                bbb.drive_partial(gl, value, src, offset, length);
            }
        }
    }

    pub fn width(&self) -> VectorSize {
        self.width
    }
}

pub struct NetSymbol {
    pub ty: VType,
    pub dims: Vec<u32>,
    pub net: Net,
    pub port_idx: Option<usize>,
}

pub struct FunctionSymbol {
    pub ast_id: AstFnDeclKey,
    pub inputs: Vec<(SignalKey, VType)>,
    pub output: SignalKey,
    pub output_ty: VType,
    pub lowered: Option<LoweredFunction>,
}

pub struct TaskSymbol {
    pub ast_id: AstTaskDeclKey,
    pub io: Vec<(SignalKey, ConnectionDirection, VType)>,
    pub lowered: Option<LoweredTask>,
}

#[derive(Clone)]
pub struct LoweredFunction {
    pub entry: BasicBlockKey,
    pub terminate: BasicBlockKey,
}

#[derive(Clone)]
pub struct LoweredTask {
    pub entry: BasicBlockKey,
    pub terminate: BasicBlockKey,
}

pub fn try_table_insert(
    arenas: &AstArenas,
    table: &mut VSymbolTable,
    parent: SymbolId,
    name: AstItem<Identifier>,
    content: VSymbol,
    diagnostics: &mut Diagnostics,
) -> Result<SymbolId, ()> {
    let Ok(symid) = table.insert(name.item.0, parent, arenas.get_item_span(name), content) else {
        diagnostics.duplicate_definition(arenas, name);
        return Err(());
    };

    Ok(symid)
}

pub fn port_declaration_to_info<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,

    id: AstId<'a, PortDeclaration<'a>>,

    parent: SymbolId,
    table: &VSymbolTable,
    diagnostics: &mut Diagnostics,
) -> Result<(VType, ConnectionDirection, AstIdRange<'a, Identifier>), ()> {
    use ConnectionDirection as D;
    let (direction, range, signed, identifiers) = match &*id {
        PortDeclaration::Inout(inout) => {
            (D::Both, inout.range, inout.signed, inout.port_identifiers)
        }
        PortDeclaration::Input(input) => (D::In, input.range, input.signed, input.port_identifiers),
        PortDeclaration::Output(output) => {
            (D::Out, output.range, output.signed, output.identifiers)
        }
    };

    let (_, _, size) = match range {
        None => (0, 0, SCALAR_VSIZE),
        Some(range) => eval_constant_range(gl, arenas, parent, table, diagnostics, range)?,
    };
    let ty = VType::net(size, signed);
    Ok((ty, direction, identifiers))
}

fn new_net(
    gl: &mut GlobalContext,
    arenas: &AstArenas,
    ty: &VType,
    dims: &[u32],
    name: AstItem<Identifier>,
    initialize: Option<VValue>,
) -> Net {
    let mut size = ty.force_net_width();
    for dim in dims {
        size = size.checked_mul(NonZeroU32::new(*dim).unwrap()).unwrap();
    }
    let origin = arenas.get_item_span(name);
    let name = arenas.ident_table[name.item.0].to_string();
    let signal = gl.signals.insert(Signal {
        name,
        size,
        initialize: initialize.map(|i| i.into_bits()),
        origin,
    });

    Net {
        width: size,
        offset: None,

        specify: None,
        ba: signal,
        nba: None,
    }
}

pub fn eval_constant_expr_elab<'a>(
    gl: &GlobalContext,
    arenas: &'a AstArenas,
    scope: SymbolId,
    table: &VSymbolTable,
    diagnostics: &mut Diagnostics,
    expr: AstId<ConstantExpr>,
) -> Result<VValue, ()> {
    eval_constant_expr(gl, arenas, table, scope, diagnostics, expr)
}

pub fn eval_constant_range<'a>(
    gl: &GlobalContext,
    arenas: &'a AstArenas,
    scope: SymbolId,
    table: &VSymbolTable,
    diagnostics: &mut Diagnostics,
    ast_range: AstId<'a, Range<'a>>,
) -> Result<(i64, i64, VectorSize), ()> {
    let range = ast_range;
    let msb = eval_constant_expr_elab(gl, arenas, scope, table, diagnostics, range.msb);
    let lsb = eval_constant_expr_elab(gl, arenas, scope, table, diagnostics, range.lsb);

    let (Ok(VValue::SignedNet(msb)), Ok(VValue::SignedNet(lsb))) = (msb, lsb) else {
        return Err(());
    };
    let msb = msb.as_i64().unwrap();
    let lsb = lsb.as_i64().unwrap();
    let width = u32::try_from(msb.abs_diff(lsb)).ok();
    let width = width.and_then(|w| w.checked_add(1));
    let width = width.and_then(|w| VectorSize::new(w));
    let Some(width) = width else {
        let tr = arenas.get_span(range.msb) | arenas.get_span(range.lsb);
        diagnostics.net_width_overflow(tr);
        return Err(());
    };
    Ok((msb, lsb, width))
}

pub fn dims_to_array_elab<'a>(
    gl: &GlobalContext,
    arenas: &'a AstArenas,
    scope: SymbolId,
    table: &VSymbolTable,
    diagnostics: &mut Diagnostics,
    dimensions: AstIdRange<'a, Dimension<'a>>,
) -> Result<Vec<u32>, ()> {
    let mut dims = Vec::with_capacity(dimensions.len());
    for dim in dimensions.iter().rev() {
        let Dimension { lhs, rhs } = &*dim;
        let lhs = eval_constant_expr_elab(gl, arenas, scope, table, diagnostics, *lhs);
        let rhs = eval_constant_expr_elab(gl, arenas, scope, table, diagnostics, *rhs);

        let lhs = lhs?.into_bits().as_i64().unwrap();
        let rhs = rhs?.into_bits().as_i64().unwrap();

        dims.push((lhs.abs_diff(rhs) + 1) as u32);
    }
    Ok(dims)
}

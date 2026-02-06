use core::fmt;
use std::num::NonZeroU32;
use std::sync::Arc;

use vogls_frontend::VgHashMap;
use vogls_frontend::ident_table::IdentId;
use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{
    BasicBlockKey, ConnectionDirection, GlobalContext, ProcessKey, SCALAR_VSIZE, Signal, SignalKey,
    VectorSize,
};

use crate::ast::constant_expr::ConstantExpr;
use crate::ast::module::{
    Dimension, FunctionDeclaration, ModuleOrGenerateItem, PortDeclaration, Range, TaskDeclaration,
};
use crate::ast::{AstId, AstIdRange, AstItem, Identifier};
use crate::lower::{Diagnostics, EvalScope, VType, VValue, eval_constant_expr};
use crate::parser::AstArenas;

pub mod function;
pub mod next;

pub type VSymbolTable = vogls_frontend::symbol_table::SymbolTable<VSymbol>;

pub enum VSymbol {
    Module(ModuleSymbol),
    Parameter(VValue),
    Net(NetSymbol),
    NamedBlock,
    GenerateBlock(AstIdRange<ModuleOrGenerateItem>),
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
}

pub struct NetSymbol {
    pub ty: VType,
    pub dims: Vec<u32>,
    pub signal: vogls_ir::SignalKey,
    pub nba: Option<(ProcessKey, SignalKey, SignalKey)>,
    pub port_idx: Option<usize>,
}

pub struct FunctionSymbol {
    pub ast_id: AstId<FunctionDeclaration>,
    pub inputs: Vec<(SignalKey, VType)>,
    pub output: SignalKey,
    pub output_ty: VType,
    pub lowered: Option<LoweredFunction>,
}

pub struct TaskSymbol {
    pub ast_id: AstId<TaskDeclaration>,
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

    id: AstId<PortDeclaration>,

    parent: SymbolId,
    table: &mut VSymbolTable,
    diagnostics: &mut Diagnostics,
) -> Result<(VType, ConnectionDirection, AstIdRange<Identifier>), ()> {
    use ConnectionDirection as D;
    let (direction, range, signed, identifiers) = match arenas.get(id) {
        PortDeclaration::Inout(id) => {
            let inout = arenas.get(*id);
            (D::Both, inout.range, inout.signed, inout.port_identifiers)
        }
        PortDeclaration::Input(id) => {
            let input = arenas.get(*id);
            (D::In, input.range, input.signed, input.port_identifiers)
        }
        PortDeclaration::Output(id) => {
            let output = arenas.get(*id);
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

fn new_signal(
    gl: &mut GlobalContext,
    arenas: &AstArenas,
    ty: &VType,
    dims: &[u32],
    name: AstItem<Identifier>,
) -> SignalKey {
    let mut size = ty.force_net_width();
    for dim in dims {
        size = size.checked_mul(NonZeroU32::new(*dim).unwrap()).unwrap();
    }
    let origin = arenas.get_item_span(name);
    let name = arenas.ident_table[name.item.0].to_string();
    gl.signals.insert(Signal {
        name,
        size,
        initialize: None,
        origin,
    })
}

pub fn eval_constant_expr_elab<'a>(
    gl: &GlobalContext,
    arenas: &'a AstArenas,
    scope: SymbolId,
    table: &VSymbolTable,
    diagnostics: &mut Diagnostics,
    expr: AstId<ConstantExpr>,
) -> Result<VValue, ()> {
    eval_constant_expr(
        gl,
        arenas,
        EvalScope { table, key: scope },
        diagnostics,
        expr,
    )
}

pub fn eval_constant_range(
    gl: &GlobalContext,
    arenas: &AstArenas,
    scope: SymbolId,
    table: &VSymbolTable,
    diagnostics: &mut Diagnostics,
    ast_range: AstId<Range>,
) -> Result<(i64, i64, VectorSize), ()> {
    let range = arenas.get(ast_range);
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
    dimensions: AstIdRange<Dimension>,
) -> Result<Vec<u32>, ()> {
    let mut dims = Vec::with_capacity(dimensions.len());
    for dim in dimensions.iter().rev() {
        let Dimension { lhs, rhs } = arenas.get(dim);
        let lhs = eval_constant_expr_elab(gl, arenas, scope, table, diagnostics, *lhs);
        let rhs = eval_constant_expr_elab(gl, arenas, scope, table, diagnostics, *rhs);

        let lhs = lhs?.into_bits().as_i64().unwrap();
        let rhs = rhs?.into_bits().as_i64().unwrap();

        dims.push((lhs.abs_diff(rhs) + 1) as u32);
    }
    Ok(dims)
}

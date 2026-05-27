use core::fmt;
use std::num::NonZeroU32;
use std::sync::Arc;

use vogls_frontend::ident_table::IdentId;
use vogls_frontend::symbol_table::SymbolId;
use vogls_ir::{
    BasicBlockBuilder, BasicBlockKey, Bits, ConnectionDirection, GlobalContext, ProcessKey, Signal, SignalFlags, SignalKey, VariableKey, VectorSize, SCALAR_VSIZE
};
use vogls_utils::{Table, VgHashMap, new_table_key};

use crate::ast::module::{
    Dimension, FunctionDeclaration, ModuleOrGenerateItem, PortDeclaration, Range, TaskDeclaration,
    TimeScale,
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
    pub time_scale: TimeScale,
}

pub struct Net {
    width: VectorSize,
    pub specify: Option<SignalKey>,
    pub ba: SignalKey,
    pub nba: Option<(ProcessKey, SignalKey, Option<SignalKey>)>,
}

impl Net {
    pub fn set_specify(&mut self, value: SignalKey) -> Option<SignalKey> {
        self.specify.replace(value)
    }

    pub fn probe_signal(&self) -> SignalKey {
        self.ba
    }

    pub fn blocking_drive_signal(&self) -> SignalKey {
        match self.specify {
            None => self.probe_signal(),
            Some(signal) => signal,
        }
    }

    pub fn non_blocking_drive_signal(&self) -> (SignalKey, Option<SignalKey>) {
        let (_, value, mask) = self.nba.unwrap();
        (value, mask)
    }

    pub fn probe(&self, gl: &mut GlobalContext, bbb: &mut BasicBlockBuilder) -> VariableKey {
        bbb.probe(gl, self.probe_signal())
    }

    pub fn drive_blocking(
        &self,
        gl: &mut GlobalContext,
        bbb: &mut BasicBlockBuilder,
        src: VariableKey,
        partial: Option<VariableKey>,
    ) {
        bbb.drive_opt_partial(
            gl,
            self.blocking_drive_signal(),
            src,
            partial.map(|o| (o, self.width())),
        )
    }
    pub fn drive_non_blocking(
        &self,
        gl: &mut GlobalContext,
        bbb: &mut BasicBlockBuilder,
        src: VariableKey,
        partial: Option<VariableKey>,
    ) {
        let (value, mask) = self.non_blocking_drive_signal();
        bbb.drive_opt_partial(gl, value, src, partial.map(|o| (o, self.width())));
        if let Some(mask) = mask {
            let size = gl.vars[src].size;
            let mask_value = bbb.constant(gl, Bits::new_ones(size));
            bbb.drive_opt_partial(gl, mask, mask_value, partial.map(|o| (o, self.width())));
        }
    }

    pub fn width(&self) -> VectorSize {
        self.width
    }
}

pub struct NetSymbol {
    pub ty: VType,
    pub dims: Vec<u32>,

    /// Nets can be defined as [8:4] in which case the least-significant bit starts at `4` not at
    /// `0`.
    pub lsb: u32,
    /// Nets can be defined as [0:7] in which case the addressing flips around, i.e. getting bit
    /// `0` actually gets the most-significant bit.
    pub bit_reversed: bool,

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

pub fn determine_module_context(
    mut sid: SymbolId,
    table: &VSymbolTable,
) -> (SymbolId, &ModuleSymbol) {
    loop {
        if let VSymbol::Module(ms) = &table[sid].content {
            return (sid, ms);
        };
        sid = table[sid].parent().expect("top-level should be a module");
    }
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

pub fn evaluate_net_msb_lsb<'a>(
    gl: &mut GlobalContext,
    arenas: &AstArenas,

    id: AstId<'a, Range<'a>>,

    scope: SymbolId,
    table: &VSymbolTable,
    diagnostics: &mut Diagnostics,
) -> Result<(u32, bool, VectorSize), ()> {
    let (msb, lsb, size) = eval_constant_range(gl, arenas, scope, table, diagnostics, id)?;

    let Ok(msb) = u32::try_from(msb) else {
        diagnostics.not_yet_implemented(arenas.get_span(id), "0 > msb > u32::MAX");
        return Err(());
    };
    let Ok(lsb) = u32::try_from(lsb) else {
        diagnostics.not_yet_implemented(arenas.get_span(id), "0 > lsb > u32::MAX");
        return Err(());
    };

    let bit_reversed = msb < lsb;
    let lsb = if bit_reversed { msb } else { lsb };
    Ok((lsb, bit_reversed, size))
}

pub fn port_declaration_to_info<'a>(
    gl: &mut GlobalContext,
    arenas: &'a AstArenas,

    id: AstId<'a, PortDeclaration<'a>>,

    parent: SymbolId,
    table: &VSymbolTable,
    diagnostics: &mut Diagnostics,
) -> Result<
    (
        VType,
        u32,
        bool,
        ConnectionDirection,
        AstIdRange<'a, Identifier>,
    ),
    (),
> {
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

    let (lsb, bit_reversed, size) = match range {
        None => (0, false, SCALAR_VSIZE),
        Some(range) => evaluate_net_msb_lsb(gl, arenas, range, parent, table, diagnostics)?,
    };

    let ty = VType::net(size, signed);
    Ok((ty, lsb, bit_reversed, direction, identifiers))
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

    let name = format!("{name}/{}", gl.signals.len());

    let signal = gl.signals.insert(Signal {
        name,
        size,
        initialize: initialize.map(|i| i.into_bits()),
        flags: SignalFlags::EMPTY,
        origin,
    });

    Net {
        width: size,
        specify: None,
        ba: signal,
        nba: None,
    }
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
    let msb = eval_constant_expr(gl, arenas, table, scope, diagnostics, range.msb, None);
    let lsb = eval_constant_expr(gl, arenas, table, scope, diagnostics, range.lsb, None);

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
        let lhs = eval_constant_expr(gl, arenas, table, scope, diagnostics, *lhs, None);
        let rhs = eval_constant_expr(gl, arenas, table, scope, diagnostics, *rhs, None);

        let lhs = lhs?.into_bits().as_i64().unwrap();
        let rhs = rhs?.into_bits().as_i64().unwrap();

        dims.push((lhs.abs_diff(rhs) + 1) as u32);
    }
    Ok(dims)
}

use std::fmt;

use vogls_ir::{Bits, ProcessKey, SCALAR_VSIZE, SignalKey, SignalSlice, VectorSize};
use vogls_utils::NonMaxU32;
use vogls_verilog::elaborate::{ArrayDim, VSymbol};
use vogls_verilog::lower::VType;

#[derive(Clone)]
pub enum Symbol {
    Module,
    Parameter(Bits),
    Signal(SignalSymbol),
    Block,
    Task,
    Function,

    GenerateBlocks,
    Modules,
}

impl fmt::Debug for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Symbol::Module => "module",
            Symbol::Modules => "modules",
            Symbol::Parameter(_) => "param",
            Symbol::Signal(_) => "signal",
            Symbol::Block => "block",
            Symbol::Task => "task",
            Symbol::Function => "function",
            Symbol::GenerateBlocks => "blocks",
        })
    }
}

impl From<VSymbol> for Symbol {
    fn from(value: VSymbol) -> Self {
        match value {
            VSymbol::Module(..) => Self::Module,
            VSymbol::ModuleRange(..) => Self::Modules,
            VSymbol::Parameter(vvalue) => Self::Parameter(vvalue.into_bits()),
            VSymbol::Signal(signal) => Self::Signal(SignalSymbol {
                ty: signal.ty,
                dims: signal.dims,
                value: SignalValue::Signal(SignalRefs {
                    width: signal.terminals.width(),
                    prb: (signal.terminals.probe_signal(), None),
                    drv: (signal.terminals.blocking_drive_signal(), None),
                    nba: signal.terminals.nba.map(|(p, s, m)| (p, s, None, m, None)),
                }),
            }),
            VSymbol::NamedBlock => Self::Block,
            VSymbol::GenerateBlock(_) => Self::Block,
            VSymbol::GenVar => Self::Parameter(Bits::new_zeroed(SCALAR_VSIZE)),
            VSymbol::Task(..) => Self::Task,
            VSymbol::Function(..) => Self::Function,
            VSymbol::GenerateBlocks => Self::GenerateBlocks,
        }
    }
}

#[derive(Clone)]
pub struct SignalSymbol {
    pub ty: VType,
    pub dims: Vec<ArrayDim>,
    pub value: SignalValue,
}

#[derive(Clone)]
pub enum SignalValue {
    Signal(SignalRefs),
    Constant(Bits),
}
impl SignalValue {
    pub fn size(&self) -> VectorSize {
        match self {
            SignalValue::Signal(s) => s.width,
            SignalValue::Constant(b) => b.size(),
        }
    }
}

#[derive(Clone)]
pub struct SignalRefs {
    width: VectorSize,
    pub prb: (SignalKey, Option<NonMaxU32>),
    pub drv: (SignalKey, Option<NonMaxU32>),
    pub nba: Option<(
        ProcessKey,
        SignalKey,
        Option<NonMaxU32>,
        Option<SignalKey>,
        Option<NonMaxU32>,
    )>,
}

impl SignalRefs {
    pub fn probe_signal(&self) -> (SignalKey, Option<SignalSlice>) {
        let (s, o) = self.prb;
        (
            s,
            o.map(|o| SignalSlice::from_width(o.get(), self.width).unwrap()),
        )
    }

    pub fn blocking_drive_signal(&self) -> (SignalKey, Option<SignalSlice>) {
        let (s, o) = self.drv;
        (
            s,
            o.map(|o| SignalSlice::from_width(o.get(), self.width).unwrap()),
        )
    }

    pub fn non_blocking_drive_signal(
        &self,
    ) -> (
        SignalKey,
        Option<SignalSlice>,
        Option<SignalKey>,
        Option<SignalSlice>,
    ) {
        let (_, value, value_offset, mask, mask_offset) = self.nba.unwrap();
        (
            value,
            value_offset.map(|lsb| SignalSlice::from_width(lsb.get(), self.width).unwrap()),
            mask,
            mask_offset.map(|lsb| SignalSlice::from_width(lsb.get(), self.width).unwrap()),
        )
    }

    pub fn map_prb(&mut self, mut f: impl FnMut(SignalKey) -> (SignalKey, Option<NonMaxU32>)) {
        self.prb = f(self.prb.0);
    }
    pub fn map_drv(&mut self, mut f: impl FnMut(SignalKey) -> (SignalKey, Option<NonMaxU32>)) {
        self.drv = f(self.prb.0);
    }
}

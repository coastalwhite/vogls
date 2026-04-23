use std::fmt;

use vogls_ir::{Bits, ProcessKey, SCALAR_VSIZE, SignalKey, SignalSlice, VectorSize};
use vogls_utils::NonMaxU32;
use vogls_verilog::elaborate::VSymbol;
use vogls_verilog::lower::VType;

pub enum Symbol {
    Module,
    Parameter(Bits),
    Net(NetSymbol),
    Block,
    Task,
    Function,

    GenerateBlocks,
}

impl fmt::Debug for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Symbol::Module => "module",
            Symbol::Parameter(_) => "param",
            Symbol::Net(_) => "net",
            Symbol::Block => "block",
            Symbol::Task => "task",
            Symbol::Function => "function",
            Symbol::GenerateBlocks => "block",
        })
    }
}

impl From<VSymbol> for Symbol {
    fn from(value: VSymbol) -> Self {
        match value {
            VSymbol::Module(..) => Self::Module,
            VSymbol::Parameter(vvalue) => Self::Parameter(vvalue.into_bits()),
            VSymbol::Net(net) => Self::Net(NetSymbol {
                ty: net.ty,
                dims: net.dims,
                net: NetValue::Signal(NetSignal {
                    width: net.net.width(),
                    specify: net.net.specify.map(|s| (s, None)),
                    ba: net.net.ba,
                    ba_offset: None,
                    nba: net.net.nba.map(|(p, s, m)| (p, s, None, m, None)),
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

pub struct NetSymbol {
    pub ty: VType,
    pub dims: Vec<u32>,
    pub net: NetValue,
}

pub enum NetValue {
    Signal(NetSignal),
    Constant(Bits),
}

pub struct NetSignal {
    width: VectorSize,
    specify: Option<(SignalKey, Option<NonMaxU32>)>,
    ba: SignalKey,
    ba_offset: Option<NonMaxU32>,
    pub nba: Option<(
        ProcessKey,
        SignalKey,
        Option<NonMaxU32>,
        Option<SignalKey>,
        Option<NonMaxU32>,
    )>,
}

impl NetSignal {
    pub fn probe_signal(&self) -> (SignalKey, Option<SignalSlice>) {
        (
            self.ba,
            self.ba_offset
                .map(|lsb| SignalSlice::from_width(lsb.get(), self.width).unwrap()),
        )
    }

    pub fn blocking_drive_signal(&self) -> (SignalKey, Option<SignalSlice>) {
        match self.specify {
            None => self.probe_signal(),
            Some((signal, lsb)) => (
                signal,
                lsb.map(|lsb| SignalSlice::from_width(lsb.get(), self.width).unwrap()),
            ),
        }
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

    pub fn replace_signals(
        &mut self,
        mut f: impl FnMut(SignalKey) -> (SignalKey, Option<NonMaxU32>),
    ) {
        assert!(self.ba_offset.is_none());
        (self.ba, self.ba_offset) = f(self.ba);
        if let Some(specify) = &mut self.specify {
            assert!(specify.1.is_none());
            *specify = f(specify.0);
        }
        if let Some((_, value, value_slice, mask, mask_slice)) = &mut self.nba {
            assert!(value_slice.is_none());
            assert!(mask_slice.is_none());
            (*value, *value_slice) = f(*value);
            if let Some(mask) = mask {
                (*mask, *mask_slice) = f(*mask);
            }
        }
    }
}

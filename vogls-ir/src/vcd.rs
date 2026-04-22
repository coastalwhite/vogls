use vogls_bits::Bits;
use vogls_utils::{Table, VgHashMap};

use crate::{SignalKey, SignalSlice};

vogls_utils::new_table_key! { pub struct VcdVariableKey; }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetType {
    Integer,
    Register,
    Wire,
}

#[derive(Debug, Clone)]
pub struct VcdOutput {
    pub table: Table<VcdVariableKey, VcdVariable>,
    pub signal_map: VgHashMap<SignalKey, Vec<VcdVariableKey>>,
    pub children: Vec<VcdScopeItem>,
}

#[derive(Debug, Clone)]
pub struct VcdScope {
    pub name: String,
    pub items: Vec<VcdScopeItem>,
}

#[derive(Debug, Clone)]
pub enum VcdValue {
    Signal(SignalKey, Option<SignalSlice>),
    Constant(Bits),
}

#[derive(Debug, Clone)]
pub struct VcdVariable {
    pub name: String,
    pub value: VcdValue,
    pub ty: NetType,
    pub msb_lsb: Option<(u32, u32)>,
}

#[derive(Debug, Clone)]
pub enum VcdScopeItem {
    Scope(VcdScope),
    Variable(VcdVariableKey),
}

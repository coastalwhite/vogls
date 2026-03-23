use vogls_bits::VectorSize;
use vogls_utils::{NonMaxU32, Table, VgHashMap};

use crate::SignalKey;

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
pub struct VcdVariable {
    pub name: String,
    pub signal: SignalKey,
    pub ty: NetType,
    pub offset: Option<NonMaxU32>,
    pub width: VectorSize,
}

#[derive(Debug, Clone)]
pub enum VcdScopeItem {
    Scope(VcdScope),
    Variable(VcdVariableKey),
}

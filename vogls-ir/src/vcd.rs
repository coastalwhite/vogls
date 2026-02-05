use crate::SignalKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetType {
    Integer,
    Register,
    Wire,
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
    pub msb: i64,
    pub lsb: i64,
}

#[derive(Debug, Clone)]
pub enum VcdScopeItem {
    Scope(VcdScope),
    Variable(VcdVariable),
}

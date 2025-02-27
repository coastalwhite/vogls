use crate::arena::{ArenaId, ArenaIdRange};

pub mod expr;
pub mod module;
pub mod statement;

#[derive(Clone, Copy)]
pub struct AstId<T> {
    pub node: ArenaId<T>,
    pub loc: usize,
}
#[derive(Clone, Copy)]
pub struct AstIdRange<T> {
    pub node: ArenaIdRange<T>,
    pub loc: usize,
}

#[derive(Clone, Copy)]
pub struct AstItem<T: Copy> {
    pub item: T,
    pub loc: usize,
}
#[derive(Clone, Copy)]
pub struct IdentRef {
    pub start: usize,
    pub end: usize,
}
#[derive(Clone, Copy)]
pub struct DecimalRef {
    pub at: usize,
}
#[derive(Clone, Copy)]
pub struct SizedNumberRef {
    pub at: usize,
}

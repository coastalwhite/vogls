use crate::arena::{ArenaId, ArenaIdRange};

pub mod expr;
pub mod constant_expr;
pub mod module;
pub mod statement;

pub struct AstId<T> {
    pub node: ArenaId<T>,
    pub loc: usize,
}
pub struct AstIdRange<T> {
    pub node: ArenaIdRange<T>,
    pub loc: usize,
}

impl<T> Clone for AstId<T> {
    fn clone(&self) -> Self {
        Self {
            node: self.node,
            loc: self.loc,
        }
    }
}
impl<T> Copy for AstId<T> {}

impl<T> Default for AstIdRange<T> {
    fn default() -> Self {
        Self {
            node: Default::default(),
            loc: Default::default(),
        }
    }
}
impl<T> Clone for AstIdRange<T> {
    fn clone(&self) -> Self {
        Self {
            node: self.node,
            loc: self.loc,
        }
    }
}
impl<T> Copy for AstIdRange<T> {}

impl<T> AstIdRange<T> {
    pub fn first(self) -> Option<AstId<T>> {
        self.node.first().map(|node| AstId {
            node,
            loc: self.loc,
        })
    }
    pub fn last(self) -> Option<AstId<T>> {
        self.node.last().map(|node| AstId {
            node,
            loc: self.loc + self.len() - 1,
        })
    }

    pub fn iter(self) -> impl ExactSizeIterator + DoubleEndedIterator<Item = AstId<T>> {
        self.node.iter().enumerate().map(move |(i, node)| AstId {
            node,
            loc: self.loc + i,
        })
    }

    pub fn len(self) -> usize {
        self.node.len()
    }
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, i: usize) -> AstId<T> {
        (*self).iter().nth(i).unwrap()
    }
}

#[derive(Clone, Copy)]
pub struct AstItem<T: Copy> {
    pub item: T,
    pub loc: usize,
}
#[derive(Clone, Copy)]
pub struct TextRef {
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
#[derive(Clone, Copy)]
pub struct StringRef(pub TextRef);

#[derive(Clone, Copy)]
pub struct Identifier(pub TextRef);

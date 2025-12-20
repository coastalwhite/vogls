use crate::arena::{ArenaId, ArenaIdRange, ArenaIdRangeIter};

use self::constant_expr::ConstantExpr;
use self::expr::Expr;

pub mod constant_expr;
pub mod expr;
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

    pub fn iter(self) -> AstIdRangeIter<T> {
        AstIdRangeIter {
            inner: self.node.iter(),
            loc: self.loc,
        }
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

    pub fn pop_front(&mut self) -> Option<AstId<T>> {
        let fst = self.node.pop_front()?;
        let fst = AstId {
            node: fst,
            loc: self.loc,
        };
        self.loc += 1;
        Some(fst)
    }
    pub fn pop_back(&mut self) -> Option<AstId<T>> {
        let lst = self.node.pop_back()?;
        let lst = AstId {
            node: lst,
            loc: self.loc + self.len() - 1,
        };
        Some(lst)
    }

    pub fn single(id: AstId<T>) -> AstIdRange<T> {
        Self {
            node: ArenaIdRange::single(id.node),
            loc: id.loc,
        }
    }
}

pub struct AstIdRangeIter<T> {
    inner: ArenaIdRangeIter<T>,
    loc: usize,
}

impl<T> Iterator for AstIdRangeIter<T> {
    type Item = AstId<T>;
    fn next(&mut self) -> Option<Self::Item> {
        let node = self.inner.next()?;
        let id = AstId {
            node,
            loc: self.loc,
        };
        self.loc += 1;
        Some(id)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
impl<T> DoubleEndedIterator for AstIdRangeIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let node = self.inner.next_back()?;
        let id = AstId {
            node,
            loc: self.loc + self.inner.len(),
        };
        Some(id)
    }
}
impl<T> ExactSizeIterator for AstIdRangeIter<T> {}
impl<T> IntoIterator for AstIdRange<T> {
    type Item = AstId<T>;
    type IntoIter = AstIdRangeIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Clone, Copy)]
pub struct AstItem<T: Copy> {
    pub item: T,
    pub loc: usize,
}
#[derive(Clone, Copy, Debug)]
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

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 507
// attribute_instance ::= (* attr_spec { , attr_spec } *)
#[derive(Clone, Copy)]
pub struct AttributeInstance(pub AstIdRange<AttrSpec>);

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 507
// attr_spec ::= attr_name [ = constant_expression ]
// attr_name ::= identifier
#[derive(Clone, Copy)]
pub struct AttrSpec {
    pub attr_name: AstItem<Identifier>,
    pub constant_expression: Option<AstId<ConstantExpr>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 505
// range_expression ::=
//   expression
// | msb_constant_expression : lsb_constant_expression
// | base_expression +: width_constant_expression
// | base_expression -: width_constant_expression
// base_expression ::= expression
// width_constant_expression ::= constant_expression
// msb_constant_expression ::= constant_expression
// lsb_constant_expression ::= constant_expression
#[derive(Clone, Copy)]
pub enum RangeExpression {
    Expr(AstId<Expr>),
    MsbLsb(AstId<ConstantExpr>, AstId<ConstantExpr>),
    BasePlus(AstId<Expr>, AstId<ConstantExpr>),
    BaseMinus(AstId<Expr>, AstId<ConstantExpr>),
}

use std::ops::Deref;

use vogls_frontend::ident_table::IdentId;
use vogls_ir::Bits;

use crate::number::{Base, Sign};

use self::constant_expr::ConstantExpr;

pub mod constant_expr;
pub mod expr;
pub mod module;
pub mod specify;
pub mod statement;
pub mod udp;

pub struct AstId<'a, T> {
    pub node: &'a T,
    pub loc: usize,
}
pub struct AstIdRange<'a, T> {
    pub node: &'a [T],
    pub loc: usize,
}

impl<'a, T> Clone for AstId<'a, T> {
    fn clone(&self) -> Self {
        Self {
            node: self.node,
            loc: self.loc,
        }
    }
}
impl<'a, T> Copy for AstId<'a, T> {}

impl<'a, T> Deref for AstId<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.node
    }
}

impl<'a, T> Default for AstIdRange<'a, T> {
    fn default() -> Self {
        Self {
            node: Default::default(),
            loc: Default::default(),
        }
    }
}
impl<'a, T> Clone for AstIdRange<'a, T> {
    fn clone(&self) -> Self {
        Self {
            node: self.node,
            loc: self.loc,
        }
    }
}
impl<'a, T> Copy for AstIdRange<'a, T> {}

impl<'a, T> AstIdRange<'a, T> {
    pub fn first(self) -> Option<AstId<'a, T>> {
        self.node.first().map(|node| AstId {
            node,
            loc: self.loc,
        })
    }
    pub fn last(self) -> Option<AstId<'a, T>> {
        self.node.last().map(|node| AstId {
            node,
            loc: self.loc + self.len() - 1,
        })
    }

    pub fn iter(self) -> AstIdRangeIter<'a, T> {
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

    pub fn get(&self, i: usize) -> AstId<'a, T> {
        (*self).iter().nth(i).unwrap()
    }

    pub fn pop_front(&mut self) -> Option<AstId<'a, T>> {
        let fst;
        (fst, self.node) = self.node.split_first()?;
        let fst = AstId {
            node: fst,
            loc: self.loc,
        };
        self.loc += 1;
        Some(fst)
    }
    pub fn pop_back(&mut self) -> Option<AstId<'a, T>> {
        let lst;
        (lst, self.node) = self.node.split_last()?;
        let lst = AstId {
            node: lst,
            loc: self.loc + self.len() - 1,
        };
        Some(lst)
    }

    pub fn single(id: AstId<'a, T>) -> Self {
        Self {
            node: std::slice::from_ref(id.node),
            loc: id.loc,
        }
    }

    pub fn truncate(self, len: usize) -> Self {
        Self {
            node: &self.node[..self.node.len().min(len)],
            loc: self.loc,
        }
    }
}

pub struct AstIdRangeIter<'a, T> {
    inner: std::slice::Iter<'a, T>,
    loc: usize,
}

impl<'a, T> Iterator for AstIdRangeIter<'a, T> {
    type Item = AstId<'a, T>;
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
impl<'a, T> DoubleEndedIterator for AstIdRangeIter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let node = self.inner.next_back()?;
        let id = AstId {
            node,
            loc: self.loc + self.inner.len(),
        };
        Some(id)
    }
}
impl<'a, T> ExactSizeIterator for AstIdRangeIter<'a, T> {}
impl<'a, T> IntoIterator for AstIdRange<'a, T> {
    type Item = AstId<'a, T>;
    type IntoIter = AstIdRangeIter<'a, T>;

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
pub struct Identifier(pub IdentId);

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 508
// hierarchical_identifier ::= { identifier [ [ constant_expression ] ] . } identifier
#[derive(Clone, Copy)]
pub struct HIdent<'a> {
    pub components: AstIdRange<'a, HIdentComponent<'a>>,
    pub ident: AstItem<Identifier>,
}
#[derive(Clone, Copy)]
pub struct HIdentComponent<'a> {
    pub ident: AstItem<Identifier>,
    pub constant_expr: Option<AstId<'a, ConstantExpr<'a>>>,
}

impl<'a> From<AstItem<Identifier>> for HIdent<'a> {
    fn from(ident: AstItem<Identifier>) -> Self {
        Self {
            components: AstIdRange::default(),
            ident,
        }
    }
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 507
// attribute_instance ::= (* attr_spec { , attr_spec } *)
#[derive(Clone, Copy)]
pub struct AttributeInstance<'a>(pub AstIdRange<'a, AttrSpec<'a>>);

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 507
// attr_spec ::= attr_name [ = constant_expression ]
// attr_name ::= identifier
#[derive(Clone, Copy)]
pub struct AttrSpec<'a> {
    pub attr_name: AstItem<Identifier>,
    pub constant_expression: Option<AstId<'a, ConstantExpr<'a>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizedNumber {
    pub inferred_size: bool,
    pub sign: Sign,
    pub base: Base,
    pub value: Bits,
}

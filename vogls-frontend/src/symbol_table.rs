use std::num::NonZeroU64;
use std::ops::{Index, IndexMut};

use crate::VgHashMap;
use crate::ident_table::IdentId;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SymbolId(NonZeroU64);

pub struct SymbolTable<T> {
    roots: Vec<SymbolId>,
    symbols: Vec<Symbol<T>>,
    lut: VgHashMap<(Option<SymbolId>, IdentId), SymbolId>,
}

impl<T> Default for SymbolTable<T> {
    fn default() -> Self {
        Self {
            roots: Vec::new(),
            symbols: Vec::new(),
            lut: VgHashMap::default(),
        }
    }
}

pub struct Symbol<T> {
    name: IdentId,
    pub content: T,
    parent: Option<SymbolId>,
    children: Vec<SymbolId>,
}

impl<T> Index<SymbolId> for SymbolTable<T> {
    type Output = Symbol<T>;
    fn index(&self, index: SymbolId) -> &Self::Output {
        &self.symbols[index.0.get() as usize]
    }
}
impl<T> IndexMut<SymbolId> for SymbolTable<T> {
    fn index_mut(&mut self, index: SymbolId) -> &mut Self::Output {
        &mut self.symbols[index.0.get() as usize]
    }
}

impl<T> SymbolTable<T> {
    fn insert_opt_parent(
        &mut self,
        name: IdentId,
        parent: Option<SymbolId>,
        content: T,
    ) -> Result<SymbolId, SymbolId> {
        let symid = self.insert_unlinked_child(name, parent, content);
        match self.lut.insert((parent, name), symid) {
            None => Ok(symid),
            Some(prev_symid) => Err(prev_symid),
        }
    }

    fn insert_unlinked_child(
        &mut self,
        name: IdentId,
        parent: Option<SymbolId>,
        content: T,
    ) -> SymbolId {
        self.symbols.push(Symbol {
            name,
            content,
            parent,
            children: Vec::new(),
        });
        SymbolId(NonZeroU64::new(self.symbols.len() as u64).unwrap())
    }

    pub fn insert_root(&mut self, name: IdentId, content: T) -> Result<SymbolId, SymbolId> {
        let symid = self.insert_opt_parent(name, None, content)?;
        self.roots.push(symid);
        Ok(symid)
    }

    pub fn insert(
        &mut self,
        name: IdentId,
        parent: SymbolId,
        content: T,
    ) -> Result<SymbolId, SymbolId> {
        let symid = self.insert_opt_parent(name, Some(parent), content)?;
        self.symbols[parent.0.get() as usize].children.push(symid);
        Ok(symid)
    }

    pub fn insert_unlinked(&mut self, name: IdentId, parent: SymbolId, content: T) -> SymbolId {
        let symid = self.insert_unlinked_child(name, Some(parent), content);
        self.symbols[parent.0.get() as usize].children.push(symid);
        symid
    }

    pub fn resolve(&self, scope: SymbolId, ident: IdentId) -> Option<SymbolId> {
        self.lut.get(&(Some(scope), ident)).copied()
    }
}

impl<T> Symbol<T> {
    pub fn name(&self) -> IdentId {
        self.name
    }

    pub fn parent(&self) -> Option<SymbolId> {
        self.parent
    }

    pub fn children(&self) -> &[SymbolId] {
        &self.children
    }
}

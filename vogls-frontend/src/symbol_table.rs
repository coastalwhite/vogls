use core::fmt;
use std::num::NonZeroU64;
use std::ops::{Index, IndexMut};

use vogls_ir::token_range::TokenRange;

use crate::VgHashMap;
use crate::ident_table::{IdentId, IdentTable};

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SymbolId(NonZeroU64);

/// A table that contains all symbols in a hierarchy.
pub struct SymbolTable<T> {
    roots: Vec<SymbolId>,
    symbols: Vec<Symbol<T>>,
    lut: VgHashMap<(Option<SymbolId>, IdentId), SymbolId>,
}

pub struct Symbol<T> {
    name: IdentId,
    origin: TokenRange,
    parent: Option<SymbolId>,
    children: Vec<SymbolId>,

    pub content: T,
}

pub struct SymbolTableDisplay<'a, T, F: Fn(&T, &mut fmt::Formatter<'_>) -> fmt::Result> {
    root: SymbolId,
    table: &'a SymbolTable<T>,
    ident_table: &'a IdentTable,
    f: F,
    indent: u64,
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

impl<T> Index<SymbolId> for SymbolTable<T> {
    type Output = Symbol<T>;
    fn index(&self, index: SymbolId) -> &Self::Output {
        &self.symbols[index.0.get() as usize - 1]
    }
}
impl<T> IndexMut<SymbolId> for SymbolTable<T> {
    fn index_mut(&mut self, index: SymbolId) -> &mut Self::Output {
        &mut self.symbols[index.0.get() as usize - 1]
    }
}

impl<T> SymbolTable<T> {
    fn insert_opt_parent(
        &mut self,
        name: IdentId,
        parent: Option<SymbolId>,
        origin: TokenRange,
        content: T,
    ) -> Result<SymbolId, SymbolId> {
        let symid = self.insert_unlinked_child(name, parent, origin, content);
        match self.lut.insert((parent, name), symid) {
            None => Ok(symid),
            Some(prev_symid) => Err(prev_symid),
        }
    }

    fn insert_unlinked_child(
        &mut self,
        name: IdentId,
        parent: Option<SymbolId>,
        origin: TokenRange,
        content: T,
    ) -> SymbolId {
        self.symbols.push(Symbol {
            name,
            origin,
            content,
            parent,
            children: Vec::new(),
        });
        SymbolId(NonZeroU64::new(self.symbols.len() as u64).unwrap())
    }

    pub fn insert_root(
        &mut self,
        name: IdentId,
        origin: TokenRange,
        content: T,
    ) -> Result<SymbolId, SymbolId> {
        let symid = self.insert_opt_parent(name, None, origin, content)?;
        self.roots.push(symid);
        Ok(symid)
    }

    pub fn insert(
        &mut self,
        name: IdentId,
        parent: SymbolId,
        origin: TokenRange,
        content: T,
    ) -> Result<SymbolId, SymbolId> {
        let symid = self.insert_opt_parent(name, Some(parent), origin, content)?;
        self[parent].children.push(symid);
        Ok(symid)
    }

    pub fn insert_unlinked(
        &mut self,
        name: IdentId,
        parent: SymbolId,
        origin: TokenRange,
        content: T,
    ) -> SymbolId {
        let symid = self.insert_unlinked_child(name, Some(parent), origin, content);
        self[parent].children.push(symid);
        symid
    }

    pub fn resolve(&self, scope: SymbolId, ident: IdentId) -> Option<SymbolId> {
        self.lut.get(&(Some(scope), ident)).copied()
    }

    pub fn display<'a, F: Fn(&T, &mut fmt::Formatter<'_>) -> fmt::Result>(
        &'a self,
        root: SymbolId,
        ident_table: &'a IdentTable,
        f: F,
    ) -> SymbolTableDisplay<'a, T, F> {
        SymbolTableDisplay {
            root,
            table: self,
            ident_table,
            f,
            indent: 0,
        }
    }

    pub fn symbol_iter(&self) -> impl Iterator<Item = &Symbol<T>> {
        self.symbols.iter()
    }

    pub fn symbol_id_iter(&self) -> impl Iterator<Item = SymbolId> + 'static {
        (0..self.symbols.len()).map(|s| SymbolId(NonZeroU64::new(s as u64 + 1).unwrap()))
    }

    pub fn pop_last_inserted(&mut self, symbol: SymbolId) {
        assert_eq!(self.symbols.len(), symbol.0.get() as usize);

        if let Some(parent) = self[symbol].parent {
            assert_eq!(Some(symbol), self[parent].children.pop());
        };
        self.symbols.pop();
    }
}

impl<T> Symbol<T> {
    pub fn name(&self) -> IdentId {
        self.name
    }

    pub fn origin(&self) -> TokenRange {
        self.origin
    }

    pub fn parent(&self) -> Option<SymbolId> {
        self.parent
    }

    pub fn children(&self) -> &[SymbolId] {
        &self.children
    }
}

impl<'a, T, F: Fn(&T, &mut fmt::Formatter<'_>) -> fmt::Result + Copy> fmt::Display
    for SymbolTableDisplay<'a, T, F>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for _ in 0..self.indent {
            f.write_str("  ")?;
        }
        write!(f, "{}: ", &self.ident_table[self.table[self.root].name])?;
        (self.f)(&self.table[self.root].content, f)?;
        writeln!(f)?;
        for child in self.table[self.root].children() {
            SymbolTableDisplay {
                root: *child,
                table: self.table,
                ident_table: self.ident_table,
                f: self.f,
                indent: self.indent + 1,
            }
            .fmt(f)?;
        }
        Ok(())
    }
}

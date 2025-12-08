use std::collections::HashMap;
use std::ops::{Index, IndexMut};

use vogls_ir::{SignalKey, Type, VariableKey};

use crate::parser::TokenRange;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolKey(usize);
#[derive(Debug, Clone, Default)]
pub struct SymbolTable(Vec<Symbol>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Symbol {
    pub name: String,
    pub definition_site: TokenRange,
    pub ty: Type,
    pub variant: SymbolVariant,
}

impl Index<SymbolKey> for SymbolTable {
    type Output = Symbol;
    fn index(&self, index: SymbolKey) -> &Self::Output {
        &self.0[index.0]
    }
}

impl IndexMut<SymbolKey> for SymbolTable {
    fn index_mut(&mut self, index: SymbolKey) -> &mut Self::Output {
        &mut self.0[index.0]
    }
}

impl SymbolTable {
    pub fn insert(&mut self, symbol: Symbol) -> SymbolKey {
        let key = SymbolKey(self.0.len());
        self.0.push(symbol);
        key
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SymbolVariant {
    Variable(Option<VariableKey>),
    Signal(SignalKey),
}

#[derive(Debug, Clone)]
pub struct Scope<'a> {
    look_up: HashMap<&'a str, Vec<(usize, SymbolKey)>>,
    pub symbols: SymbolTable,

    scope_stack: Vec<&'a str>,
    scope_stack_offsets: Vec<usize>,
}

impl<'a> Default for Scope<'a> {
    fn default() -> Self {
        Self {
            look_up: Default::default(),
            symbols: SymbolTable::default(),
            scope_stack: Default::default(),
            scope_stack_offsets: Default::default(),
        }
    }
}

impl Scope<'_> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'a> Scope<'a> {
    pub fn push_scope(&mut self) {
        self.scope_stack_offsets.push(self.scope_stack.len());
    }

    pub fn pop_scope(&mut self) {
        let offset = self.scope_stack_offsets.pop().unwrap_or(0);
        for k in self.scope_stack.drain(offset..) {
            self.look_up.get_mut(k).unwrap().pop().unwrap();
        }
    }

    pub fn push(&mut self, key: &'a str, value: SymbolKey) {
        let variable_stack = self.look_up.entry(key).or_default();
        if let Some((scope, current_value)) = variable_stack.last_mut() {
            // Shadow existing variables.
            if *scope == self.scope_stack_offsets.len() {
                *current_value = value;
                return;
            }
        }

        self.scope_stack.push(key);
        variable_stack.push((self.scope_stack_offsets.len(), value));
    }

    pub fn get(&self, key: &str) -> Option<SymbolKey> {
        let values = self.look_up.get(key)?;
        Some(values.last()?.1)
    }
}

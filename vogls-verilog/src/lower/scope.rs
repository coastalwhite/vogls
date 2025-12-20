use std::collections::HashMap;
use std::ops::{Index, IndexMut};

use vogls_ir::{SignalKey, VariableKey};

use crate::parser::TokenRange;

use super::VTypeKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolKey(usize);
#[derive(Debug, Clone, Default)]
pub struct SymbolTable(Vec<Symbol>);
#[derive(Debug, Clone, Default)]
pub struct ScopeVariables(Vec<Vec<(usize, VariableKey)>>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Symbol {
    pub name: String,
    pub definition_site: TokenRange,
    pub ty: VTypeKey,
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

impl Index<SymbolKey> for ScopeVariables {
    type Output = Vec<(usize, VariableKey)>;
    fn index(&self, index: SymbolKey) -> &Self::Output {
        &self.0[index.0]
    }
}

impl IndexMut<SymbolKey> for ScopeVariables {
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
    Genvar(Option<i64>),
    Constant(Option<i64>),
    Variable(Option<VariableKey>),
    Signal(SignalKey),
}

#[derive(Debug, Clone)]
pub struct Scope<'a> {
    look_up: HashMap<&'a str, Vec<(usize, SymbolKey)>>,
    pub symbols: SymbolTable,
    pub scope_variables: ScopeVariables,

    scope_stack: Vec<&'a str>,
    scope_stack_offsets: Vec<usize>,

    scope_assigns: Vec<SymbolKey>,
    scope_assigns_offsets: Vec<usize>,
}

impl<'a> Default for Scope<'a> {
    fn default() -> Self {
        Self {
            look_up: Default::default(),
            symbols: SymbolTable::default(),
            scope_variables: Default::default(),
            scope_stack: Default::default(),
            scope_stack_offsets: Default::default(),
            scope_assigns: Default::default(),
            scope_assigns_offsets: Default::default(),
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
        self.scope_assigns_offsets.push(self.scope_assigns.len());
    }

    pub fn scope_assigned_symbols(&self) -> impl Iterator<Item = (SymbolKey, VariableKey)> {
        let offset = self.scope_assigns_offsets.last().copied().unwrap_or(0);
        self.scope_assigns[offset..]
            .iter()
            .map(|s| (*s, self.scope_variables[*s].last().unwrap().1))
    }

    pub fn pop_scope<'b>(&'b mut self) {
        let offset = self.scope_stack_offsets.pop().unwrap_or(0);
        for k in self.scope_stack.drain(offset..) {
            self.look_up.get_mut(k).unwrap().pop().unwrap();
        }

        let offset = self.scope_assigns_offsets.pop().unwrap_or(0);
        for s in self.scope_assigns.drain(offset..) {
            self.scope_variables[s].pop();
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

    pub fn assign(&mut self, key: SymbolKey, value: VariableKey) {
        let SymbolVariant::Variable(v) = &mut self.symbols[key].variant else {
            panic!();
        };
        *v = Some(value);
        self.scope_assigns.push(key);

        let scope_offset = self.scope_assigns_offsets.len();

        if self.scope_variables.0.len() <= key.0 {
            self.scope_variables.0.extend(std::iter::repeat_n(
                Vec::new(),
                key.0 - self.scope_variables.0.len() + 1,
            ));
        }
        let variables = &mut self.scope_variables[key];
        match variables.last_mut() {
            Some((o, v)) if *o == scope_offset => *v = value,
            _ => variables.push((scope_offset, value)),
        }
    }
}

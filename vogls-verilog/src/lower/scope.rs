use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Scope<K, V> {
    look_up: HashMap<K, Vec<(usize, V)>>,

    scope_stack: Vec<K>,
    scope_stack_offsets: Vec<usize>,
}

impl<K, V> Default for Scope<K, V> {
    fn default() -> Self {
        Self {
            look_up: Default::default(),
            scope_stack: Default::default(),
            scope_stack_offsets: Default::default(),
        }
    }
}

impl<K, V> Scope<K, V> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<K: std::hash::Hash + Eq + Clone, V> Scope<K, V> {
    #[expect(unused)]
    pub fn push_scope(&mut self) {
        self.scope_stack_offsets.push(self.scope_stack.len());
    }

    #[expect(unused)]
    pub fn pop_scope(&mut self) {
        let offset = self.scope_stack_offsets.pop().unwrap_or(0);
        for k in self.scope_stack.drain(offset..) {
            self.look_up.get_mut(&k).unwrap().pop().unwrap();
        }
    }

    pub fn push(&mut self, key: K, value: V) {
        let variable_stack = self.look_up.entry(key.clone()).or_default();
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

    pub fn get(&mut self, key: &K) -> Option<&V> {
        let values = self.look_up.get(key)?;
        Some(&values.last()?.1)
    }
}

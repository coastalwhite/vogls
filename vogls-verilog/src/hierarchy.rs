use std::collections::HashMap;
use std::fmt;
use std::num::NonZeroUsize;

use vogls_ir::SignalKey;
use vogls_ir::vcd::{NetType, VcdScope, VcdScopeItem, VcdVariable};

#[derive(Clone, PartialEq, Eq)]
pub struct Hierarchy {
    modules: Vec<ModuleInstance>,
    module_children: Vec<ModuleChild>,
    lookup_table: HashMap<(ModuleKey, String), ModuleChildKey>,
}

pub struct ModuleBuilder<'a> {
    hierarchy: &'a mut Hierarchy,
    module_key: ModuleKey,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ModuleInstance {
    instance_name: String,
    module_name: String,
    parent: Option<ModuleKey>,
    children: ModuleChildRange,
}

impl ModuleInstance {
    fn vcd_scope(&self, hierarchy: &Hierarchy, remaining_levels: u32) -> VcdScope {
        VcdScope {
            name: self.instance_name.clone(),
            items: (self.children.start..self.children.end)
                .filter_map(|i| match &hierarchy.module_children[i] {
                    ModuleChild::Net(net) => Some(VcdScopeItem::Variable(VcdVariable {
                        signal: net.signal,
                        ty: net.ty,
                        msb: net.msb,
                        lsb: net.lsb,
                    })),
                    ModuleChild::Module(_) if remaining_levels == 0 => None,
                    ModuleChild::Module(module) => Some(VcdScopeItem::Scope(
                        hierarchy.modules[module.as_idx()]
                            .vcd_scope(hierarchy, remaining_levels - 1),
                    )),
                })
                .collect(),
        }
    }
}

pub struct ModuleInstanceDisplay<'a> {
    module: ModuleKey,
    hierarchy: &'a Hierarchy,
    indent: u32,
}

pub struct NetDisplay<'a>(&'a Net);

impl ModuleInstance {
    fn flat_fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mod {}: {}", self.module_name, self.instance_name)
    }
}

const SPACES_PER_INDENT: usize = 2;

impl<'a> fmt::Display for ModuleInstanceDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let indent = self.indent as usize * SPACES_PER_INDENT;
        write!(f, "{:indent$}", "")?;
        let instance = &self.hierarchy.modules[self.module.as_idx()];
        instance.flat_fmt(f)?;
        writeln!(f)?;
        for child in instance.children.start..instance.children.end {
            match &self.hierarchy.module_children[child] {
                ModuleChild::Net(net) => {
                    let indent = indent + SPACES_PER_INDENT;
                    write!(f, "{:indent$}", " ")?;
                    net.display().fmt(f)?;
                    writeln!(f)?;
                }
                ModuleChild::Module(key) => {
                    ModuleInstanceDisplay {
                        module: *key,
                        hierarchy: self.hierarchy,
                        indent: self.indent + 1,
                    }
                    .fmt(f)?;
                }
            }
        }
        Ok(())
    }
}

impl<'a> fmt::Display for NetDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Net {
            name,
            signal: _,
            parent: _,
            ty,
            msb,
            lsb,
        } = &self.0;
        let (ty, show_msb_lsb) = match ty {
            NetType::Integer => ("integer", false),
            NetType::Register => ("reg", true),
            NetType::Wire => ("wire", true),
        };

        if show_msb_lsb && msb != lsb {
            write!(f, "{ty} [{msb}:{lsb}] {name}")
        } else {
            write!(f, "{ty} {name}")
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleChild {
    Net(Net),
    Module(ModuleKey),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Net {
    name: String,
    signal: SignalKey,
    parent: ModuleKey,
    ty: NetType,
    msb: i64,
    lsb: i64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ModuleChildRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModuleKey(NonZeroUsize);
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ModuleChildKey(NonZeroUsize);

impl ModuleKey {
    fn as_idx(self) -> usize {
        self.0.get() - 1
    }

    fn new(offset: usize) -> Self {
        Self(NonZeroUsize::new(offset + 1).expect("ModuleKey overflow"))
    }
}
impl ModuleChildKey {
    fn as_idx(self) -> usize {
        self.0.get() - 1
    }

    fn new(offset: usize) -> Self {
        Self(NonZeroUsize::new(offset + 1).expect("ModuleChildKey overflow"))
    }
}

impl Hierarchy {
    pub fn new(top_level_name: String) -> Self {
        let top_level = ModuleInstance {
            instance_name: top_level_name.clone(),
            module_name: top_level_name.clone(),
            parent: None,
            children: ModuleChildRange { start: 0, end: 0 },
        };
        Self {
            modules: vec![top_level],
            module_children: Vec::new(),
            lookup_table: HashMap::new(),
        }
    }

    pub fn top_level_key(&self) -> ModuleKey {
        ModuleKey::new(0)
    }

    pub fn vcd_scope(&self, remaining_levels: u32) -> VcdScope {
        let top_level_key = self.top_level_key();
        self.modules[top_level_key.as_idx()].vcd_scope(&self, remaining_levels)
    }

    pub fn display<'b>(&'b self, module: ModuleKey) -> ModuleInstanceDisplay<'b> {
        ModuleInstanceDisplay {
            module,
            hierarchy: self,
            indent: 0,
        }
    }
}

impl<'a> ModuleBuilder<'a> {
    pub fn new(hierarchy: &'a mut Hierarchy, module_key: ModuleKey) -> Self {
        hierarchy.modules[module_key.as_idx()].children.start = hierarchy.module_children.len();
        hierarchy.modules[module_key.as_idx()].children.end = hierarchy.module_children.len();
        Self {
            hierarchy,
            module_key,
        }
    }

    pub fn move_to(&mut self, module_key: ModuleKey) {
        self.module_key = module_key;
        self.hierarchy.modules[module_key.as_idx()].children.start =
            self.hierarchy.module_children.len();
        self.hierarchy.modules[module_key.as_idx()].children.end =
            self.hierarchy.module_children.len();
    }

    pub fn push_net(&mut self, name: String, signal: SignalKey, ty: NetType, msb: i64, lsb: i64) {
        let child_key = self.push_child(ModuleChild::Net(Net {
            name: name.clone(),
            signal,
            parent: self.module_key,
            ty,
            msb,
            lsb,
        }));
        self.hierarchy
            .lookup_table
            .insert((self.module_key, name.clone()), child_key);
    }

    pub fn push_module_instance(
        &mut self,
        module_name: String,
        instance_name: String,
    ) -> ModuleKey {
        let key = ModuleKey::new(self.hierarchy.modules.len());
        let child_key = self.push_child(ModuleChild::Module(key));
        self.hierarchy
            .lookup_table
            .insert((self.module_key, instance_name.clone()), child_key);
        self.hierarchy.modules.push(ModuleInstance {
            instance_name,
            module_name,
            parent: Some(self.module_key),
            children: ModuleChildRange { start: 0, end: 0 },
        });
        key
    }

    fn push_child(&mut self, child: ModuleChild) -> ModuleChildKey {
        let key = ModuleChildKey::new(self.hierarchy.module_children.len());
        self.hierarchy.module_children.push(child);
        self.hierarchy.modules[self.module_key.as_idx()]
            .children
            .end += 1;
        key
    }
}

impl Net {
    pub fn display<'a>(&'a self) -> NetDisplay<'a> {
        NetDisplay(self)
    }
}

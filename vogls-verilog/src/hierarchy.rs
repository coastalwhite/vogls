use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::num::NonZeroUsize;

use vogls_ir::vcd::VcdScope;
use vogls_ir::{ConnectionDirection, SignalKey};

use crate::ast::AstId;
use crate::ast::module::{FunctionDeclaration, ModuleInstance, TaskDeclaration};
use crate::ast::statement::SeqBlock;
use crate::lower::{EvalScope, VType, VValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HierarchyKey(NonZeroUsize);

#[derive(Clone)]
pub struct Hierarchy {
    pub symbols: Vec<HierarchyItem>,
    pub lookup_table: HashMap<(HierarchyKey, String), HierarchyKey>,

    pub modules: Vec<HierarchyModule>,
    pub named_blocks: Vec<HierarchyNamedBlock>,
    pub tasks: Vec<HierarchyTask>,
    pub functions: Vec<HierarchyFunction>,
    pub nets: Vec<HierarchyNet>,
    pub parameters: Vec<HierarchyParameter>,
    pub genvars: Vec<HierarchyGenvar>,
}

#[derive(Clone)]
pub struct HierarchyModule {
    pub name: String,
    pub module_name: String,
    pub children: HierarchyItemRange,

    pub ast: Option<AstId<ModuleInstance>>,
    pub parent: Option<HierarchyKey>,

    // @TODO: Do something smarter here.
    pub lut: HashMap<String, usize>,
    pub ports: Vec<(usize, ConnectionDirection)>,
    pub parameter_lut: HashMap<String, usize>,
    pub parameters: Vec<usize>,
    pub parameter_overrides: Option<ParameterOverrides>,
}

#[derive(Clone)]
pub enum ParameterOverrides {
    Ordered(Vec<VValue>),

    // @TODO: Make consistently ordered.
    Named(HashMap<String, VValue>),
}

#[derive(Clone)]
pub struct HierarchyNamedBlock {
    pub name: String,
    pub ast: AstId<SeqBlock>,
    pub children: HierarchyItemRange,
    pub parent: HierarchyKey,
}

#[derive(Clone)]
pub struct HierarchyTask {
    pub name: String,
    pub ast: AstId<TaskDeclaration>,
    pub children: HierarchyItemRange,
    pub parent: HierarchyKey,
}

#[derive(Clone)]
pub struct HierarchyFunction {
    pub name: String,
    pub ast: AstId<FunctionDeclaration>,
    pub children: HierarchyItemRange,
    pub parent: HierarchyKey,
}

#[derive(Clone, PartialEq, Eq)]
pub struct HierarchyNet {
    pub name: String,
    pub parent: HierarchyKey,
    pub signal: SignalKey,
    pub ty: VType,
    pub dims: Box<[u32]>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct HierarchyParameter {
    pub name: String,
    pub parent: HierarchyKey,
    pub value: VValue,
}

#[derive(Clone, PartialEq, Eq)]
pub struct HierarchyGenvar {
    pub name: String,
    pub parent: HierarchyKey,

    // @TODO: Don't use a RefCell here.
    pub value: RefCell<VValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HierarchyItem {
    Module(usize),
    NamedBlock(usize),
    Task(usize),
    Function(usize),
    Net(usize),
    Parameter(usize),
    GenVar(usize),
}

impl HierarchyItem {
    fn name<'a>(&self, hierarchy: &'a Hierarchy) -> &'a str {
        use HierarchyItem as I;
        match self {
            I::Module(i) => &hierarchy.modules[*i].name,
            I::NamedBlock(i) => &hierarchy.named_blocks[*i].name,
            I::Task(i) => &hierarchy.tasks[*i].name,
            I::Function(i) => &hierarchy.functions[*i].name,
            I::Net(i) => &hierarchy.nets[*i].name,
            I::Parameter(i) => &hierarchy.parameters[*i].name,
            I::GenVar(i) => &hierarchy.genvars[*i].name,
        }
    }

    fn children(&self, hierarchy: &Hierarchy) -> HierarchyItemRange {
        use HierarchyItem as I;
        match self {
            I::Module(i) => hierarchy.modules[*i].children,
            I::NamedBlock(i) => hierarchy.named_blocks[*i].children,
            I::Task(i) => hierarchy.tasks[*i].children,
            I::Function(i) => hierarchy.functions[*i].children,
            I::Net(_) | I::Parameter(_) | I::GenVar(_) => HierarchyItemRange { start: 0, end: 0 },
        }
    }

    fn children_mut(self, hierarchy: &mut Hierarchy) -> Option<&mut HierarchyItemRange> {
        use HierarchyItem as I;
        match self {
            I::Module(i) => Some(&mut hierarchy.modules[i].children),
            I::NamedBlock(i) => Some(&mut hierarchy.named_blocks[i].children),
            I::Task(i) => Some(&mut hierarchy.tasks[i].children),
            I::Function(i) => Some(&mut hierarchy.functions[i].children),
            I::Net(_) | I::Parameter(_) | I::GenVar(_) => None,
        }
    }

    fn parent<'a>(&self, hierarchy: &'a Hierarchy) -> Option<HierarchyKey> {
        use HierarchyItem as I;
        match self {
            I::Module(i) => hierarchy.modules[*i].parent,
            I::NamedBlock(i) => Some(hierarchy.named_blocks[*i].parent),
            I::Task(i) => Some(hierarchy.tasks[*i].parent),
            I::Function(i) => Some(hierarchy.functions[*i].parent),
            I::Net(i) => Some(hierarchy.nets[*i].parent),
            I::Parameter(i) => Some(hierarchy.parameters[*i].parent),
            I::GenVar(i) => Some(hierarchy.genvars[*i].parent),
        }
    }

    fn vcd_scope(&self, hierarchy: &Hierarchy, remaining_levels: u32) -> VcdScope {
        todo!()
        // use HierarchyItem as I;
        // VcdScope {
        //     name: self.name().to_string(),
        //     items: todo!(),
        //     // self
        //     // .children()
        //     // .iter()
        //     // .filter_map(|i| match &hierarchy.items[i.as_idx()] {
        //     //     I::Module { instance_name, module_name, children, parent } => {
        //     //     },
        //     //     I::NamedBlock { instance_name, module_name, children, parent } => {
        //     //     },
        //     //     I::Task { instance_name, module_name, children, parent } => {
        //     //     },
        //     //     I::Function { instance_name, module_name, children, parent } => {
        //     //     },
        //     //     I::Net { signal, .. } => {
        //     //         Some(VcdScopeItem::Variable(VcdVariable {
        //     //             signal: net.signal,
        //     //             ty: net.ty,
        //     //             msb: net.msb,
        //     //             lsb: net.lsb,
        //     //         }))
        //     //     },
        //     //     I::Parameter { .. } => None,
        //     //     // ScopeItem::Net(net) => {
        //     //     // }
        //     //     // ScopeItem::Scope(_) if remaining_levels == 0 => None,
        //     //     // ScopeItem::Scope(module) => Some(VcdScopeItem::Scope(
        //     //     //     hierarchy.items[module.as_idx()].vcd_scope(hierarchy, remaining_levels - 1),
        //     //     // )),
        //     // })
        //     // .collect(),
        // }
    }
}

impl HierarchyItem {
    fn flat_fmt(&self, f: &mut fmt::Formatter<'_>, hierarchy: &Hierarchy) -> fmt::Result {
        use HierarchyItem as I;
        match &self {
            I::Module(i) => {
                let HierarchyModule {
                    name: instance_name,
                    module_name,
                    children: _,
                    ast: _,
                    parent: _,
                    lut: _,
                    ports,
                    parameter_lut: _,
                    parameters: _,
                    parameter_overrides: _,
                } = &hierarchy.modules[*i];
                write!(f, "mod {module_name}: {instance_name}")
            }
            I::NamedBlock(i) => {
                let HierarchyNamedBlock {
                    name,
                    ast: _,
                    children: _,
                    parent: _,
                } = &hierarchy.named_blocks[*i];
                write!(f, "named_block {name}")
            }
            I::Task(i) => {
                let HierarchyTask {
                    name,
                    ast: _,
                    children: _,
                    parent: _,
                } = &hierarchy.tasks[*i];
                write!(f, "tasks {name}")
            }
            I::Function(i) => {
                let HierarchyFunction {
                    name,
                    ast: _,
                    children: _,
                    parent: _,
                } = &hierarchy.functions[*i];
                write!(f, "function {name}")
            }
            I::Net(i) => {
                let HierarchyNet {
                    name,
                    parent: _,
                    signal: _,
                    ty,
                    dims,
                } = &hierarchy.nets[*i];
                f.write_str("net ")?;
                if ty.is_signed() || ty.force_net_width().get() != 1 {
                    match ty {
                        VType::SignedNet(s) => write!(f, "i{} ", s.get()),
                        VType::UnsignedNet(s) => write!(f, "u{} ", s.get()),
                        VType::String(_) => f.write_str("str "),
                    }?;
                }
                f.write_str(name)?;
                for d in dims {
                    write!(f, "[{d}]")?;
                }

                Ok(())
            }
            I::Parameter(i) => {
                let HierarchyParameter {
                    name,
                    parent: _,
                    value: _,
                } = &hierarchy.parameters[*i];
                write!(f, "parameter {name}")
            }
            I::GenVar(i) => {
                let HierarchyGenvar {
                    name,
                    parent: _,
                    value: _,
                } = &hierarchy.genvars[*i];
                write!(f, "genvar {name}")
            }
        }
    }
}

pub struct HierarchyPathDisplay<'a> {
    key: HierarchyKey,
    hierarchy: &'a Hierarchy,
}

pub struct HierarchyDisplay<'a> {
    key: HierarchyKey,
    hierarchy: &'a Hierarchy,
    indent: u32,
}

const SPACES_PER_INDENT: usize = 2;

impl<'a> fmt::Display for HierarchyDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let indent = self.indent as usize * SPACES_PER_INDENT;
        write!(f, "{:indent$}", "")?;
        let item = &self.hierarchy.symbols[self.key.as_idx()];
        item.flat_fmt(f, self.hierarchy)?;
        writeln!(f)?;
        for key in item.children(self.hierarchy).iter() {
            HierarchyDisplay {
                key,
                hierarchy: self.hierarchy,
                indent: self.indent + 1,
            }
            .fmt(f)?;
        }
        Ok(())
    }
}

impl<'a> fmt::Display for HierarchyPathDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let key = self.key;
        if let Some(parent) = self.hierarchy.items()[key.as_idx()].parent(self.hierarchy) {
            write!(f, "{}.", self.hierarchy.path_display(parent))?;
        }
        f.write_str(self.hierarchy.items()[key.as_idx()].name(self.hierarchy))?;
        Ok(())
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct HierarchyItemRange {
    pub start: usize,
    pub end: usize,
}

impl HierarchyItemRange {
    pub fn iter(self) -> impl ExactSizeIterator<Item = HierarchyKey> + DoubleEndedIterator {
        (self.start..self.end).map(|i| HierarchyKey::new(i))
    }
}

impl HierarchyKey {
    pub fn as_idx(self) -> usize {
        self.0.get() - 1
    }

    pub fn new(offset: usize) -> Self {
        Self(NonZeroUsize::new(offset + 1).expect("HierarchyKey overflow"))
    }
}
impl Hierarchy {
    pub fn new(top_level_name: String) -> Self {
        let mut slf = Self {
            symbols: vec![HierarchyItem::Module(0)],
            lookup_table: HashMap::new(),
            modules: Vec::new(),
            named_blocks: Vec::new(),
            tasks: Vec::new(),
            functions: Vec::new(),
            nets: Vec::new(),
            parameters: Vec::new(),
            genvars: Vec::new(),
        };
        slf.modules.push(HierarchyModule {
            name: top_level_name.clone(),
            module_name: top_level_name,
            children: HierarchyItemRange { start: 1, end: 1 },
            ast: None,
            parent: None,
            lut: Default::default(),
            ports: Default::default(),
            parameter_lut: Default::default(),
            parameters: Default::default(),
            parameter_overrides: None,
        });
        slf
    }

    pub fn top_level_key(&self) -> HierarchyKey {
        HierarchyKey::new(0)
    }

    pub fn vcd_scope(&self, remaining_levels: u32) -> VcdScope {
        let top_level_key = self.top_level_key();
        self.symbols[top_level_key.as_idx()].vcd_scope(&self, remaining_levels)
    }

    pub fn display<'b>(&'b self, key: HierarchyKey) -> HierarchyDisplay<'b> {
        HierarchyDisplay {
            key,
            hierarchy: self,
            indent: 0,
        }
    }

    pub fn path_display<'b>(&'b self, key: HierarchyKey) -> HierarchyPathDisplay<'b> {
        HierarchyPathDisplay {
            key,
            hierarchy: self,
        }
    }

    pub fn items(&self) -> &[HierarchyItem] {
        &self.symbols
    }
    pub fn modules(&self) -> &[HierarchyModule] {
        &self.modules
    }
    pub fn named_blocks(&self) -> &[HierarchyNamedBlock] {
        &self.named_blocks
    }
    pub fn tasks(&self) -> &[HierarchyTask] {
        &self.tasks
    }
    pub fn function(&self) -> &[HierarchyFunction] {
        &self.functions
    }
    pub fn net(&self) -> &[HierarchyNet] {
        &self.nets
    }
    pub fn parameters(&self) -> &[HierarchyParameter] {
        &self.parameters
    }
    pub fn genvars(&self) -> &[HierarchyGenvar] {
        &self.genvars
    }

    pub(crate) fn lookup(&self) -> &HashMap<(HierarchyKey, String), HierarchyKey> {
        &self.lookup_table
    }
}

pub struct ScopeBuilder<'a> {
    pub hierarchy: &'a mut Hierarchy,
    pub key: HierarchyKey,
}

macro_rules! insert_fn {
    ($f:ident, $struct:ty, $field:tt, $v:ident, $item:ident) => {
        pub fn $f(&mut self, $v: $struct) -> Option<HierarchyKey> {
            let i = self.hierarchy.$field.len();
            let item_key = HierarchyKey::new(self.hierarchy.symbols.len());
            let name = $v.name.clone();
            self.hierarchy.$field.push($v);

            let items_len = self.hierarchy.symbols.len();
            let children = self.hierarchy.symbols[self.key.as_idx()]
                .children_mut(self.hierarchy)
                .unwrap();
            assert_eq!(children.end, items_len);
            children.end += 1;

            self.hierarchy.symbols.push(HierarchyItem::$item(i));
            self.hierarchy
                .lookup_table
                .insert((self.key, name), item_key)
        }
    };
}

impl<'a> ScopeBuilder<'a> {
    insert_fn!(insert_module, HierarchyModule, modules, module, Module);
    insert_fn!(insert_net, HierarchyNet, nets, net, Net);
    insert_fn!(
        insert_parameter,
        HierarchyParameter,
        parameters,
        parameter,
        Parameter
    );
    insert_fn!(insert_task, HierarchyTask, tasks, task, Task);
    insert_fn!(
        insert_function,
        HierarchyFunction,
        functions,
        function,
        Function
    );
    insert_fn!(
        insert_named_block,
        HierarchyNamedBlock,
        named_blocks,
        named_block,
        NamedBlock
    );

    pub fn key(&self) -> HierarchyKey {
        self.key
    }

    pub fn eval_scope<'b>(&'b self) -> EvalScope<'b> {
        EvalScope {
            hierarchy: self.hierarchy,
            key: self.key,
        }
    }
}

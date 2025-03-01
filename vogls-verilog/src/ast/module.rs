use super::statement::Statement;
use super::{AstId, AstIdRange, AstItem, Identifier};


// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 487
// module_declaration ::=
// { attribute_instance } module_keyword module_identifier [ module_parameter_port_list ]
// list_of_ports ; { module_item }
// endmodule
// | { attribute_instance } module_keyword module_identifier [ module_parameter_port_list ]
// [ list_of_port_declarations ] ; { non_port_module_item }
// endmodule
#[derive(Clone, Copy)]
pub struct Module {
    pub module_identifier: AstItem<Identifier>,
    pub module_items: AstIdRange<NonPortModuleItem>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
// module_or_generate_item ::=
// { attribute_instance } module_or_generate_item_declaration
// | { attribute_instance } local_parameter_declaration ;
// | { attribute_instance } parameter_override
// | { attribute_instance } continuous_assign
// | { attribute_instance } gate_instantiation
// | { attribute_instance } udp_instantiation
// | { attribute_instance } module_instantiation
// | { attribute_instance } initial_construct
// | { attribute_instance } always_construct
// | { attribute_instance } loop_generate_construct
// | { attribute_instance } conditional_generate_construct
#[derive(Clone, Copy)]
pub enum ModuleOrGenerateItem {
    ModuleOrGenerateItemDeclaration,
    LocalParameterDeclaration,
    ParameterOverride,
    ContinuousAssign,
    GateInstantiation,
    UdpInstantiation,
    ModuleInstantiation,
    InitialConstruct(AstId<InitialConstruct>),
    AlwaysConstruct(AstId<AlwaysConstruct>),
    LoopGenerateConstruct,
    ConditionalGenerateConstruct,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// initial_construct ::= initial statement
#[derive(Clone, Copy)]
pub struct InitialConstruct(pub AstId<Statement>);

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// always_construct ::= always statement
#[derive(Clone, Copy)]
pub struct AlwaysConstruct(pub AstId<Statement>);

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
// non_port_module_item ::=
// module_or_generate_item
// | generate_region
// | specify_block
// | { attribute_instance } parameter_declaration ;
// | { attribute_instance } specparam_declaration
#[derive(Clone, Copy)]
pub enum NonPortModuleItem {
    ModuleOrGenerateItem(AstId<ModuleOrGenerateItem>),
    GenerateRegion,
    SpecifyBlock,
    ParameterDeclaration,
    SpecParamDeclaration,
}


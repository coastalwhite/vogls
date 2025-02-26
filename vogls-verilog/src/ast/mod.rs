use crate::arena::{ArenaId, ArenaIdRange};
use crate::ident::Ident;

pub mod expr;

#[derive(Clone, Copy)]
pub struct AstId<T> {
    pub node: ArenaId<T>,
    pub loc: usize,
}
#[derive(Clone, Copy)]
pub struct AstIdRange<T> {
    pub node: ArenaIdRange<T>,
    pub loc: usize,
}


// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 487
// module_declaration ::=
// { attribute_instance } module_keyword module_identifier [ module_parameter_port_list ]
// list_of_ports ; { module_item }
// endmodule
// | { attribute_instance } module_keyword module_identifier [ module_parameter_port_list ]
// [ list_of_port_declarations ] ; { non_port_module_item }
// endmodule
pub struct Module {
    name: AstId<Ident>,
    module_items: AstIdRange<NonPortModuleItem>,
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
pub struct InitialConstruct(AstId<Statement>);

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// always_construct ::= always statement
pub struct AlwaysConstruct(AstId<Statement>);

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
// non_port_module_item ::=
// module_or_generate_item
// | generate_region
// | specify_block
// | { attribute_instance } parameter_declaration ;
// | { attribute_instance } specparam_declaration
pub enum NonPortModuleItem {
    ModuleOrGenerateItem(AstId<ModuleOrGenerateItem>),
    GenerateRegion,
    SpecifyBlock,
    ParameterDeclaration,
    SpecParamDeclaration,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
// statement ::=
//   { attribute_instance } blocking_assignment ;
//   | { attribute_instance } case_statement
//   | { attribute_instance } conditional_statement
//   | { attribute_instance } disable_statement
//   | { attribute_instance } event_trigger
//   | { attribute_instance } loop_statement
//   | { attribute_instance } nonblocking_assignment ;
//   | { attribute_instance } par_block
//   | { attribute_instance } procedural_continuous_assignments ;
//   | { attribute_instance } procedural_timing_control_statement
//   | { attribute_instance } seq_block
//   | { attribute_instance } system_task_enable
//   | { attribute_instance } task_enable
//   | { attribute_instance } wait_statement
pub enum Statement {
    BlockingAssignment,
    CaseStatement,
    ConditionalStatement,
    DisableStatement,
    EventTrigger,
    LoopStatement,
    NonBlockingAssignment,
    ParBlock,
    ProceduralContinuousAssignments,
    ProceduralTimingControlStatement,
    SeqBlock(AstId<SeqBlock>),
    SystemTaskEnable,
    TaskEnable,
    WaitStatement,
}

pub struct AttributeInstance;


// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
// seq_block ::= begin [ : block_identifier { block_item_declaration } ] { statement } end
pub struct SeqBlock {
}

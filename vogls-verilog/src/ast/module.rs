use super::constant_expr::{ConstantExpr, ConstantMinTypMaxExpression};
use super::expr::Expr;
use super::statement::{NetLValue, Statement};
use super::{AstId, AstIdRange, AstItem, AttributeInstance, Identifier};

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 487
// module_declaration ::=
//   { attribute_instance } module_keyword module_identifier [ module_parameter_port_list ] list_of_ports ; { module_item }
//   endmodule
// | { attribute_instance } module_keyword module_identifier [ module_parameter_port_list ] [ list_of_port_declarations ] ; { non_port_module_item }
//   endmodule
#[derive(Clone, Copy)]
pub struct Module {
    pub attribute_instances: AstIdRange<AttributeInstance>,
    pub module_identifier: AstItem<Identifier>,
    pub module_parameter_port_list: Option<AstIdRange<ParameterDeclaration>>,
    pub ports: ModulePorts,
    pub module_items: AstIdRange<ModuleItem>,
}

#[derive(Clone, Copy)]
pub enum ModulePorts {
    Ports(AstIdRange<Port>),
    PortDeclarations(AstIdRange<PortDeclaration>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
// port ::=
//   [ port_expression ]
// | . port_identifier ( [ port_expression ] )
#[derive(Clone, Copy)]
pub enum Port {
    PortExpression(AstId<PortExpression>),
    // PortIdentifer(AstId<PortIdentifier>, AstId<PortExpression>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
// port_expression ::=
//   port_reference
// | { port_reference { , port_reference } }
#[derive(Clone, Copy)]
pub struct PortExpression {
    // @Incomplete: { port_reference { , port_reference } }
    pub references: AstId<PortReference>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
// port_reference ::=
//   port_identifier [ [ constant_range_expression ] ]
#[derive(Clone, Copy)]
pub struct PortReference {
    pub identifier: AstItem<Identifier>,
    // @Incomplete
    // range: Option<ConstantRangeExpression>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
// port_declaration ::=
//   {attribute_instance} inout_declaration
// | {attribute_instance} input_declaration
// | {attribute_instance} output_declaration
#[derive(Clone, Copy)]
pub enum PortDeclaration {
    Inout(AstId<InoutDeclaration>),
    Input(AstId<InputDeclaration>),
    Output(AstId<OutputDeclaration>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
// inout_declaration ::= inout [ net_type ] [ signed ] [ range ] list_of_port_identifiers
#[derive(Clone, Copy)]
pub struct InoutDeclaration {
    pub net_type: Option<AstItem<NetType>>,
    pub signed: bool,
    pub range: Option<AstId<Range>>,
    pub port_identifiers: AstIdRange<Identifier>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
// input_declaration ::= input [ net_type ] [ signed ] [ range ] list_of_port_identifiers
#[derive(Clone, Copy)]
pub struct InputDeclaration {
    pub net_type: Option<AstItem<NetType>>,
    pub signed: bool,
    pub range: Option<AstId<Range>>,
    pub port_identifiers: AstIdRange<Identifier>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
// output_declaration ::=
//   output [ net_type ] [ signed ] [ range ] list_of_port_identifiers
// | output reg [ signed ] [ range ] list_of_variable_port_identifiers
// | output output_variable_type list_of_variable_port_identifiers
#[derive(Clone, Copy)]
pub struct OutputDeclaration {
    // @Incomplete: reg | output_variable_type
    pub net: Option<AstItem<NetType>>,
    pub signed: bool,
    pub range: Option<AstId<Range>>,
    pub identifiers: AstIdRange<Identifier>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 492
// range ::= [ msb_constant_expression : lsb_constant_expression ]
#[derive(Clone, Copy)]
pub struct Range {
    pub msb: AstId<ConstantExpr>,
    pub lsb: AstId<ConstantExpr>,
}

#[derive(Clone, Copy)]
pub enum OutputNet {
    NetType(NetType),
    Register,

    // @Incomplete
    Variable,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
// net_type ::=
//   supply0 | supply1
// | tri
// | triand | trior | tri0 | tri1
// | uwire | wire | wand | wor
#[derive(Clone, Copy)]
pub enum NetType {
    Supply0,
    Supply1,
    Tri,
    TriAnd,
    TriOr,
    Tri0,
    Tri1,
    Uwire,
    Wire,
    WAnd,
    WOr,
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
    ModuleOrGenerateItemDeclaration(AstId<ModuleOrGenerateItemDeclaration>),
    LocalParameterDeclaration,
    ParameterOverride,
    ContinuousAssign(AstId<ContinousAssign>),
    GateInstantiation(AstId<GateInstantiation>),
    UdpInstantiation,
    ModuleInstantiation(AstId<ModuleInstantiation>),
    InitialConstruct(AstId<InitialConstruct>),
    AlwaysConstruct(AstId<AlwaysConstruct>),
    LoopGenerateConstruct(AstId<LoopGenerateConstruct>),
    IfGenerateConstruct(AstId<IfGenerateConstruct>),
    CaseGenerateConstruct(AstId<CaseGenerateConstruct>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
// gate_instantiation ::=
//   cmos_switchtype [delay3] cmos_switch_instance { , cmos_switch_instance } ;
// | enable_gatetype [drive_strength] [delay3] enable_gate_instance { , enable_gate_instance } ;
// | mos_switchtype [delay3] mos_switch_instance { , mos_switch_instance } ;
// | n_input_gatetype [drive_strength] [delay2] n_input_gate_instance { , n_input_gate_instance } ;
// | n_output_gatetype [drive_strength] [delay2] n_output_gate_instance { , n_output_gate_instance } ;
// | pass_en_switchtype [delay2] pass_enable_switch_instance { , pass_enable_switch_instance } ;
// | pass_switchtype pass_switch_instance { , pass_switch_instance } ;
// | pulldown [pulldown_strength] pull_gate_instance { , pull_gate_instance } ;
// | pullup [pullup_strength] pull_gate_instance { , pull_gate_instance } ;
#[derive(Clone, Copy)]
pub enum GateInstantiation {
    // @Incomplete
    // Cmos(CmosGateInstantiation),
    // Enable(EnableGateInstantiation),
    // Mos(MosGateInstantiation),
    NInput(AstId<NInputGateInstantiation>),
    // NOutput(NOutputGateInstantiation),
    // PassEn(PassEnGateInstantiation),
    // PassSwitch(PassSwitchGateInstantiation),
    // Pulldown(PulldownGateInstantiation),
    // Pullup(PullupGateInstantiation),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
// n_input_gatetype [drive_strength] [delay2] n_input_gate_instance { , n_input_gate_instance }
#[derive(Clone, Copy)]
pub struct NInputGateInstantiation {
    pub gatetype: AstItem<NInputGateType>,
    pub instances: AstIdRange<NInputGateInstance>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
// n_input_gate_instance ::= [ name_of_gate_instance ] ( output_terminal , input_terminal { , input_terminal } )
#[derive(Clone, Copy)]
pub struct NInputGateInstance {
    pub name: Option<AstId<NameOfGateInstance>>,
    pub output_terminal: AstId<NetLValue>,
    pub input_terminals: AstIdRange<Expr>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
// name_of_gate_instance ::= gate_instance_identifier [ range ]
#[derive(Clone, Copy)]
pub struct NameOfGateInstance {
    pub identifier: AstItem<Identifier>,
    // @Incomplete
    // range:
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
// n_input_gatetype ::= and | nand | or | nor | xor | xnor
#[derive(Clone, Copy)]
pub enum NInputGateType {
    And,
    Nand,
    Or,
    Nor,
    Xor,
    Xnor,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// continuous_assign ::= assign [ drive_strength ] [ delay3 ] list_of_net_assignments ;
// list_of_net_assignments ::= net_assignment { , net_assignment }
#[derive(Clone, Copy)]
pub struct ContinousAssign {
    pub list_of_net_assignments: AstIdRange<NetAssignment>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// net_assignment ::= net_lvalue = expression
#[derive(Clone, Copy)]
pub struct NetAssignment {
    pub net_lvalue: AstId<NetLValue>,
    pub expression: AstId<Expr>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
// module_instantiation ::=
//   module_identifier [ parameter_value_assignment ]
//   module_instance { , module_instance } ;
#[derive(Clone, Copy)]
pub struct ModuleInstantiation {
    pub module_identifier: AstItem<Identifier>,
    pub module_instances: AstIdRange<ModuleInstance>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
// module_instance ::= name_of_module_instance ( [ list_of_port_connections ] )
#[derive(Clone, Copy)]
pub struct ModuleInstance {
    pub name_of_module_instance: AstItem<Identifier>,
    pub list_of_port_connections: AstId<ListOfPortConnections>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
// list_of_port_connections ::=
//   ordered_port_connection { , ordered_port_connection }
// | named_port_connection { , named_port_connection }
#[derive(Clone, Copy)]
pub enum ListOfPortConnections {
    Ordered(AstIdRange<Expr>),
    Named(AstIdRange<NamedPortConnection>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
// named_port_connection ::= { attribute_instance } . port_identifier ( [ expression ] )
#[derive(Clone, Copy)]
pub struct NamedPortConnection {
    pub port_identifier: AstItem<Identifier>,
    pub expression: Option<AstId<Expr>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// initial_construct ::= initial statement
#[derive(Clone, Copy)]
pub struct InitialConstruct(pub AstId<Statement>);

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// always_construct ::= always statement
#[derive(Clone, Copy)]
pub struct AlwaysConstruct(pub AstId<Statement>);

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
// loop_generate_construct ::= for ( genvar_initialization ; genvar_expression ; genvar_iteration ) generate_block
#[derive(Clone, Copy)]
pub struct LoopGenerateConstruct {
    pub initialization: AstId<GenvarAssignment>,
    pub condition: AstId<ConstantExpr>,
    pub iteration: AstId<GenvarAssignment>,
    pub block: AstId<GenerateBlock>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
// if_generate_construct ::= if ( constant_expression ) generate_block_or_null
//   [ else generate_block_or_null ]
#[derive(Clone, Copy)]
pub struct IfGenerateConstruct {
    pub condition: AstId<ConstantExpr>,
    pub truthy: AstId<Option<GenerateBlock>>,
    pub falsy: Option<AstId<Option<GenerateBlock>>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
// case_generate_construct ::= case ( constant_expression ) case_generate_item { case_generate_item } endcase
#[derive(Clone, Copy)]
pub struct CaseGenerateConstruct {
    pub value: AstId<ConstantExpr>,
    pub items: AstIdRange<CaseGenerateItem>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
// case_generate_item ::= constant_expression { , constant_expression } : generate_block_or_null | default [ : ] generate_block_or_null
#[derive(Clone, Copy)]
pub struct CaseGenerateItem {
    pub pattern: CaseGeneratePattern,
    pub block: AstId<Option<GenerateBlock>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
// case_generate_item ::= constant_expression { , constant_expression } : generate_block_or_null | default [ : ] generate_block_or_null
#[derive(Clone, Copy)]
pub enum CaseGeneratePattern {
    Default,
    Exprs(AstIdRange<ConstantExpr>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
// generate_block ::= module_or_generate_item | begin [ : generate_block_identifier ] { module_or_generate_item } end
#[derive(Clone, Copy)]
pub enum GenerateBlock {
    ModuleOrGenerateItem(AstId<ModuleOrGenerateItem>),
    BeginEnd(
        Option<AstItem<Identifier>>,
        AstIdRange<ModuleOrGenerateItem>,
    ),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// genvar_initialization ::= genvar_identifier = constant_expression
// genvar_iteration      ::= genvar_identifier = genvar_expression
#[derive(Clone, Copy)]
pub struct GenvarAssignment {
    pub ident: AstItem<Identifier>,
    pub expr: AstId<ConstantExpr>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
// module_item ::=
//   port_declaration ;
// | non_port_module_item
#[derive(Clone, Copy)]
pub enum ModuleItem {
    PortDeclaration(AstId<PortDeclaration>),
    NonPortModuleItem(AstId<NonPortModuleItem>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
// module_or_generate_item_declaration ::=
//   net_declaration
// | reg_declaration
// | integer_declaration
// | real_declaration
// | time_declaration
// | realtime_declaration
// | event_declaration
// | genvar_declaration
// | task_declaration
// | function_declaration
#[derive(Clone, Copy)]
pub enum ModuleOrGenerateItemDeclaration {
    // @Incomplete
    Net(AstId<NetDeclaration>),
    Reg(AstId<RegDeclaration>),
    Integer(AstId<IntegerDeclaration>),
    // Real(AstId<RealDeclaration>),
    // Time(AstId<TimeDeclaration>),
    // Realtime(AstId<RealtimeDeclaration>),
    // Event(AstId<EventDeclaration>),
    Genvar(AstId<GenvarDeclaration>),
    // Task(AstId<TaskDeclaration>),
    // Function(AstId<FunctionDeclaration>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
// net_declaration ::=
//   net_type [ signed ] [ delay3 ] list_of_net_identifiers ;
// | net_type [ drive_strength ] [ signed ] [ delay3 ] list_of_net_decl_assignments ;
// | net_type [ vectored | scalared ] [ signed ] range [ delay3 ] list_of_net_identifiers ;
// | net_type [ drive_strength ] [ vectored | scalared ] [ signed ] range [ delay3 ] list_of_net_decl_assignments ;
// | trireg [ charge_strength ] [ signed ] [ delay3 ] list_of_net_identifiers ;
// | trireg [ drive_strength ] [ signed ] [ delay3 ] list_of_net_decl_assignments ;
// | trireg [ charge_strength ] [ vectored | scalared ] [ signed ] range [ delay3 ] list_of_net_identifiers ;
// | trireg [ drive_strength ] [ vectored | scalared ] [ signed ] range [ delay3 ] list_of_net_decl_assignments ;
#[derive(Clone, Copy)]
pub struct NetDeclaration {
    // @Incomplete
    pub net_type: AstItem<NetType>,
    pub signed: bool,
    pub range: Option<AstId<Range>>,
    pub nets: NetDeclarationNets,
}

#[derive(Clone, Copy)]
pub enum NetDeclarationNets {
    Idents(AstIdRange<NetIdent>),
    Assignments(AstIdRange<NetDeclAssignment>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 491
// net_decl_assignment ::= net_identifier = expression
#[derive(Clone, Copy)]
pub struct NetDeclAssignment {
    pub ident: AstItem<Identifier>,
    pub expr: AstId<Expr>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 491
// net_identifier { dimension }
#[derive(Clone, Copy)]
pub struct NetIdent {
    pub ident: AstItem<Identifier>,
    pub dimension: AstIdRange<Dimension>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
// reg_declaration ::= reg [ signed ] [ range ] list_of_variable_identifiers ;
#[derive(Clone, Copy)]
pub struct RegDeclaration {
    pub signed: bool,
    pub range: Option<AstId<Range>>,
    pub identifiers: AstIdRange<Identifier>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
// integer_declaration ::= integer list_of_variable_identifiers ;
#[derive(Clone, Copy)]
pub struct IntegerDeclaration {
    pub identifiers: AstIdRange<Identifier>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
// genvar_declaration ::= genvar list_of_genvar_identifiers ;
#[derive(Clone, Copy)]
pub struct GenvarDeclaration {
    pub identifiers: AstIdRange<Identifier>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
// non_port_module_item ::=
//   module_or_generate_item
// | generate_region
// | specify_block
// | { attribute_instance } parameter_declaration ;
// | { attribute_instance } specparam_declaration
#[derive(Clone, Copy)]
pub enum NonPortModuleItem {
    ModuleOrGenerateItem(AstId<ModuleOrGenerateItem>),
    GenerateRegion(GenerateRegion),
    SpecifyBlock,
    ParameterDeclaration(AstId<ParameterDeclaration>),
    SpecParamDeclaration,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
// generate_region ::= generate { module_or_generate_item } endgenerate
#[derive(Clone, Copy)]
pub struct GenerateRegion {
    pub module_or_generate_item: AstIdRange<ModuleOrGenerateItem>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
// parameter_declaration ::=
//   parameter [ signed ] [ range ] list_of_param_assignments
// | parameter parameter_type list_of_param_assignments
#[derive(Clone, Copy)]
pub struct ParameterDeclaration {
    // @Incomplete
    // typing: AstItem<ParameterDeclarationTyping>,
    pub assignments: AstIdRange<ParamAssignment>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 491
// param_assignment ::= parameter_identifier = constant_mintypmax_expression
#[derive(Clone, Copy)]
pub struct ParamAssignment {
    pub param: AstItem<Identifier>,
    pub constant: AstId<ConstantMinTypMaxExpression>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 492
// dimension ::= [ dimension_constant_expression : dimension_constant_expression ]
#[derive(Clone, Copy)]
pub struct Dimension {
    pub lhs: AstId<ConstantExpr>,
    pub rhs: AstId<ConstantExpr>,
}

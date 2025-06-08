use super::expr::Expr;
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
    // @Incomplete: These are in fact constant expressions.
    msb: AstId<Expr>,
    lsb: AstId<Expr>,
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
    ModuleOrGenerateItemDeclaration,
    LocalParameterDeclaration,
    ParameterOverride,
    ContinuousAssign,
    GateInstantiation,
    UdpInstantiation,
    ModuleInstantiation(AstId<ModuleInstantiation>),
    InitialConstruct(AstId<InitialConstruct>),
    AlwaysConstruct(AstId<AlwaysConstruct>),
    LoopGenerateConstruct,
    ConditionalGenerateConstruct,
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
    Ordered(u64),
    Named,
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
// module_item ::=
//   port_declaration ;
// | non_port_module_item
#[derive(Clone, Copy)]
pub enum ModuleItem {
    PortDeclaration(AstId<PortDeclaration>),
    NonPortModuleItem(AstId<NonPortModuleItem>),
}

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


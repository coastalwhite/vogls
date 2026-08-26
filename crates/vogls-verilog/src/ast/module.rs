use crate::parser::DefaultNettype;
use vogls_ir::time::TimeResolution;

use super::constant_expr::{ConstantExpr, ConstantMinTypMaxExpression, ConstantRangeExpression};
use super::expr::Expr;
use super::specify::SpecifyBlock;
use super::statement::{Delay2, Delay3, NetLValue, Statement, StatementOrNull};
use super::udp::{UdpDeclaration, UdpInstantiation};
use super::{AstId, AstIdRange, AstItem, AttributeInstance, DriveStrength, Identifier};

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 487
// description ::=
//   module_declaration
// | udp_declaration
// | config_declaration
#[derive(Clone, Copy)]
pub enum Description<'a> {
    Module(AstId<'a, Module<'a>>),
    Udp(AstId<'a, UdpDeclaration<'a>>),
    // @Incomplete
    Config,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 487
// module_declaration ::=
//   { attribute_instance } module_keyword module_identifier [ module_parameter_port_list ] list_of_ports ; { module_item }
//   endmodule
// | { attribute_instance } module_keyword module_identifier [ module_parameter_port_list ] [ list_of_port_declarations ] ; { non_port_module_item }
//   endmodule
#[derive(Clone, Copy)]
pub struct Module<'a> {
    pub attribute_instances: AstIdRange<'a, AttributeInstance<'a>>,
    pub module_identifier: AstItem<Identifier>,
    pub module_parameter_port_list: Option<AstIdRange<'a, ParameterDeclaration<'a>>>,
    pub ports: ModulePorts<'a>,
    pub module_items: AstIdRange<'a, ModuleItem<'a>>,
    pub default_nettype: Option<DefaultNettype>,
    pub time_scale: TimeScale,
}

#[derive(Clone, Copy)]
pub struct TimeScale {
    pub unit: TimeResolution,
    pub precision: TimeResolution,
}

impl TimeScale {
    pub const fn new() -> Self {
        Self {
            unit: TimeResolution::S1,
            precision: TimeResolution::S1,
        }
    }
}

impl Default for TimeScale {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy)]
pub enum ModulePorts<'a> {
    Ports(AstIdRange<'a, Port<'a>>),
    PortDeclarations(AstIdRange<'a, PortDeclaration<'a>>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
// port ::=
//   [ port_expression ]
// | . port_identifier ( [ port_expression ] )
#[derive(Clone, Copy)]
pub enum Port<'a> {
    PortExpression(AstId<'a, PortExpression<'a>>),
    // PortIdentifer(AstId<PortIdentifier>, AstId<PortExpression>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
// port_expression ::=
//   port_reference
// | { port_reference { , port_reference } }
#[derive(Clone, Copy)]
pub struct PortExpression<'a> {
    // @Incomplete: { port_reference { , port_reference } }
    pub references: AstId<'a, PortReference<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
// port_reference ::=
//   port_identifier [ [ constant_range_expression ] ]
#[derive(Clone, Copy)]
pub struct PortReference<'a> {
    pub identifier: AstItem<Identifier>,
    pub range: Option<AstId<'a, ConstantRangeExpression<'a>>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
// port_declaration ::=
//   {attribute_instance} inout_declaration
// | {attribute_instance} input_declaration
// | {attribute_instance} output_declaration
#[derive(Clone, Copy)]
pub enum PortDeclaration<'a> {
    Inout(AstId<'a, InoutDeclaration<'a>>),
    Input(AstId<'a, InputDeclaration<'a>>),
    Output(AstId<'a, OutputDeclaration<'a>>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
// inout_declaration ::= inout [ net_type ] [ signed ] [ range ] list_of_port_identifiers
#[derive(Clone, Copy)]
pub struct InoutDeclaration<'a> {
    pub net_type: Option<AstItem<NetType>>,
    pub signed: bool,
    pub range: Option<AstId<'a, Range<'a>>>,
    pub port_identifiers: AstIdRange<'a, Identifier>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
// input_declaration ::= input [ net_type ] [ signed ] [ range ] list_of_port_identifiers
#[derive(Clone, Copy)]
pub struct InputDeclaration<'a> {
    pub net_type: Option<AstItem<NetType>>,
    pub signed: bool,
    pub range: Option<AstId<'a, Range<'a>>>,
    pub port_identifiers: AstIdRange<'a, Identifier>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
// output_declaration ::=
//   output [ net_type ] [ signed ] [ range ] list_of_port_identifiers
// | output reg [ signed ] [ range ] list_of_variable_port_identifiers
// | output output_variable_type list_of_variable_port_identifiers
#[derive(Clone, Copy)]
pub struct OutputDeclaration<'a> {
    pub net: Option<AstItem<OutputNet>>,
    pub signed: bool,
    pub range: Option<AstId<'a, Range<'a>>>,
    pub identifiers: AstIdRange<'a, Identifier>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 492
// range ::= [ msb_constant_expression : lsb_constant_expression ]
#[derive(Clone, Copy)]
pub struct Range<'a> {
    pub msb: AstId<'a, ConstantExpr<'a>>,
    pub lsb: AstId<'a, ConstantExpr<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489-490
// output_declaration ::=
//   output [ net_type ] [ signed ] [ range ] list_of_port_identifiers
// | output reg [ signed ] [ range ] list_of_variable_port_identifiers
// | output output_variable_type list_of_variable_port_identifiers
// output_variable_type ::= integer | time
#[derive(Clone, Copy)]
pub enum OutputNet {
    NetType(NetType),
    Register,
    Integer,
    Time,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
// net_type ::=
//   supply0 | supply1
// | tri
// | triand | trior | tri0 | tri1
// | uwire | wire | wand | wor
#[derive(Debug, Clone, Copy)]
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
pub struct ModuleOrGenerateItem<'a> {
    pub attribute_instances: AstIdRange<'a, AttributeInstance<'a>>,
    pub content: ModuleOrGenerateItemContent<'a>,
}
#[derive(Clone, Copy)]
pub enum ModuleOrGenerateItemContent<'a> {
    ModuleOrGenerateItemDeclaration(AstId<'a, ModuleOrGenerateItemDeclaration<'a>>),
    LocalParameterDeclaration(AstId<'a, LocalParameterDeclaration<'a>>),
    ParameterOverride,
    ContinuousAssign(AstId<'a, ContinousAssign<'a>>),
    GateInstantiation(AstId<'a, GateInstantiation<'a>>),
    UdpInstantiation(AstId<'a, UdpInstantiation<'a>>),
    ModuleInstantiation(AstId<'a, ModuleInstantiation<'a>>),
    InitialConstruct(AstId<'a, InitialConstruct<'a>>),
    AlwaysConstruct(AstId<'a, AlwaysConstruct<'a>>),
    LoopGenerateConstruct(AstId<'a, LoopGenerateConstruct<'a>>),
    IfGenerateConstruct(AstId<'a, IfGenerateConstruct<'a>>),
    CaseGenerateConstruct(AstId<'a, CaseGenerateConstruct<'a>>),
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
pub enum GateInstantiation<'a> {
    Cmos(AstId<'a, CmosSwitchInstantiation<'a>>),
    Enable(AstId<'a, EnableGateInstantiation<'a>>),
    Mos(AstId<'a, MosSwitchInstantiation<'a>>),
    NInput(AstId<'a, NInputGateInstantiation<'a>>),
    NOutput(AstId<'a, NOutputGateInstantiation<'a>>),
    PassEn(AstId<'a, PassEnSwitchInstantiation<'a>>),
    PassSwitch(AstId<'a, PassSwitchInstantiation<'a>>),
    Pulldown(AstId<'a, PullGateInstantiation<'a>>),
    Pullup(AstId<'a, PullGateInstantiation<'a>>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
// cmos_switchtype [delay3] cmos_switch_instance { , cmos_switch_instance } ;
#[derive(Clone, Copy)]
pub struct CmosSwitchInstantiation<'a> {
    pub gatetype: AstItem<CmosSwitchType>,
    pub delay: Option<AstId<'a, Delay3<'a>>>,
    pub instances: AstIdRange<'a, CmosSwitchInstance<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
// cmos_switch_instance ::= [ name_of_gate_instance ] ( output_terminal , input_terminal , ncontrol_terminal , pcontrol_terminal )
#[derive(Clone, Copy)]
pub struct CmosSwitchInstance<'a> {
    pub name: Option<AstId<'a, NameOfGateInstance<'a>>>,
    pub output_terminal: AstId<'a, NetLValue<'a>>,
    pub input_terminal: AstId<'a, Expr<'a>>,
    pub ncontrol_terminal: AstId<'a, Expr<'a>>,
    pub pcontrol_terminal: AstId<'a, Expr<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
// enable_gatetype [drive_strength] [delay3] enable_gate_instance { , enable_gate_instance } ;
#[derive(Clone, Copy)]
pub struct EnableGateInstantiation<'a> {
    pub gatetype: AstItem<EnableGateType>,
    pub drive_strength: Option<AstItem<DriveStrength>>,
    pub delay: Option<AstId<'a, Delay3<'a>>>,
    pub instances: AstIdRange<'a, EnableGateInstance<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
// enable_gate_instance ::= [ name_of_gate_instance ] ( output_terminal , input_terminal , enable_terminal )
#[derive(Clone, Copy)]
pub struct EnableGateInstance<'a> {
    pub name: Option<AstId<'a, NameOfGateInstance<'a>>>,
    pub output_terminal: AstId<'a, NetLValue<'a>>,
    pub input_terminal: AstId<'a, Expr<'a>>,
    pub enable_terminal: AstId<'a, Expr<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
// mos_switchtype [delay3] mos_switch_instance { , mos_switch_instance } ;
#[derive(Clone, Copy)]
pub struct MosSwitchInstantiation<'a> {
    pub gatetype: AstItem<MosSwitchType>,
    pub delay: Option<AstId<'a, Delay3<'a>>>,
    pub instances: AstIdRange<'a, EnableGateInstance<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
// mos_switch_instance ::= [ name_of_gate_instance ] ( output_terminal , input_terminal , enable_terminal )
#[derive(Clone, Copy)]
pub struct MosSwitchInstance<'a> {
    pub name: Option<AstId<'a, NameOfGateInstance<'a>>>,
    pub output_terminal: AstId<'a, NetLValue<'a>>,
    pub input_terminal: AstId<'a, Expr<'a>>,
    pub enable_terminal: AstId<'a, Expr<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
// pass_en_switchtype [delay2] pass_enable_switch_instance { , pass_enable_switch_instance } ;
#[derive(Clone, Copy)]
pub struct PassEnSwitchInstantiation<'a> {
    pub gatetype: AstItem<PassEnSwitchType>,
    pub delay: Option<AstId<'a, Delay2<'a>>>,
    pub instances: AstIdRange<'a, PassEnSwitchInstance<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
// pass_switchtype pass_switch_instance { , pass_switch_instance } ;
#[derive(Clone, Copy)]
pub struct PassSwitchInstantiation<'a> {
    pub gatetype: AstItem<PassSwitchType>,
    pub instances: AstIdRange<'a, PassSwitchInstance<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
// pass_enable_switch_instance ::= [ name_of_gate_instance ] ( inout_terminal , inout_terminal , enable_terminal )
#[derive(Clone, Copy)]
pub struct PassEnSwitchInstance<'a> {
    pub name: Option<AstId<'a, NameOfGateInstance<'a>>>,
    pub fst: AstId<'a, Expr<'a>>,
    pub snd: AstId<'a, Expr<'a>>,
    pub enable_terminal: AstId<'a, Expr<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
// pass_switch_instance ::= [ name_of_gate_instance ] ( inout_terminal , inout_terminal )
#[derive(Clone, Copy)]
pub struct PassSwitchInstance<'a> {
    pub name: Option<AstId<'a, NameOfGateInstance<'a>>>,
    pub fst: AstId<'a, Expr<'a>>,
    pub snd: AstId<'a, Expr<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
//   pullup [pullup_strength] pull_gate_instance { , pull_gate_instance } ;
// | pulldown [pullup_strength] pull_gate_instance { , pull_gate_instance } ;
#[derive(Clone, Copy)]
pub struct PullGateInstantiation<'a> {
    pub pullup_strength: Option<AstItem<DriveStrength>>,
    pub instances: AstIdRange<'a, PullGateInstance<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
// pull_gate_instance ::= [ name_of_gate_instance ] ( output_terminal )
#[derive(Clone, Copy)]
pub struct PullGateInstance<'a> {
    pub name: Option<AstId<'a, NameOfGateInstance<'a>>>,
    pub output_terminal: AstId<'a, NetLValue<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
// n_input_gatetype [drive_strength] [delay2] n_input_gate_instance { , n_input_gate_instance }
#[derive(Clone, Copy)]
pub struct NInputGateInstantiation<'a> {
    pub gatetype: AstItem<NInputGateType>,
    pub instances: AstIdRange<'a, NInputGateInstance<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
// n_input_gate_instance ::= [ name_of_gate_instance ] ( output_terminal , input_terminal { , input_terminal } )
#[derive(Clone, Copy)]
pub struct NInputGateInstance<'a> {
    pub name: Option<AstId<'a, NameOfGateInstance<'a>>>,
    pub output_terminal: AstId<'a, NetLValue<'a>>,
    pub input_terminals: AstIdRange<'a, Expr<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
// name_of_gate_instance ::= gate_instance_identifier [ range ]
#[derive(Clone, Copy)]
pub struct NameOfGateInstance<'a> {
    pub identifier: AstItem<Identifier>,
    pub range: Option<AstId<'a, Range<'a>>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
// cmos_switchtype ::= cmos | rcmos
#[derive(Clone, Copy)]
pub enum CmosSwitchType {
    Cmos,
    Rcmos,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
// enable_gatetype ::= bufif0 | bufif1 | notif0 | notif1
#[derive(Clone, Copy)]
pub enum EnableGateType {
    BufIf0,
    BufIf1,
    NotIf0,
    NotIf1,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
// mos_switchtype ::= nmos | pmos | rnmos | rpmos
#[derive(Clone, Copy)]
pub enum MosSwitchType {
    NMos,
    PMos,
    RNMos,
    RPMos,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
// pass_en_switchtype ::= tranif0 | tranif1 | rtranif1 | rtranif0
#[derive(Clone, Copy)]
pub enum PassEnSwitchType {
    Tranif0,
    Tranif1,
    Rtranif1,
    Rtranif0,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
// pass_switchtype ::= tran | rtran
#[derive(Clone, Copy)]
pub enum PassSwitchType {
    Tran,
    RTran,
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

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
// n_output_gatetype [drive_strength] [delay2] n_output_gate_instance { , n_output_gate_instance }
#[derive(Clone, Copy)]
pub struct NOutputGateInstantiation<'a> {
    pub gatetype: AstItem<NOutputGateType>,
    pub instances: AstIdRange<'a, NOutputGateInstance<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
// n_output_gate_instance ::= [ name_of_gate_instance ] ( output_terminal { , output_terminal } , input_terminal )
#[derive(Clone, Copy)]
pub struct NOutputGateInstance<'a> {
    pub name: Option<AstId<'a, NameOfGateInstance<'a>>>,
    pub output_terminals: AstIdRange<'a, NetLValue<'a>>,
    pub input_terminal: AstId<'a, Expr<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 494
// n_output_gatetype ::= buf | not
#[derive(Clone, Copy)]
pub enum NOutputGateType {
    Buf,
    Not,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// continuous_assign ::= assign [ drive_strength ] [ delay3 ] list_of_net_assignments ;
// list_of_net_assignments ::= net_assignment { , net_assignment }
#[derive(Clone, Copy)]
pub struct ContinousAssign<'a> {
    pub list_of_net_assignments: AstIdRange<'a, NetAssignment<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// net_assignment ::= net_lvalue = expression
#[derive(Clone, Copy)]
pub struct NetAssignment<'a> {
    pub net_lvalue: AstId<'a, NetLValue<'a>>,
    pub expression: AstId<'a, Expr<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
// module_instantiation ::=
//   module_identifier [ parameter_value_assignment ]
//   module_instance { , module_instance } ;
#[derive(Clone, Copy)]
pub struct ModuleInstantiation<'a> {
    pub module_identifier: AstItem<Identifier>,
    pub parameter_value_assignment: Option<AstId<'a, ParameterValueAssignment<'a>>>,
    pub module_instances: AstIdRange<'a, ModuleInstance<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
// parameter_value_assignment ::= # ( list_of_parameter_assignments )
// list_of_parameter_assignments ::=
//   ordered_parameter_assignment { , ordered_parameter_assignment }
// | named_parameter_assignment { , named_parameter_assignment }
#[derive(Clone, Copy)]
pub enum ParameterValueAssignment<'a> {
    Ordered(AstIdRange<'a, ConstantExpr<'a>>),
    Named(AstIdRange<'a, NamedParameterAssignment<'a>>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
// named_parameter_assignment ::= . parameter_identifier ( [ mintypmax_expression ] )
#[derive(Clone, Copy)]
pub struct NamedParameterAssignment<'a> {
    pub identifier: AstItem<Identifier>,
    pub expression: Option<AstId<'a, ConstantMinTypMaxExpression<'a>>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
// module_instance ::= name_of_module_instance ( [ list_of_port_connections ] )
#[derive(Clone, Copy)]
pub struct ModuleInstance<'a> {
    pub name_of_module_instance: AstItem<Identifier>,
    pub range: Option<AstId<'a, Range<'a>>>,
    pub list_of_port_connections: AstId<'a, ListOfPortConnections<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
// list_of_port_connections ::=
//   ordered_port_connection { , ordered_port_connection }
// | named_port_connection { , named_port_connection }
#[derive(Clone, Copy)]
pub enum ListOfPortConnections<'a> {
    Ordered(AstIdRange<'a, Expr<'a>>),
    Named(AstIdRange<'a, NamedPortConnection<'a>>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
// named_port_connection ::= { attribute_instance } . port_identifier ( [ expression ] )
#[derive(Clone, Copy)]
pub struct NamedPortConnection<'a> {
    pub port_identifier: AstItem<Identifier>,
    pub expression: Option<AstId<'a, Expr<'a>>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// initial_construct ::= initial statement
#[derive(Clone, Copy)]
pub struct InitialConstruct<'a>(pub AstId<'a, Statement<'a>>);

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// always_construct ::= always statement
#[derive(Clone, Copy)]
pub struct AlwaysConstruct<'a>(pub AstId<'a, Statement<'a>>);

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
// loop_generate_construct ::= for ( genvar_initialization ; genvar_expression ; genvar_iteration ) generate_block
#[derive(Clone, Copy)]
pub struct LoopGenerateConstruct<'a> {
    pub initialization: AstId<'a, GenvarAssignment<'a>>,
    pub condition: AstId<'a, ConstantExpr<'a>>,
    pub iteration: AstId<'a, GenvarAssignment<'a>>,
    pub block: AstId<'a, GenerateBlock<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
// if_generate_construct ::= if ( constant_expression ) generate_block_or_null
//   [ else generate_block_or_null ]
#[derive(Clone, Copy)]
pub struct IfGenerateConstruct<'a> {
    pub condition: AstId<'a, ConstantExpr<'a>>,
    pub truthy: AstId<'a, Option<GenerateBlock<'a>>>,
    pub falsy: Option<AstId<'a, Option<GenerateBlock<'a>>>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
// case_generate_construct ::= case ( constant_expression ) case_generate_item { case_generate_item } endcase
#[derive(Clone, Copy)]
pub struct CaseGenerateConstruct<'a> {
    pub value: AstId<'a, ConstantExpr<'a>>,
    pub items: AstIdRange<'a, CaseGenerateItem<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
// case_generate_item ::= constant_expression { , constant_expression } : generate_block_or_null | default [ : ] generate_block_or_null
#[derive(Clone, Copy)]
pub struct CaseGenerateItem<'a> {
    pub pattern: CaseGeneratePattern<'a>,
    pub block: AstId<'a, Option<GenerateBlock<'a>>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
// case_generate_item ::= constant_expression { , constant_expression } : generate_block_or_null | default [ : ] generate_block_or_null
#[derive(Clone, Copy)]
pub enum CaseGeneratePattern<'a> {
    Default,
    Exprs(AstIdRange<'a, ConstantExpr<'a>>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
// generate_block ::= module_or_generate_item | begin [ : generate_block_identifier ] { module_or_generate_item } end
#[derive(Clone, Copy)]
pub enum GenerateBlock<'a> {
    ModuleOrGenerateItem(AstId<'a, ModuleOrGenerateItem<'a>>),
    BeginEnd(
        Option<AstItem<Identifier>>,
        AstIdRange<'a, ModuleOrGenerateItem<'a>>,
    ),
}

impl<'a> GenerateBlock<'a> {
    pub fn ident(self) -> Option<AstItem<Identifier>> {
        match self {
            GenerateBlock::ModuleOrGenerateItem(_) => None,
            GenerateBlock::BeginEnd(ident, _) => ident,
        }
    }
    pub fn module_or_generate_items(self) -> AstIdRange<'a, ModuleOrGenerateItem<'a>> {
        match self {
            GenerateBlock::ModuleOrGenerateItem(id) => AstIdRange::single(id),
            GenerateBlock::BeginEnd(_, ids) => ids,
        }
    }
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// genvar_initialization ::= genvar_identifier = constant_expression
// genvar_iteration      ::= genvar_identifier = genvar_expression
#[derive(Clone, Copy)]
pub struct GenvarAssignment<'a> {
    pub ident: AstItem<Identifier>,
    pub expr: AstId<'a, ConstantExpr<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
// module_item ::=
//   port_declaration ;
// | non_port_module_item
#[derive(Clone, Copy)]
pub enum ModuleItem<'a> {
    PortDeclaration(AstId<'a, PortDeclaration<'a>>),
    NonPortModuleItem(AstId<'a, NonPortModuleItem<'a>>),
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
pub enum ModuleOrGenerateItemDeclaration<'a> {
    // @Incomplete
    Net(AstId<'a, NetDeclaration<'a>>),
    Reg(AstId<'a, RegDeclaration<'a>>),
    Integer(AstId<'a, IntegerDeclaration<'a>>),
    Real(AstId<'a, RealDeclaration<'a>>),
    Time(AstId<'a, TimeDeclaration<'a>>),
    Realtime(AstId<'a, RealtimeDeclaration<'a>>),
    // Event(AstId<EventDeclaration>),
    Genvar(AstId<'a, GenvarDeclaration<'a>>),
    Task(AstId<'a, TaskDeclaration<'a>>),
    Function(AstId<'a, FunctionDeclaration<'a>>),
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
pub struct NetDeclaration<'a> {
    // @Incomplete
    pub net_type: AstItem<NetType>,
    pub signed: bool,
    pub range: Option<AstId<'a, Range<'a>>>,
    pub nets: NetDeclarationNets<'a>,
}

#[derive(Clone, Copy)]
pub enum NetDeclarationNets<'a> {
    Idents(AstIdRange<'a, NetIdent<'a>>),
    Assignments(AstIdRange<'a, NetDeclAssignment<'a>>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 491
// net_decl_assignment ::= net_identifier = expression
#[derive(Clone, Copy)]
pub struct NetDeclAssignment<'a> {
    pub ident: AstItem<Identifier>,
    pub expr: AstId<'a, Expr<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 491
// net_identifier { dimension }
#[derive(Clone, Copy)]
pub struct NetIdent<'a> {
    pub ident: AstItem<Identifier>,
    pub dimension: AstIdRange<'a, Dimension<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
// reg_declaration ::= reg [ signed ] [ range ] list_of_variable_identifiers ;
#[derive(Clone, Copy)]
pub struct RegDeclaration<'a> {
    pub signed: bool,
    pub range: Option<AstId<'a, Range<'a>>>,
    pub variable_types: AstIdRange<'a, VariableType<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
// variable_type ::=
//   variable_identifier { dimension } |
//   variable_identifier = constant_expression
#[derive(Clone, Copy)]
pub struct VariableType<'a> {
    pub identifier: AstItem<Identifier>,
    pub variant: VariableTypeVariant<'a>,
}
#[derive(Clone, Copy)]
pub enum VariableTypeVariant<'a> {
    Dimensions(AstIdRange<'a, Dimension<'a>>),
    ConstantExpr(AstId<'a, ConstantExpr<'a>>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
// integer_declaration ::= integer list_of_variable_identifiers ;
#[derive(Clone, Copy)]
pub struct IntegerDeclaration<'a> {
    pub variable_types: AstIdRange<'a, VariableType<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
// real_declaration ::= real list_of_real_identifiers ;
#[derive(Clone, Copy)]
pub struct RealDeclaration<'a> {
    pub variable_types: AstIdRange<'a, VariableType<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
// time_declaration ::= time list_of_variable_identifiers ;
#[derive(Clone, Copy)]
pub struct TimeDeclaration<'a> {
    pub variable_types: AstIdRange<'a, VariableType<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 490
// realtime_declaration ::= realtime list_of_real_identifiers ;
#[derive(Clone, Copy)]
pub struct RealtimeDeclaration<'a> {
    pub variable_types: AstIdRange<'a, VariableType<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
// genvar_declaration ::= genvar list_of_genvar_identifiers ;
#[derive(Clone, Copy)]
pub struct GenvarDeclaration<'a> {
    pub identifiers: AstIdRange<'a, Identifier>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 492
// task_declaration ::= task [ automatic ] task_identifier ;
//   { task_item_declaration }
//   statement_or_null
//   endtask
// | task [ automatic ] task_identifier ( [ task_port_list ] ) ;
//   { block_item_declaration }
//   statement_or_null
//   endtask
#[derive(Clone, Copy)]
pub struct TaskDeclaration<'a> {
    pub ident: AstItem<Identifier>,
    pub automatic: bool,
    pub task_ports: AstIdRange<'a, TaskPortItem<'a>>,
    pub block_item_decls: AstIdRange<'a, BlockItemDeclaration<'a>>,
    pub statement_or_null: AstId<'a, StatementOrNull<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 492
// function_declaration ::=
//   function [ automatic ] [ function_range_or_type ] function_identifier ;
//     function_item_declaration { function_item_declaration }
//     function_statement
//   endfunction
// | function [ automatic ] [ function_range_or_type ] function_identifier ( function_port_list ) ;
//     { block_item_declaration }
//     function_statement
//   endfunction
#[derive(Clone, Copy)]
pub struct FunctionDeclaration<'a> {
    pub automatic: bool,
    pub range_or_type: AstId<'a, FunctionRangeOrType<'a>>,
    pub ident: AstItem<Identifier>,
    pub tf_input_decls: AstIdRange<'a, TfInputDeclaration<'a>>,
    pub block_item_decls: AstIdRange<'a, BlockItemDeclaration<'a>>,
    pub statement: AstId<'a, Statement<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 492
// function_range_or_type ::= [ signed ] [ range ] | integer | real | realtime | time
#[derive(Clone, Copy)]
pub enum FunctionRangeOrType<'a> {
    Signed(Option<AstId<'a, Range<'a>>>),
    Unsigned(Option<AstId<'a, Range<'a>>>),
    Integer,
    Real,
    Realtime,
    Time,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
// tf_input_declaration ::=
//   input [ reg ] [ signed ] [ range ] list_of_port_identifiers
// | input task_port_type list_of_port_identifiers
#[derive(Clone, Copy)]
pub struct TfInputDeclaration<'a> {
    pub tf_type: TfType<'a>,
    pub port_identifiers: AstIdRange<'a, Identifier>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
// tf_output_declaration ::=
//   output [ reg ] [ signed ] [ range ] list_of_port_identifiers
// | output task_port_type list_of_port_identifiers
#[derive(Clone, Copy)]
pub struct TfOutputDeclaration<'a> {
    pub tf_type: TfType<'a>,
    pub port_identifiers: AstIdRange<'a, Identifier>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
// tf_inout_declaration ::=
//   inout [ reg ] [ signed ] [ range ] list_of_port_identifiers
// | inout task_port_type list_of_port_identifiers
#[derive(Clone, Copy)]
pub struct TfInoutDeclaration<'a> {
    pub tf_type: TfType<'a>,
    pub port_identifiers: AstIdRange<'a, Identifier>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 492
// task_port_item ::=
//   { attribute_instance } tf_input_declaration
// | { attribute_instance } tf_output_declaration
// | { attribute_instance } tf_inout_declaration
#[derive(Clone, Copy)]
pub struct TaskPortItem<'a> {
    pub attribute_instances: AstIdRange<'a, AttributeInstance<'a>>,
    pub content: TaskPortItemContent<'a>,
}
#[derive(Clone, Copy)]
pub enum TaskPortItemContent<'a> {
    Input(TfInputDeclaration<'a>),
    Output(TfOutputDeclaration<'a>),
    Inout(TfInoutDeclaration<'a>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
// task_port_type ::= integer | real | realtime | time
#[derive(Clone, Copy)]
pub enum TfType<'a> {
    Net {
        reg: bool,
        signed: bool,
        range: Option<AstId<'a, Range<'a>>>,
    },
    Integer,
    Real,
    Realtime,
    Time,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 493
// block_item_declaration ::=
//   { attribute_instance } reg [ signed ] [ range ] list_of_block_variable_identifiers ;
// | { attribute_instance } integer list_of_block_variable_identifiers ;
// | { attribute_instance } time list_of_block_variable_identifiers ;
// | { attribute_instance } real list_of_block_real_identifiers ;
// | { attribute_instance } realtime list_of_block_real_identifiers ;
// | { attribute_instance } event_declaration
// | { attribute_instance } local_parameter_declaration ;
// | { attribute_instance } parameter_declaration ;
#[derive(Clone, Copy)]
pub enum BlockItemDeclaration<'a> {
    Reg {
        signed: bool,
        range: Option<AstId<'a, Range<'a>>>,
        identifiers: AstIdRange<'a, VariableType<'a>>,
    },
    Integer(AstIdRange<'a, VariableType<'a>>),
    // @Incomplete
    Time,
    Real(AstIdRange<'a, VariableType<'a>>),
    Realtime,
    Event,
    LocalParameterDeclaration(AstId<'a, LocalParameterDeclaration<'a>>),
    ParameterDeclaration(AstId<'a, ParameterDeclaration<'a>>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 488
// non_port_module_item ::=
//   module_or_generate_item
// | generate_region
// | specify_block
// | { attribute_instance } parameter_declaration ;
// | { attribute_instance } specparam_declaration
#[derive(Clone, Copy)]
pub enum NonPortModuleItem<'a> {
    ModuleOrGenerateItem(AstId<'a, ModuleOrGenerateItem<'a>>),
    GenerateRegion(GenerateRegion<'a>),
    SpecifyBlock(SpecifyBlock<'a>),
    ParameterDeclaration(AstId<'a, ParameterDeclaration<'a>>),
    SpecParamDeclaration,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 495
// generate_region ::= generate { module_or_generate_item } endgenerate
#[derive(Clone, Copy)]
pub struct GenerateRegion<'a> {
    pub module_or_generate_item: AstIdRange<'a, ModuleOrGenerateItem<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
// local_parameter_declaration ::=
//   localparam [ signed ] [ range ] list_of_param_assignments
// | localparam parameter_type list_of_param_assignments
#[derive(Clone, Copy)]
pub struct LocalParameterDeclaration<'a> {
    pub typing: AstId<'a, ParameterDeclarationTyping<'a>>,
    pub assignments: AstIdRange<'a, ParamAssignment<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
// parameter_declaration ::=
//   parameter [ signed ] [ range ] list_of_param_assignments
// | parameter parameter_type list_of_param_assignments
#[derive(Clone, Copy)]
pub struct ParameterDeclaration<'a> {
    pub typing: AstId<'a, ParameterDeclarationTyping<'a>>,
    pub assignments: AstIdRange<'a, ParamAssignment<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 489
// parameter_declaration ::=
//   parameter [ signed ] [ range ] list_of_param_assignments
// | parameter parameter_type list_of_param_assignments
// parameter_type ::= integer | real | realtime | time
#[derive(Clone, Copy)]
pub enum ParameterDeclarationTyping<'a> {
    None(bool, Option<AstId<'a, Range<'a>>>),
    Integer,
    Real,
    Realtime,
    Time,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 491
// param_assignment ::= parameter_identifier = constant_mintypmax_expression
#[derive(Clone, Copy)]
pub struct ParamAssignment<'a> {
    pub param: AstItem<Identifier>,
    pub constant: AstId<'a, ConstantMinTypMaxExpression<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 492
// dimension ::= [ dimension_constant_expression : dimension_constant_expression ]
#[derive(Clone, Copy)]
pub struct Dimension<'a> {
    pub lhs: AstId<'a, ConstantExpr<'a>>,
    pub rhs: AstId<'a, ConstantExpr<'a>>,
}

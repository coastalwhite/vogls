use super::constant_expr::ConstantExpr;
use super::expr::Expr;
use super::module::Range;
use super::statement::NetLValue;
use super::{AstId, AstIdRange, AstItem, AttributeInstance, Identifier};

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
// udp_declaration ::=
//   { attribute_instance } primitive udp_identifier ( udp_port_list ) ;
//     udp_port_declaration { udp_port_declaration }
//     udp_body
//   endprimitive
// | { attribute_instance } primitive udp_identifier ( udp_declaration_port_list ) ;
//     udp_body
//   endprimitive
#[derive(Clone, Copy)]
pub struct UdpDeclaration {
    pub attribute_instances: AstIdRange<AttributeInstance>,
    pub identifier: AstItem<Identifier>,
    pub ports: UdpPorts,
    pub body: UdpBody,
}

#[derive(Clone, Copy)]
pub enum UdpPorts {
    PortList(AstIdRange<Identifier>, AstIdRange<UdpPortDeclaration>),
    DeclarationPortList(UdpDeclarationPortList),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
// udp_port_list ::= output_port_identifier , input_port_identifier { , input_port_identifier }
#[derive(Clone, Copy)]
pub struct UdpPortList {
    pub output_port_ident: AstItem<Identifier>,
    pub input_port_idents: AstIdRange<Identifier>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
// udp_declaration_port_list ::= udp_output_declaration , udp_input_declaration { , udp_input_declaration }
#[derive(Clone, Copy)]
pub struct UdpDeclarationPortList {
    pub output_decl: AstId<UdpOutputDeclaration>,
    pub input_decls: AstIdRange<UdpInputDeclaration>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
// udp_output_declaration ::=
//   { attribute_instance } output port_identifier
// | { attribute_instance } output reg port_identifier [ = constant_expression ]
#[derive(Clone, Copy)]
pub struct UdpOutputDeclaration {
    pub attribute_instances: AstIdRange<AttributeInstance>,
    pub is_reg: bool,
    pub port_identifier: AstItem<Identifier>,
    pub constant_expr: Option<AstId<ConstantExpr>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
// udp_input_declaration ::= { attribute_instance } input list_of_port_identifiers
#[derive(Clone, Copy)]
pub struct UdpInputDeclaration {
    pub attribute_instances: AstIdRange<AttributeInstance>,
    pub port_idents: AstIdRange<Identifier>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
// udp_reg_declaration ::= { attribute_instance } reg variable_identifier
#[derive(Clone, Copy)]
pub struct UdpRegDeclaration {
    pub attribute_instances: AstIdRange<AttributeInstance>,
    pub ident: AstItem<Identifier>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
// udp_port_declaration ::=
//   udp_output_declaration ;
// | udp_input_declaration ;
// | udp_reg_declaration ;
#[derive(Clone, Copy)]
pub enum UdpPortDeclaration {
    Output(AstId<UdpOutputDeclaration>),
    Input(AstId<UdpInputDeclaration>),
    Reg(AstId<UdpRegDeclaration>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
// udp_body ::= combinational_body | sequential_body
// combinational_body ::= table combinational_entry { combinational_entry } endtable
// sequential_body ::= [ udp_initial_statement ] table sequential_entry { sequential_entry } endtable
#[derive(Clone, Copy)]
pub enum UdpBody {
    Combinational(AstIdRange<UdpCombinationalEntry>),
    Sequential(
        Option<AstId<UdpInitialStatement>>,
        AstIdRange<UdpSequentialEntry>,
    ),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// udp_initial_statement ::= initial output_port_identifier = init_val ;
#[derive(Clone, Copy)]
pub struct UdpInitialStatement {
    pub output_port_ident: AstItem<Identifier>,
    pub init_val: AstItem<UdpInitVal>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// combinational_entry ::= level_input_list : output_symbol ;
#[derive(Clone, Copy)]
pub struct UdpCombinationalEntry {
    pub level_input_list: AstIdRange<UdpLevelSymbol>,
    pub output_symbol: AstItem<UdpOutputSymbol>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// sequential_entry ::= seq_input_list : current_state : next_state ;
// seq_input_list ::= level_input_list | edge_input_list
// edge_input_list ::= { level_symbol } edge_indicator { level_symbol }
#[derive(Clone, Copy)]
pub struct UdpSequentialEntry {
    pub level_list: AstIdRange<UdpLevelSymbol>,
    pub edge_list: Option<(AstId<UdpEdgeIndicator>, AstIdRange<UdpLevelSymbol>)>,
    pub current_state: AstItem<UdpLevelSymbol>,
    pub next_state: AstItem<UdpNextState>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// level_symbol ::= 0 | 1 | x | X | ? | b | B
#[derive(Clone, Copy)]
#[repr(u64)]
pub enum UdpLevelSymbol {
    L0,
    L1,
    X,
    QuestionMark,
    B,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// output_symbol ::= 0 | 1 | x | X
#[derive(Clone, Copy)]
#[repr(u64)]
pub enum UdpOutputSymbol {
    L0,
    L1,
    X,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// edge_symbol ::= r | R | f | F | p | P | n | N | *
#[derive(Clone, Copy)]
#[repr(u64)]
pub enum UdpEdgeSymbol {
    R,
    F,
    P,
    N,
    Star,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// next_state ::= output_symbol | -
#[derive(Clone, Copy)]
#[repr(u64)]
pub enum UdpNextState {
    Output(UdpOutputSymbol),
    Dash,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// edge_indicator ::= ( level_symbol level_symbol ) | edge_symbol
#[derive(Clone, Copy)]
#[repr(u64)]
pub enum UdpEdgeIndicator {
    Levels(AstItem<UdpLevelSymbol>, AstItem<UdpLevelSymbol>),
    Edge(AstItem<UdpEdgeSymbol>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// init_val ::= 1'b0 | 1'b1 | 1'bx | 1'bX | 1'B0 | 1'B1 | 1'Bx | 1'BX | 1 | 0
#[derive(Clone, Copy)]
#[repr(u64)]
pub enum UdpInitVal {
    L0,
    L1,
    X,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// udp_instantiation ::= udp_identifier [ drive_strength ] [ delay2 ] udp_instance { , udp_instance } ;
#[derive(Clone, Copy)]
pub struct UdpInstantiation {
    pub identifier: AstItem<Identifier>,
    pub instances: AstIdRange<UdpInstance>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// udp_instance ::= [ name_of_udp_instance ] ( output_terminal , input_terminal { , input_terminal } )
// name_of_udp_instance ::= udp_instance_identifier [ range ]
#[derive(Clone, Copy)]
pub struct UdpInstance {
    pub name: Option<(AstItem<Identifier>, Option<AstId<Range>>)>,
    pub output_terminal: AstId<NetLValue>,
    pub input_terminals: AstIdRange<Expr>,
}

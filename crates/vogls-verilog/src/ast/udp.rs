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
pub struct UdpDeclaration<'a> {
    pub attribute_instances: AstIdRange<'a, AttributeInstance<'a>>,
    pub identifier: AstItem<Identifier>,
    pub ports: UdpPorts<'a>,
    pub body: UdpBody<'a>,
}

#[derive(Clone, Copy)]
pub enum UdpPorts<'a> {
    PortList(
        AstIdRange<'a, Identifier>,
        AstIdRange<'a, UdpPortDeclaration<'a>>,
    ),
    DeclarationPortList(UdpDeclarationPortList<'a>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
// udp_port_list ::= output_port_identifier , input_port_identifier { , input_port_identifier }
#[derive(Clone, Copy)]
pub struct UdpPortList<'a> {
    pub output_port_ident: AstItem<Identifier>,
    pub input_port_idents: AstIdRange<'a, Identifier>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
// udp_declaration_port_list ::= udp_output_declaration , udp_input_declaration { , udp_input_declaration }
#[derive(Clone, Copy)]
pub struct UdpDeclarationPortList<'a> {
    pub output_decl: AstId<'a, UdpOutputDeclaration<'a>>,
    pub input_decls: AstIdRange<'a, UdpInputDeclaration<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
// udp_output_declaration ::=
//   { attribute_instance } output port_identifier
// | { attribute_instance } output reg port_identifier [ = constant_expression ]
#[derive(Clone, Copy)]
pub struct UdpOutputDeclaration<'a> {
    pub attribute_instances: AstIdRange<'a, AttributeInstance<'a>>,
    pub is_reg: bool,
    pub port_identifier: AstItem<Identifier>,
    pub constant_expr: Option<AstId<'a, ConstantExpr<'a>>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
// udp_input_declaration ::= { attribute_instance } input list_of_port_identifiers
#[derive(Clone, Copy)]
pub struct UdpInputDeclaration<'a> {
    pub attribute_instances: AstIdRange<'a, AttributeInstance<'a>>,
    pub port_idents: AstIdRange<'a, Identifier>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
// udp_reg_declaration ::= { attribute_instance } reg variable_identifier
#[derive(Clone, Copy)]
pub struct UdpRegDeclaration<'a> {
    pub attribute_instances: AstIdRange<'a, AttributeInstance<'a>>,
    pub ident: AstItem<Identifier>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
// udp_port_declaration ::=
//   udp_output_declaration ;
// | udp_input_declaration ;
// | udp_reg_declaration ;
#[derive(Clone, Copy)]
pub enum UdpPortDeclaration<'a> {
    Output(AstId<'a, UdpOutputDeclaration<'a>>),
    Input(AstId<'a, UdpInputDeclaration<'a>>),
    Reg(AstId<'a, UdpRegDeclaration<'a>>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
// udp_body ::= combinational_body | sequential_body
// combinational_body ::= table combinational_entry { combinational_entry } endtable
// sequential_body ::= [ udp_initial_statement ] table sequential_entry { sequential_entry } endtable
#[derive(Clone, Copy)]
pub enum UdpBody<'a> {
    Combinational(AstIdRange<'a, UdpCombinationalEntry<'a>>),
    Sequential(
        Option<AstId<'a, UdpInitialStatement>>,
        AstIdRange<'a, UdpSequentialEntry<'a>>,
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
pub struct UdpCombinationalEntry<'a> {
    pub level_input_list: AstIdRange<'a, UdpLevelSymbol>,
    pub output_symbol: AstItem<UdpOutputSymbol>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// sequential_entry ::= seq_input_list : current_state : next_state ;
// seq_input_list ::= level_input_list | edge_input_list
// edge_input_list ::= { level_symbol } edge_indicator { level_symbol }
#[derive(Clone, Copy)]
pub struct UdpSequentialEntry<'a> {
    pub level_list: AstIdRange<'a, UdpLevelSymbol>,
    pub edge_list: Option<(AstId<'a, UdpEdgeIndicator>, AstIdRange<'a, UdpLevelSymbol>)>,
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
pub struct UdpInstantiation<'a> {
    pub identifier: AstItem<Identifier>,
    pub instances: AstIdRange<'a, UdpInstance<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// udp_instance ::= [ name_of_udp_instance ] ( output_terminal , input_terminal { , input_terminal } )
// name_of_udp_instance ::= udp_instance_identifier [ range ]
#[derive(Clone, Copy)]
pub struct UdpInstance<'a> {
    pub name: Option<(AstItem<Identifier>, Option<AstId<'a, Range<'a>>>)>,
    pub output_terminal: AstId<'a, NetLValue<'a>>,
    pub input_terminals: AstIdRange<'a, Expr<'a>>,
}

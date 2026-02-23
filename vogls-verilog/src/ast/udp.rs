use super::constant_expr::ConstantExpr;
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
    PortList(AstId<UdpPortList>, AstIdRange<UdpPortDeclaration>),
    DeclarationPortList(AstId<UdpDeclarationPortList>),
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
    attribute_instances: AstIdRange<AttributeInstance>,
    is_reg: bool,
    port_identifier: AstItem<Identifier>,
    constant_expr: AstId<ConstantExpr>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
// udp_input_declaration ::= { attribute_instance } input list_of_port_identifiers
#[derive(Clone, Copy)]
pub struct UdpInputDeclaration {
    attribute_instances: AstIdRange<AttributeInstance>,
    port_idents: AstIdRange<Identifier>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 496
// udp_reg_declaration ::= { attribute_instance } reg variable_identifier
#[derive(Clone, Copy)]
pub struct UdpRegDeclaration {
    attribute_instances: AstIdRange<AttributeInstance>,
    ident: AstIdRange<Identifier>,
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
    Sequential(AstId<UdpInitialStatement>, AstIdRange<UdpCombinationalEntry>),
}


// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// udp_initial_statement ::= initial output_port_identifier = init_val ;
// init_val ::= 1'b0 | 1'b1 | 1'bx | 1'bX | 1'B0 | 1'B1 | 1'Bx | 1'BX | 1 | 0
#[derive(Clone, Copy)]
pub struct UdpInitialStatement {
    pub output_port_ident: AstItem<Identifier>,
    // @Incorrect. This should actually accept a very small subset.
    pub init_val: ConstantExpr,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// combinational_entry ::= level_input_list : output_symbol ;
#[derive(Clone, Copy)]
pub struct UdpCombinationalEntry {
    level_input_list: AstIdRange<UdpLevelSymbol>,
    output_symbol: AstItem<UdpOutputSymbol>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// sequential_entry ::= seq_input_list : current_state : next_state ;
pub struct UdpSequentialEntry {
    level_list: AstIdRange<UdpLevelSymbol>,
    edge_list: Option<(AstId<EdgeIndicator>, AstIdRange<UdpLevelSymbol>)>,
    current_state: AstItem<UdpLevelSymbol>,
    next_state: AstItem<UdpNextState>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// level_symbol ::= 0 | 1 | x | X | ? | b | B
#[derive(Clone, Copy)]
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
pub enum UdpOutputSymbol {
    L0,
    L1,
    X,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// edge_symbol ::= r | R | f | F | p | P | n | N | *
#[derive(Clone, Copy)]
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
pub enum UdpNextState {
    Output(UdpOutputSymbol),
    Dash,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// edge_indicator ::= ( level_symbol level_symbol ) | edge_symbol
pub enum EdgeIndicator {
    Levels(AstItem<UdpLevelSymbol>, AstItem<UdpLevelSymbol>),
    Edge(AstItem<UdpEdgeSymbol>),
}

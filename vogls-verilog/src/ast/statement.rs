use super::expr::Expr;
use super::{AstId, AstIdRange, AstItem, DecimalRef, Identifier, TextRef};

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
#[derive(Clone, Copy)]
pub enum Statement {
    BlockingAssignment(AstId<BlockingAssignment>),
    CaseStatement,
    ConditionalStatement,
    DisableStatement,
    EventTrigger,
    LoopStatement,
    NonBlockingAssignment(AstId<NonBlockingAssignment>),
    ParBlock,
    ProceduralContinuousAssignments,
    ProceduralTimingControlStatement(AstId<ProceduralTimingControl>, Option<AstId<Statement>>),
    SeqBlock(AstId<SeqBlock>),
    SystemTaskEnable(AstId<SystemTaskEnable>),
    TaskEnable,
    WaitStatement,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 506
// net_lvalue ::=
//   hierarchical_net_identifier [ { [ constant_expression ] } [ constant_range_expression ] ]
// | { net_lvalue { , net_lvalue } }
#[derive(Clone, Copy)]
pub struct NetLValue {
    // @Incomplete
    pub ident: AstItem<Identifier>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 506
// variable_lvalue ::=
//   hierarchical_variable_identifier [ { [ expression ] } [ range_expression ] ]
//   | { variable_lvalue { , variable_lvalue } }
#[derive(Clone, Copy)]
pub struct VariableLValue {
    // @Incomplete
    pub ident: AstItem<Identifier>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
// delay_or_event_control ::=
//   delay_control
//   | event_control
//   | repeat ( expression ) event_control
#[derive(Clone, Copy)]
pub enum DelayOrEventControl {
    DelayControl(AstId<DelayControl>),
    EventControl(AstId<EventControl>),
    // @Incomplete
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
// delay_control ::=
//   # delay_value
// | # ( mintypmax_expression )
#[derive(Clone, Copy)]
pub enum DelayControl {
    DelayValue(AstId<DelayValue>), // @Incomplete: | # ( mintypmax_expression )
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
// event_control ::=
//   @ hierarchical_event_identifier
// | @ ( event_expression )
// | @*
// | @ (*)
#[derive(Clone, Copy)]
pub enum EventControl {
    // @Incomplete: @ hierarchical_event_identifier
    EventExpression(AstId<EventExpression>), // @Incomplete: | @*
                                             // @Incomplete: | @ (*)
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
// event_expression ::=
//   expression
// | posedge expression
// | negedge expression
// | event_expression or event_expression
#[derive(Clone, Copy)]
pub enum EventExpression {
    Expression(AstId<Expr>),
    Posedge(AstId<Expr>),
    Negedge(AstId<Expr>),
    OrList(AstId<EventExpression>, AstId<EventExpression>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 491
// delay_value ::=
//   unsigned_number
// | real_number
// | identifier
#[derive(Clone, Copy)]
pub enum DelayValue {
    UnsignedNumber(DecimalRef),
    Identifier(Identifier),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// blocking_assignment ::= variable_lvalue = [ delay_or_event_control ] expression
#[derive(Clone, Copy)]
pub struct BlockingAssignment {
    pub variable_lvalue: AstId<VariableLValue>,
    pub delay_or_event_control: Option<AstId<DelayOrEventControl>>,
    pub expression: AstId<Expr>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// nonblocking_assignment ::= variable_lvalue <= [ delay_or_event_control ] expression
#[derive(Clone, Copy)]
pub struct NonBlockingAssignment {
    pub variable_lvalue: AstId<VariableLValue>,
    pub delay_or_event_control: Option<AstId<DelayOrEventControl>>,
    pub expression: AstId<Expr>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
// procedural_timing_control ::=
//   delay_control
// | event_control
#[derive(Clone, Copy)]
pub enum ProceduralTimingControl {
    DelayControl(AstId<DelayControl>),
    EventControl(AstId<EventControl>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
// seq_block ::= begin [ : block_identifier { block_item_declaration } ] { statement } end
#[derive(Clone, Copy)]
pub struct SeqBlock {
    pub statements: AstIdRange<Statement>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
// system_task_enable ::= system_task_identifier [ ( [ expression ] { , [ expression ] } ) ] ;
#[derive(Clone, Copy)]
pub struct SystemTaskEnable {
    pub system_task_identifier: AstItem<SystemTaskIdentifier>,
    pub expressions: AstIdRange<Expr>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 508
// system_task_identifier ::= $[ a-zA-Z0-9_$ ]{ [ a-zA-Z0-9_$ ] }
#[derive(Clone, Copy)]
pub struct SystemTaskIdentifier(pub TextRef);

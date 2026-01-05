use super::constant_expr::{ConstantExpr, ConstantRangeExpression};
use super::expr::Expr;
use super::{
    AstId, AstIdRange, AstItem, AttributeInstance, DecimalRef, Identifier, RangeExpression, TextRef,
};

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
pub struct Statement {
    pub attr_instances: AstIdRange<AttributeInstance>,
    pub content: StatementContent,
}

#[derive(Clone, Copy)]
pub enum StatementContent {
    BlockingAssignment(AstId<BlockingAssignment>),
    CaseStatement(AstId<CaseStatement>),
    ConditionalStatement(AstId<ConditionalStatement>),
    DisableStatement,
    EventTrigger,
    LoopStatement(AstId<LoopStatement>),
    NonBlockingAssignment(AstId<NonBlockingAssignment>),
    ParBlock,
    ProceduralContinuousAssignments,
    ProceduralTimingControlStatement(AstId<ProceduralTimingControlStatement>),
    SeqBlock(AstId<SeqBlock>),
    SystemTaskEnable(AstId<SystemTaskEnable>),
    TaskEnable(AstId<TaskEnable>),
    WaitStatement(AstId<WaitStatement>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
// procedural_timing_control_statement ::= procedural_timing_control statement_or_null
#[derive(Clone, Copy)]
pub struct ProceduralTimingControlStatement {
    pub procedural_timing_control: AstId<ProceduralTimingControl>,
    pub statement_or_null: AstId<StatementOrNull>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 506
// net_lvalue ::=
//   hierarchical_net_identifier [ { [ constant_expression ] } [ constant_range_expression ] ]
// | { net_lvalue { , net_lvalue } }
#[derive(Clone, Copy)]
pub struct NetLValue(pub AstIdRange<NetLValueFlat>);
#[derive(Clone, Copy)]
pub struct NetLValueFlat {
    pub ident: AstItem<Identifier>,
    pub constant_exprs: AstIdRange<ConstantExpr>,
    pub constant_range_expression: Option<AstId<ConstantRangeExpression>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 506
// variable_lvalue ::=
//   hierarchical_variable_identifier [ { [ expression ] } [ range_expression ] ]
//   | { variable_lvalue { , variable_lvalue } }
#[derive(Clone, Copy)]
pub struct VariableLValue(pub AstIdRange<VariableLValueFlat>);

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 506
// variable_lvalue ::=
//   hierarchical_variable_identifier [ { [ expression ] } [ range_expression ] ]
//   | { variable_lvalue { , variable_lvalue } }
#[derive(Clone, Copy)]
pub struct VariableLValueFlat {
    pub ident: AstItem<Identifier>,
    pub exprs: AstIdRange<Expr>,
    pub range_expression: Option<AstId<RangeExpression>>,
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
    Star,
    EventExpression(EventExpression), // @Incomplete: | @*
                                      // @Incomplete: | @ (*)
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
// event_expression ::=
//   expression
// | posedge expression
// | negedge expression
// | event_expression or event_expression
// | event_expression, event_expression
#[derive(Clone, Copy)]
pub enum EventExpressionPrimary {
    Expression(AstId<Expr>),
    Posedge(AstId<Expr>),
    Negedge(AstId<Expr>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
// event_expression ::=
//   expression
// | posedge expression
// | negedge expression
// | event_expression or event_expression
// | event_expression, event_expression
#[derive(Clone, Copy)]
pub struct EventExpression(pub AstIdRange<EventExpressionPrimary>);

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

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// variable_assignment ::= variable_lvalue = expression
#[derive(Clone, Copy)]
pub struct VariableAssignment {
    pub lvalue: AstId<VariableLValue>,
    pub expr: AstId<Expr>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
// loop_statement ::=
//   forever statement
// | repeat ( expression ) statement
// | while ( expression ) statement
// | for ( variable_assignment ; expression ; variable_assignment ) statement
#[derive(Clone, Copy)]
pub struct LoopStatement {
    pub variant: LoopStatementVariant,
    pub statement: AstId<Statement>,
}

#[derive(Clone, Copy)]
pub enum LoopStatementVariant {
    Forever,
    Repeat(AstId<Expr>),
    While(AstId<Expr>),
    For(
        AstId<VariableAssignment>,
        AstId<Expr>,
        AstId<VariableAssignment>,
    ),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
// case_statement ::=
//   case ( expression )  case_item { case_item } endcase
// | casez ( expression ) case_item { case_item } endcase
// | casex ( expression ) case_item { case_item } endcase
#[derive(Clone, Copy)]
pub struct CaseStatement {
    pub variant: CaseStatementVariant,
    pub expr: AstId<Expr>,
    pub items: AstIdRange<CaseItem>,
}

#[derive(Clone, Copy)]
pub enum CaseStatementVariant {
    Case,
    CaseZ,
    CaseX,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
// case_item ::=
//   expression { , expression } : statement_or_null
// | default [ : ] statement_or_null
#[derive(Clone, Copy)]
pub struct CaseItem {
    pub pattern: AstItem<CaseItemPattern>,
    pub statement_or_null: AstId<StatementOrNull>,
}

#[derive(Clone, Copy)]
pub enum CaseItemPattern {
    Default,
    Expressions(AstIdRange<Expr>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
// statement_or_null ::= statement | { attribute_instance } ;
#[derive(Clone, Copy)]
pub enum StatementOrNull {
    Attribute(AstIdRange<AttributeInstance>),
    Statement(AstId<Statement>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
// conditional_statement ::
//   if ( expression ) statement_or_null
//   [ else statement_or_null ]
// | if_else_if_statement
// if_else_if_statement ::=
//   if ( expression ) statement_or_null
//   { else if ( expression ) statement_or_null }
//   [ else statement_or_null ]
#[derive(Clone, Copy)]
pub struct ConditionalStatement {
    pub if_branch: IfBranch,
    pub else_ifs: AstIdRange<IfBranch>,
    pub else_branch: Option<AstId<StatementOrNull>>,
}

#[derive(Clone, Copy)]
pub struct IfBranch {
    pub condition: AstId<Expr>,
    pub statement: AstId<StatementOrNull>,
}

#[derive(Clone, Copy)]
pub struct TaskEnable {
    // @Incomplete
    pub ident: AstItem<Identifier>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
// wait_statement ::= wait ( expression ) statement_or_null
#[derive(Clone, Copy)]
pub struct WaitStatement {
    pub expression: AstId<Expr>,
    pub statement_or_null: AstId<StatementOrNull>,
}

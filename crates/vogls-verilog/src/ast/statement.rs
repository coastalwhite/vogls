use vogls_frontend::ident_table::IdentId;

use super::constant_expr::{ConstantBitSlice, ConstantExpr};
use super::expr::{BitSlice, Expr};
use super::module::BlockItemDeclaration;
use super::{AstId, AstIdRange, AstItem, AttributeInstance, DecimalRef, HIdent, Identifier};

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
pub struct Statement<'a> {
    pub attr_instances: AstIdRange<'a, AttributeInstance<'a>>,
    pub content: StatementContent<'a>,
}

#[derive(Clone, Copy)]
pub enum StatementContent<'a> {
    BlockingAssignment(AstId<'a, BlockingAssignment<'a>>),
    CaseStatement(AstId<'a, CaseStatement<'a>>),
    ConditionalStatement(AstId<'a, ConditionalStatement<'a>>),
    DisableStatement,
    EventTrigger,
    LoopStatement(AstId<'a, LoopStatement<'a>>),
    NonBlockingAssignment(AstId<'a, NonBlockingAssignment<'a>>),
    ParBlock(AstId<'a, ParBlock<'a>>),
    ProceduralContinuousAssignments,
    ProceduralTimingControlStatement(AstId<'a, ProceduralTimingControlStatement<'a>>),
    SeqBlock(AstId<'a, SeqBlock<'a>>),
    SystemTaskEnable(AstId<'a, SystemTaskEnable<'a>>),
    TaskEnable(AstId<'a, TaskEnable<'a>>),
    WaitStatement(AstId<'a, WaitStatement<'a>>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
// procedural_timing_control_statement ::= procedural_timing_control statement_or_null
#[derive(Clone, Copy)]
pub struct ProceduralTimingControlStatement<'a> {
    pub procedural_timing_control: AstId<'a, ProceduralTimingControl<'a>>,
    pub statement_or_null: AstId<'a, StatementOrNull<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 506
// net_lvalue ::=
//   hierarchical_net_identifier [ { [ constant_expression ] } [ constant_range_expression ] ]
// | { net_lvalue { , net_lvalue } }
#[derive(Clone, Copy)]
pub struct NetLValue<'a>(pub AstIdRange<'a, NetLValueFlat<'a>>);
#[derive(Clone, Copy)]
pub struct NetLValueFlat<'a> {
    pub ident: HIdent<'a>,
    pub constant_exprs: AstIdRange<'a, ConstantExpr<'a>>,
    pub constant_range_expression: Option<AstId<'a, ConstantBitSlice<'a>>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 506
// variable_lvalue ::=
//   hierarchical_variable_identifier [ { [ expression ] } [ range_expression ] ]
//   | { variable_lvalue { , variable_lvalue } }
#[derive(Clone, Copy)]
pub struct VariableLValue<'a>(pub AstIdRange<'a, VariableLValueFlat<'a>>);

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 506
// variable_lvalue ::=
//   hierarchical_variable_identifier [ { [ expression ] } [ range_expression ] ]
//   | { variable_lvalue { , variable_lvalue } }
#[derive(Clone, Copy)]
pub struct VariableLValueFlat<'a> {
    pub ident: HIdent<'a>,
    pub exprs: AstIdRange<'a, Expr<'a>>,
    pub range_expression: Option<AstId<'a, BitSlice<'a>>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
// delay_or_event_control ::=
//   delay_control
//   | event_control
//   | repeat ( expression ) event_control
#[derive(Clone, Copy)]
pub enum DelayOrEventControl<'a> {
    DelayControl(AstId<'a, DelayControl<'a>>),
    EventControl(AstId<'a, EventControl<'a>>),
    // @Incomplete
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
// delay_control ::=
//   # delay_value
// | # ( mintypmax_expression )
#[derive(Clone, Copy)]
pub enum DelayControl<'a> {
    DelayValue(AstId<'a, DelayValue>),
    MinTypMax(AstId<'a, MinTypMaxExpression<'a>>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 505
// mintypmax_expression ::=
//   expression
// | expression : expression : expression
#[derive(Clone, Copy)]
pub struct MinTypMaxExpression<'a> {
    pub typical: AstId<'a, Expr<'a>>,
    pub min_max: Option<(AstId<'a, Expr<'a>>, AstId<'a, Expr<'a>>)>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
// event_control ::=
//   @ hierarchical_event_identifier
// | @ ( event_expression )
// | @*
// | @ (*)
#[derive(Clone, Copy)]
pub enum EventControl<'a> {
    // @Incomplete: @ hierarchical_event_identifier
    Star,
    EventExpression(EventExpression<'a>), // @Incomplete: | @*
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
pub enum EventExpressionPrimary<'a> {
    Expression(AstId<'a, Expr<'a>>),
    Posedge(AstId<'a, Expr<'a>>),
    Negedge(AstId<'a, Expr<'a>>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
// event_expression ::=
//   expression
// | posedge expression
// | negedge expression
// | event_expression or event_expression
// | event_expression, event_expression
#[derive(Clone, Copy)]
pub struct EventExpression<'a>(pub AstIdRange<'a, EventExpressionPrimary<'a>>);

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 491
// delay3 ::=
//   # delay_value
// | # ( mintypmax_expression [ , mintypmax_expression [ , mintypmax_expression ] ] )
#[derive(Clone, Copy)]
pub enum Delay3<'a> {
    Value(AstId<'a, DelayValue>),
    Single(AstId<'a, MinTypMaxExpression<'a>>),
    Double(
        AstId<'a, MinTypMaxExpression<'a>>,
        AstId<'a, MinTypMaxExpression<'a>>,
    ),
    Triple(
        AstId<'a, MinTypMaxExpression<'a>>,
        AstId<'a, MinTypMaxExpression<'a>>,
        AstId<'a, MinTypMaxExpression<'a>>,
    ),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 491
// delay2 ::=
//   # delay_value
// | # ( mintypmax_expression [ , mintypmax_expression ] )
#[derive(Clone, Copy)]
pub enum Delay2<'a> {
    Value(AstId<'a, DelayValue>),
    Tuple(
        AstId<'a, MinTypMaxExpression<'a>>,
        Option<AstId<'a, MinTypMaxExpression<'a>>>,
    ),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 491
// delay_value ::=
//   unsigned_number
// | real_number
// | identifier
#[derive(Clone, Copy)]
pub enum DelayValue {
    UnsignedNumber(DecimalRef),
    RealNumber(f64),
    Identifier(Identifier),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// blocking_assignment ::= variable_lvalue = [ delay_or_event_control ] expression
#[derive(Clone, Copy)]
pub struct BlockingAssignment<'a> {
    pub variable_lvalue: AstId<'a, VariableLValue<'a>>,
    pub delay_or_event_control: Option<AstId<'a, DelayOrEventControl<'a>>>,
    pub expression: AstId<'a, Expr<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// nonblocking_assignment ::= variable_lvalue <= [ delay_or_event_control ] expression
#[derive(Clone, Copy)]
pub struct NonBlockingAssignment<'a> {
    pub variable_lvalue: AstId<'a, VariableLValue<'a>>,
    pub delay_or_event_control: Option<AstId<'a, DelayOrEventControl<'a>>>,
    pub expression: AstId<'a, Expr<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
// procedural_timing_control ::=
//   delay_control
// | event_control
#[derive(Clone, Copy)]
pub enum ProceduralTimingControl<'a> {
    DelayControl(AstId<'a, DelayControl<'a>>),
    EventControl(AstId<'a, EventControl<'a>>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// par_block ::= fork [ : block_identifier { block_item_declaration } ] { statement } join
#[derive(Clone, Copy)]
pub struct ParBlock<'a> {
    pub block: Option<AstId<'a, Block<'a>>>,
    pub statements: AstIdRange<'a, Statement<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
// seq_block ::= begin [ : block_identifier { block_item_declaration } ] { statement } end
#[derive(Clone, Copy)]
pub struct SeqBlock<'a> {
    pub block: Option<AstId<'a, Block<'a>>>,
    pub statements: AstIdRange<'a, Statement<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
// block_identifier { block_item_declaration }
#[derive(Clone, Copy)]
pub struct Block<'a> {
    pub block_identifier: AstItem<Identifier>,
    pub block_item_decls: AstIdRange<'a, BlockItemDeclaration<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
// system_task_enable ::= system_task_identifier [ ( [ expression ] { , [ expression ] } ) ] ;
#[derive(Clone, Copy)]
pub struct SystemTaskEnable<'a> {
    pub system_task_identifier: AstItem<SystemTaskIdentifier>,
    pub expressions: AstIdRange<'a, Expr<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 508
// system_task_identifier ::= $[ a-zA-Z0-9_$ ]{ [ a-zA-Z0-9_$ ] }
#[derive(Clone, Copy)]
pub struct SystemTaskIdentifier(pub IdentId);

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// variable_assignment ::= variable_lvalue = expression
#[derive(Clone, Copy)]
pub struct VariableAssignment<'a> {
    pub lvalue: AstId<'a, VariableLValue<'a>>,
    pub expr: AstId<'a, Expr<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
// loop_statement ::=
//   forever statement
// | repeat ( expression ) statement
// | while ( expression ) statement
// | for ( variable_assignment ; expression ; variable_assignment ) statement
#[derive(Clone, Copy)]
pub struct LoopStatement<'a> {
    pub variant: LoopStatementVariant<'a>,
    pub statement: AstId<'a, Statement<'a>>,
}

#[derive(Clone, Copy)]
pub enum LoopStatementVariant<'a> {
    Forever,
    Repeat(AstId<'a, Expr<'a>>),
    While(AstId<'a, Expr<'a>>),
    For(
        AstId<'a, VariableAssignment<'a>>,
        AstId<'a, Expr<'a>>,
        AstId<'a, VariableAssignment<'a>>,
    ),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
// case_statement ::=
//   case ( expression )  case_item { case_item } endcase
// | casez ( expression ) case_item { case_item } endcase
// | casex ( expression ) case_item { case_item } endcase
#[derive(Clone, Copy)]
pub struct CaseStatement<'a> {
    pub variant: CaseStatementVariant,
    pub expr: AstId<'a, Expr<'a>>,
    pub items: AstIdRange<'a, CaseItem<'a>>,
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
pub struct CaseItem<'a> {
    pub pattern: AstItem<CaseItemPattern<'a>>,
    pub statement_or_null: AstId<'a, StatementOrNull<'a>>,
}

#[derive(Clone, Copy)]
pub enum CaseItemPattern<'a> {
    Default,
    Expressions(AstIdRange<'a, Expr<'a>>),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
// statement_or_null ::= statement | { attribute_instance } ;
#[derive(Clone, Copy)]
pub enum StatementOrNull<'a> {
    Attribute(AstIdRange<'a, AttributeInstance<'a>>),
    Statement(AstId<'a, Statement<'a>>),
}

impl<'a> StatementOrNull<'a> {
    pub fn into_stmt_range(self) -> AstIdRange<'a, Statement<'a>> {
        match self {
            StatementOrNull::Attribute(_) => AstIdRange::empty(),
            StatementOrNull::Statement(stmt) => AstIdRange::single(stmt),
        }
    }
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
pub struct ConditionalStatement<'a> {
    pub if_branch: IfBranch<'a>,
    pub else_ifs: AstIdRange<'a, IfBranch<'a>>,
    pub else_branch: Option<AstId<'a, StatementOrNull<'a>>>,
}

#[derive(Clone, Copy)]
pub struct IfBranch<'a> {
    pub condition: AstId<'a, Expr<'a>>,
    pub statement: AstId<'a, StatementOrNull<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
// task_enable ::= hierarchical_task_identifier [ ( expression { , expression } ) ] ;
#[derive(Clone, Copy)]
pub struct TaskEnable<'a> {
    pub ident: AstItem<Identifier>,
    pub exprs: AstIdRange<'a, Expr<'a>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 499
// wait_statement ::= wait ( expression ) statement_or_null
#[derive(Clone, Copy)]
pub struct WaitStatement<'a> {
    pub expression: AstId<'a, Expr<'a>>,
    pub statement_or_null: AstId<'a, StatementOrNull<'a>>,
}

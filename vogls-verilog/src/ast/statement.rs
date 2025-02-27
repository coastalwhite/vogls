use super::expr::Expr;
use super::{AstId, AstIdRange, AstItem, IdentRef};

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
    ProceduralTimingControlStatement,
    SeqBlock(AstId<SeqBlock>),
    SystemTaskEnable,
    TaskEnable,
    WaitStatement,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 506
// variable_lvalue ::=
//   hierarchical_variable_identifier [ { [ expression ] } [ range_expression ] ]
//   | { variable_lvalue { , variable_lvalue } }
#[derive(Clone, Copy)]
pub struct VariableLValue {
    // @Incomplete
    pub ident: AstItem<IdentRef>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// blocking_assignment ::= variable_lvalue = [ delay_or_event_control ] expression
#[derive(Clone, Copy)]
pub struct BlockingAssignment {
    pub variable_lvalue: AstId<VariableLValue>,
    pub expression: AstId<Expr>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 497
// nonblocking_assignment ::= variable_lvalue <= [ delay_or_event_control ] expression
#[derive(Clone, Copy)]
pub struct NonBlockingAssignment {
    pub variable_lvalue: AstId<VariableLValue>,
    pub expression: AstId<Expr>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 498
// seq_block ::= begin [ : block_identifier { block_item_declaration } ] { statement } end
#[derive(Clone, Copy)]
pub struct SeqBlock {
    pub statements: AstIdRange<Statement>,
}

use super::constant_expr::{ConstantMinTypMaxExpression, ConstantRangeExpression};
use super::expr::Expr;
use super::{AstId, AstIdRange, AstItem, Identifier};

// specify_block ::= specify { specify_item } endspecify
// specify_item ::=
// specparam_declaration
// | pulsestyle_declaration
// | showcancelled_declaration
// | path_declaration
// | system_timing_check
// pulsestyle_declaration ::=
// pulsestyle_onevent list_of_path_outputs ;
// | pulsestyle_ondetect list_of_path_outputs ;
// showcancelled_declaration ::=
// showcancelled list_of_path_outputs ;
// | noshowcancelled list_of_path_outputs ;
//
// path_declaration ::=
// simple_path_declaration ;
// | edge_sensitive_path_declaration ;
// | state_dependent_path_declaration ;
// simple_path_declaration ::=
// parallel_path_description = path_delay_value
// | full_path_description = path_delay_value
// parallel_path_description ::=
// ( specify_input_terminal_descriptor [ polarity_operator ] => specify_output_terminal_descriptor )
// full_path_description ::=
// ( list_of_path_inputs [ polarity_operator ] *> list_of_path_outputs )
// list_of_path_inputs ::=
// specify_input_terminal_descriptor { , specify_input_terminal_descriptor }
// list_of_path_outputs ::=
// specify_output_terminal_descriptor { , specify_output_terminal_descriptor }
//
// specify_input_terminal_descriptor ::=
// input_identifier [ [ constant_range_expression ] ]
// specify_output_terminal_descriptor ::=
// output_identifier [ [ constant_range_expression ] ]
// input_identifier ::= input_port_identifier | inout_port_identifier
// output_identifier ::= output_port_identifier | inout_port_identifier
//
// path_delay_value ::=
// list_of_path_delay_expressions
//
// ---
//
// | ( list_of_path_delay_expressions )
// list_of_path_delay_expressions ::=
// t_path_delay_expression
// | trise_path_delay_expression , tfall_path_delay_expression
// | trise_path_delay_expression , tfall_path_delay_expression , tz_path_delay_expression
// | t01_path_delay_expression , t10_path_delay_expression , t0z_path_delay_expression ,
// tz1_path_delay_expression , t1z_path_delay_expression , tz0_path_delay_expression
// | t01_path_delay_expression , t10_path_delay_expression , t0z_path_delay_expression ,
// tz1_path_delay_expression , t1z_path_delay_expression , tz0_path_delay_expression ,
// t0x_path_delay_expression , tx1_path_delay_expression , t1x_path_delay_expression ,
// tx0_path_delay_expression , txz_path_delay_expression , tzx_path_delay_expression
// t_path_delay_expression ::= path_delay_expression
// trise_path_delay_expression ::= path_delay_expression
// tfall_path_delay_expression ::= path_delay_expression
// tz_path_delay_expression ::= path_delay_expression
// t01_path_delay_expression ::= path_delay_expression
// t10_path_delay_expression ::= path_delay_expression
// t0z_path_delay_expression ::= path_delay_expression
// tz1_path_delay_expression ::= path_delay_expression
// t1z_path_delay_expression ::= path_delay_expression
// tz0_path_delay_expression ::= path_delay_expression
// t0x_path_delay_expression ::= path_delay_expression
// tx1_path_delay_expression ::= path_delay_expression
// t1x_path_delay_expression ::= path_delay_expression
// tx0_path_delay_expression ::= path_delay_expression
// txz_path_delay_expression ::= path_delay_expression
// tzx_path_delay_expression ::= path_delay_expression
// path_delay_expression ::= constant_mintypmax_expression
// edge_sensitive_path_declaration ::=
// parallel_edge_sensitive_path_description = path_delay_value
// | full_edge_sensitive_path_description = path_delay_value
// parallel_edge_sensitive_path_description ::=
// ( [ edge_identifier ] specify_input_terminal_descriptor =>
// ( specify_output_terminal_descriptor [ polarity_operator ] : data_source_expression ) )
// full_edge_sensitive_path_description ::=
// ( [ edge_identifier ] list_of_path_inputs *>
// ( list_of_path_outputs [ polarity_operator ] : data_source_expression ) )
// data_source_expression ::= expression
// edge_identifier ::= posedge | negedge
// state_dependent_path_declaration ::=
// if ( module_path_expression ) simple_path_declaration
// | if ( module_path_expression ) edge_sensitive_path_declaration
// | ifnone simple_path_declaration
// polarity_operator ::= + | -
//
// ---
//
//system_timing_check ::=
// $setup_timing_check
// | $hold_timing_check
// | $setuphold_timing_check
// | $recovery_timing_check
// | $removal_timing_check
// | $recrem_timing_check
// | $skew_timing_check
// | $timeskew_timing_check
// | $fullskew_timing_check
// | $period_timing_check
// | $width_timing_check
// | $nochange_timing_check
// $setup_timing_check ::=
// $setup ( data_event , reference_event , timing_check_limit [ , [ notifier ] ] ) ;
// $hold_timing_check ::=
// $hold ( reference_event , data_event , timing_check_limit [ , [ notifier ] ] ) ;
// $setuphold_timing_check ::=
// $setuphold ( reference_event , data_event , timing_check_limit , timing_check_limit
// [ , [ notifier ] [ , [ stamptime_condition ] [ , [ checktime_condition ]
// [ , [ delayed_reference ] [ , [ delayed_data ] ] ] ] ] ] ) ;
// $recovery_timing_check ::=
// $recovery ( reference_event , data_event , timing_check_limit [ , [ notifier ] ] ) ;
// $removal_timing_check ::=
// $removal ( reference_event , data_event , timing_check_limit [ , [ notifier ] ] ) ;
// $recrem_timing_check ::=
// $recrem ( reference_event , data_event , timing_check_limit , timing_check_limit
// [ , [ notifier ] [ , [ stamptime_condition ] [ , [ checktime_condition ]
// [ , [ delayed_reference ] [ , [ delayed_data ] ] ] ] ] ] ) ;
// $skew_timing_check ::=
// $skew ( reference_event , data_event , timing_check_limit [ , [ notifier ] ] ) ;
// $timeskew_timing_check ::=
// $timeskew ( reference_event , data_event , timing_check_limit
// [ , [ notifier ] [ , [ event_based_flag ] [ , [ remain_active_flag ] ] ] ] ) ;
// $fullskew_timing_check ::=
// $fullskew ( reference_event , data_event , timing_check_limit , timing_check_limit
// [ , [ notifier ] [ , [ event_based_flag ] [ , [ remain_active_flag ] ] ] ] ) ;
// $period_timing_check ::=
// $period ( controlled_reference_event , timing_check_limit [ , [ notifier ] ] ) ;
// $width_timing_check ::=
// $width ( controlled_reference_event , timing_check_limit
// [ , threshold [ , notifier ] ] ) ;
// $nochange_timing_check ::=
// $nochange ( reference_event , data_event , start_edge_offset ,
// end_edge_offset [ , [ notifier ] ] ) ;
//
// checktime_condition ::= mintypmax_expression
// controlled_reference_event ::= controlled_timing_check_event
// data_event ::= timing_check_event
// delayed_data ::=
// terminal_identifier
// | terminal_identifier [ constant_mintypmax_expression ]
// delayed_reference ::=
// terminal_identifier
// | terminal_identifier [ constant_mintypmax_expression ]
// end_edge_offset ::= mintypmax_expression
// event_based_flag ::= constant_expression
// notifier ::= variable_identifier
// reference_event ::= timing_check_event
// remain_active_flag ::= constant_expression
// stamptime_condition ::= mintypmax_expression
// start_edge_offset ::= mintypmax_expression
// threshold ::= constant_expression
// timing_check_limit ::= expression
//
// ---
// timing_check_event ::=
// [timing_check_event_control] specify_terminal_descriptor [ &&& timing_check_condition ]
// controlled_timing_check_event ::=
// timing_check_event_control specify_terminal_descriptor [ &&& timing_check_condition ]
// timing_check_event_control ::=
// posedge
// | negedge
// | edge_control_specifier
// specify_terminal_descriptor ::=
// specify_input_terminal_descriptor
// | specify_output_terminal_descriptor
// edge_control_specifier ::= edge [ edge_descriptor { , edge_descriptor } ]
// edge_descriptor2 ::=
// 01
// | 10
// | z_or_x zero_or_one
// | zero_or_one z_or_x
// zero_or_one ::= 0 | 1
// z_or_x ::= x | X | z | Z
// timing_check_condition ::=
// scalar_timing_check_condition
// | ( scalar_timing_check_condition )
//
// ---
// scalar_timing_check_condition ::=
// expression
// | ~ expression
// | expression == scalar_constant
// | expression === scalar_constant
// | expression != scalar_constant
// | expression !== scalar_constant
// scalar_constant ::=
// 1'b0 | 1'b1 | 1'B0 | 1'B1 | 'b0 | 'b1 | 'B0 | 'B1 | 1 | 0

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 500
// specify_block ::= specify { specify_item } endspecify
#[derive(Clone, Copy)]
pub struct SpecifyBlock {
    pub items: AstIdRange<SpecifyBlockItem>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 500
// specify_item ::=
//   specparam_declaration
// | pulsestyle_declaration
// | showcancelled_declaration
// | path_declaration
// | system_timing_check
#[derive(Clone, Copy)]
pub enum SpecifyBlockItem {
    SpecParamDeclaration,
    PulseStyleDeclaration,
    ShowCancelledDeclaration,
    PathDeclaration(PathDeclaration),
    SystemTimingCheck(SystemTimingCheck),
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 500
// simple_path_declaration ::=
//   parallel_path_description = path_delay_value
// | full_path_description = path_delay_value
//
// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 501
// edge_sensitive_path_declaration ::=
//   parallel_edge_sensitive_path_description = path_delay_value
// | full_edge_sensitive_path_description = path_delay_value
// parallel_edge_sensitive_path_description ::=
//   ( [ edge_identifier ] specify_input_terminal_descriptor =>
//   ( specify_output_terminal_descriptor [ polarity_operator ] : data_source_expression ) )
// full_edge_sensitive_path_description ::=
//   ( [ edge_identifier ] list_of_path_inputs *>
//   ( list_of_path_outputs [ polarity_operator ] : data_source_expression ) )
// data_source_expression ::= expression
#[derive(Clone, Copy)]
pub struct PathDeclaration {
    pub state_dependent_condition: Option<AstId<StateDependentCondition>>,
    pub edge_identifier: Option<AstItem<EdgeIdentifier>>,
    pub input_terminal_descriptors: AstIdRange<TerminalDescriptor>,
    pub polarity_operator: Option<AstItem<PolarityOperator>>,
    pub simple_path_declaration_variant: SimplePathDeclarationVariant,
    pub data_source_expression: Option<AstId<Expr>>,
    pub output_terminal_descriptors: AstIdRange<TerminalDescriptor>,
    pub path_delay_value: AstId<PathDelayValue>,
}

impl PathDeclaration {
    pub fn is_simple(&self) -> bool {
        self.data_source_expression.is_none()
    }

    pub fn is_edge_sensitive(&self) -> bool {
        self.data_source_expression.is_some()
    }

    pub fn is_state_dependent(&self) -> bool {
        self.state_dependent_condition.is_some()
    }
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 501
// edge_identifier ::= posedge | negedge
#[derive(Clone, Copy)]
pub enum EdgeIdentifier {
    Posedge,
    Negedge,
}

#[derive(Clone, Copy)]
pub enum StateDependentCondition {
    If(AstId<ModulePathExpr>),
    Ifnone,
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct ModulePathExpr(pub Expr);

#[derive(Clone, Copy)]
pub enum SimplePathDeclarationVariant {
    Parallel,
    Full,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 501
// polarity_operator ::= + | -
#[derive(Clone, Copy)]
pub enum PolarityOperator {
    Plus,
    Minus,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 500
// specify_input_terminal_descriptor ::= input_identifier [ [ constant_range_expression ] ]
// specify_output_terminal_descriptor ::= output_identifier [ [ constant_range_expression ] ]
#[derive(Clone, Copy)]
pub struct TerminalDescriptor {
    pub ident: AstItem<Identifier>,
    pub constant_range_expr: Option<AstId<ConstantRangeExpression>>,
}

// IEEE Std 1364-2005 (Revision of IEEE Std 1364-2001) p. 500 - 501
// path_delay_value ::=
//   list_of_path_delay_expressions
// | ( list_of_path_delay_expressions )
#[derive(Clone, Copy)]
pub struct PathDelayValue {
    pub list_of_delay_expressions: AstIdRange<ConstantMinTypMaxExpression>,
}

// system_timing_check ::=
//   $setup_timing_check
// | $hold_timing_check
// | $setuphold_timing_check
// | $recovery_timing_check
// | $removal_timing_check
// | $recrem_timing_check
// | $skew_timing_check
// | $timeskew_timing_check
// | $fullskew_timing_check
// | $period_timing_check
// | $width_timing_check
// | $nochange_timing_check
#[derive(Clone, Copy)]
pub enum SystemTimingCheck {
    Setup,
    Hold,
    SetupHold,
    Recovery,
    Removal,
    Recrem,
    Skew,
    TimeSkew,
    FullSkew,
    Period,
    Width,
    NoChange,
}

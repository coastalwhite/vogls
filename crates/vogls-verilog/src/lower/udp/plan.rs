//! Planning the lowering of a sequential UDP instance.
//!
//! This does a brute-force for small tables to find more optimal ways of lower. This is especially
//! useful flipflops as they have quite a distinctive shape that can optimized for. Consequently, we
//! can go from processes that have roughly 120 optimized IR instructions to 4 optimized IR
//! instructions.
//!
//! Specifically, this forms a `Table` consisting of `Row`s. Then, for relatively small tables which
//! are the large majority, it will filter out rows that can be statically determined to be
//! unreachable. It determines what inputs can actually cause a change in the state to form a
//! reduced sensitivity list. Then, if the sensitivity list is only a single item. It can do
//! additional optimizations.

use std::ops::{ControlFlow, Range};
use std::rc::Rc;

use vogls_frontend::ident_table::IdentId;
use vogls_ir::LogicMode;
use vogls_utils::{BitsetU8, BitsetU32, VgHashMap};

use crate::ast::AstIdRange;
use crate::ast::udp::{UdpLevelSymbol, UdpNextState, UdpSequentialEntry};

use super::levels::{Level, Transition};

/// The entries of a sequential UDP, in the order the lowering matches them: the level-sensitive
/// entries first, as they win from the edge-sensitive entries (IEEE Std 1364-2005 8.7).
pub struct Table {
    pub rows: Vec<Row>,
    /// The transitions that match of all rows, one per input for every row. An input that did not
    /// change matches with the transition from its value to itself.
    pub transitions: Box<[Transition]>,
}

/// One entry of a [`Table`].
pub struct Row {
    /// Range in Table.transitions.
    pub transitions: Range<u32>,
    pub current: UdpLevelSymbol,
    /// `None` for no state change (`-`).
    pub next: Option<Level>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wake {
    Posedge,
    Negedge,
    Any,
}

/// The next state on a waking transition of the single live input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Body {
    /// Always this value.
    Constant(Level),

    /// The value of this input.
    Copy(usize),

    /// - `before`: the watched input's before the transition
    /// - `after`: the watched input's after the transition
    /// - `v`: the source input's value
    /// - `q`: the current state
    ///
    /// merge(v, q) = v       if v === q
    ///             = x       otherwise
    ///
    /// next = v              if before != x and after != x    (a clean 0 ↔ 1 edge)
    ///      = merge(v, q)    otherwise                        (an edge from or to x)
    CopyOrMerge(usize),
}

/// The lowering of an instance in which only one input can change the state.
#[derive(Debug, Clone, Copy)]
pub struct SingleInput {
    pub input: usize,

    /// The transitions that can change the state.
    pub wake: Wake,
    /// Does the next state depend on the value of the input before the transition?
    pub needs_before: bool,
    /// Optional simplification.
    pub body: Option<Body>,
}

pub struct UdpPlan {
    pub table: Table,

    /// The inputs of which a change on its own can change the state: the process only watches
    /// these. The other inputs can still be read, the process does not need to be sensitive to
    /// these.
    pub sensitivity: BitsetU32,
    pub single: Option<SingleInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UdpPlanKey {
    udp: IdentId,
    constants: Values,
    input_is_fv: BitsetU32,
    state_mode: LogicMode,
}

pub type UdpPlanCache = VgHashMap<UdpPlanKey, Rc<UdpPlan>>;

/// Above this number of inputs, the plan is no-longer analyzed.
const MAX_INPUTS: usize = 32;
/// Above this number of non-constant inputs, the plan is no-longer analyzed.
const MAX_ANALYZED_INPUTS: usize = 7;

/// The values of at most 32 inputs, with two bits per input `i` at bit `2 * i`.
///
/// Each value is a [`Level`] or `0b11` if it has no value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Values(u64);

/// The inputs and the state of an instance that is analyzed, which stay the same for every event.
#[derive(Clone, Copy)]
struct Domain {
    /// The constants on the inputs: the inputs that are not tied to a constant have no value.
    constants: Values,
    /// The inputs that are not tied to a constant.
    free: BitsetU32,
    /// The free inputs that are four-valued and can be `x`.
    input_is_fv: BitsetU32,
    state_mode: LogicMode,
}

impl Table {
    /// Returns `None` if an entry does not have a value for every input.
    pub fn new(entries: AstIdRange<'_, UdpSequentialEntry<'_>>, num_inputs: usize) -> Option<Self> {
        let mut rows = Vec::with_capacity(entries.len());
        let mut transitions = Vec::with_capacity(entries.len() * num_inputs);
        for allow_edge_list in [false, true] {
            for entry in entries.iter() {
                let UdpSequentialEntry {
                    level_list,
                    edge_list,
                    current_state,
                    next_state,
                } = &*entry;
                if edge_list.is_some() != allow_edge_list {
                    continue;
                }

                let start = transitions.len();
                transitions.extend(level_list.iter().map(|l| Transition::from_end_level(*l)));
                if let Some((edge_indicator, after_list)) = edge_list {
                    transitions.push(Transition::from(**edge_indicator));
                    transitions.extend(after_list.iter().map(|l| Transition::from_end_level(*l)));
                }
                if transitions.len() - start != num_inputs {
                    return None;
                }

                rows.push(Row {
                    transitions: u32::try_from(start).ok()?
                        ..u32::try_from(transitions.len()).ok()?,
                    current: current_state.item,
                    next: match next_state.item {
                        UdpNextState::Output(o) => Some(o.into()),
                        UdpNextState::Dash => None,
                    },
                });
            }
        }
        Some(Self {
            rows,
            transitions: transitions.into_boxed_slice(),
        })
    }

    pub fn transitions(&self, row: &Row) -> &[Transition] {
        &self.transitions[row.transitions.start as usize..row.transitions.end as usize]
    }

    pub fn edge_inputs(&self) -> impl Iterator<Item = usize> + '_ {
        self.rows.iter().filter_map(|row| {
            self.transitions(row)
                .iter()
                .position(|transition| transition.is_edge())
        })
    }

    fn retain(&mut self, mut f: impl FnMut(&[Transition]) -> bool) {
        let transitions = &self.transitions;
        self.rows.retain(|row| {
            f(&transitions[row.transitions.start as usize..row.transitions.end as usize])
        });
    }

    fn fold_constants(&mut self, constants: &[Option<Level>]) {
        self.retain(|transitions| {
            transitions
                .iter()
                .zip(constants)
                .all(|(transition, c)| c.is_none_or(|c| transition.contains(Transition::new(c, c))))
        });
    }
}

/// The next state after the inputs went from `before` to `after` with a given current `state`.
fn evaluate(
    table: &Table,
    before: Values,
    after: Values,
    state: Level,
    state_mode: LogicMode,
) -> Level {
    'rows: for row in &table.rows {
        if !state.matches(row.current) {
            continue;
        }

        for (i, transition) in table.transitions(row).iter().enumerate() {
            let before = before.get(i);
            let after = after.get(i);
            if !transition.contains(Transition::new(before, after)) {
                continue 'rows;
            }
        }

        return row.next.unwrap_or(state).to_mode(state_mode);
    }

    Level::X.to_mode(state_mode)
}

impl UdpPlan {
    fn compute(
        mut table: Table,
        constants: &[Option<Level>],
        input_is_fv: BitsetU32,
        state_mode: LogicMode,
    ) -> Self {
        let domain = Domain::new(constants, input_is_fv, state_mode);
        let free = domain.free;
        debug_assert!(free.num_set() as usize <= MAX_ANALYZED_INPUTS);

        table.fold_constants(constants);

        let sensitivity: BitsetU32 = free
            .iter()
            .filter(|&i| {
                let mut changes_state = false;
                domain.for_each_event(i, |before, after, state| {
                    let next = evaluate(&table, after.with(i, before), after, state, state_mode);
                    if next == state {
                        return ControlFlow::Continue(());
                    }
                    changes_state = true;
                    ControlFlow::Break(())
                });
                changes_state
            })
            .collect();

        // Edges of inert inputs never need to match: drop them so their previous value is not kept.
        table.retain(|transitions| {
            transitions
                .iter()
                .enumerate()
                .all(|(i, transition)| sensitivity.get(i) || transition.is_level())
        });

        // If the UDP is only sensitive to a change on a specific input, we can do some additional
        // optimization.
        let Some(input) = sensitivity.into_bit() else {
            return Self {
                table,
                sensitivity,
                single: None,
            };
        };

        // Calculate whether a positive / negative edge can lead to a new state value.
        let mut posedge_only = true;
        let mut negedge_only = true;
        domain.for_each_event(input, |before, after, state| {
            let next = evaluate(&table, after.with(input, before), after, state, state_mode);
            if next != state {
                let transition = Transition::new(before, after.get(input));
                posedge_only &= Transition::POSEDGE.contains(transition);
                negedge_only &= Transition::NEGEDGE.contains(transition);
            }
            if posedge_only | negedge_only {
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(())
            }
        });
        let wake = match (posedge_only, negedge_only) {
            (true, _) => Wake::Posedge, // Only a positive edge lead to a new state value
            (_, true) => Wake::Negedge, // Only a negative edge lead to a new state value
            _ => Wake::Any,             // Transitions in either direction lead to a new state value
        };

        // Are there different next state values per after value?
        //
        // @NOTE: We cannot easily have this outside the single input case, because you need to
        // differentiate between:
        // 1. Did this input change?
        // 2. Does this input now have a certain value?
        // With multiple inputs and without access to the before, you can answer question 2, but not
        // question 1. In the single input case, question 1 is trivially answered because the
        // process was woken.
        let wake_edges = match wake {
            Wake::Posedge => Transition::POSEDGE,
            Wake::Negedge => Transition::NEGEDGE,
            Wake::Any => Transition::QM_TO_QM,
        };
        let mut needs_before = false;
        // 2-bits per value (X, 0, 1). If 0b11 if not yet inhabited.
        let mut per_after_value = 0b11_11_11u8;
        let mut last_after = None;

        // The bodies that give the next state on every waking transition, as the constants, and
        // the inputs to copy or to copy or merge. The state is driven with the value of the body,
        // which a two-valued state stores with `x` as `0`, like `evaluate` does.
        let state_values = Level::values(state_mode);
        let mut constants: BitsetU8 = state_values.iter().map(|&c| c as usize).collect();
        let mut copies = free;
        let mut merges = free;

        domain.for_each_event(input, |before, after, state| {
            // Start again per after value.
            if last_after != Some(after) {
                last_after = Some(after);
                per_after_value = 0b11_11_11;
            }

            if !wake_edges.contains(Transition::new(before, after.get(input))) {
                return ControlFlow::Continue(());
            }

            let next = evaluate(&table, after.with(input, before), after, state, state_mode);

            let shamt = 2 * state as u8;
            let first = per_after_value >> shamt & 0b11;
            if first == 0b11 {
                per_after_value = per_after_value & !(0b11 << shamt) | (next as u8) << shamt;
            } else if first != next as u8 {
                needs_before = true;
            }

            for c in constants.iter() {
                if Level::ALL[c] != next {
                    constants.clear(c);
                }
            }
            for j in copies.iter() {
                if after.get(j).to_mode(state_mode) != next {
                    copies.clear(j);
                }
            }
            // The value of `j` if the transition is between `0` and `1`, otherwise the state if it
            // equals that value, and `x` if it does not.
            let exact = before != Level::X && after.get(input) != Level::X;
            for j in merges.iter() {
                let value = after.get(j);
                let merged = if exact || value == state {
                    value
                } else {
                    Level::X
                };
                if merged.to_mode(state_mode) != next {
                    merges.clear(j);
                }
            }

            // Neither can change anymore.
            if needs_before && constants.is_empty() && copies.is_empty() && merges.is_empty() {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        });

        let body = if let Some(&c) = state_values.iter().find(|&&c| constants.get(c as usize)) {
            Some(Body::Constant(c))
        } else if let Some(j) = copies.first() {
            Some(Body::Copy(j))
        } else {
            merges.first().map(Body::CopyOrMerge)
        };

        Self {
            table,
            sensitivity,
            single: Some(SingleInput {
                input,
                wake,
                needs_before,
                body,
            }),
        }
    }
}

/// Compute a UDP plan or get a cached plan.
///
/// If a UDP table is too large, it is returned bare.
pub fn compute_or_get_cache_plan(
    cache: &mut UdpPlanCache,
    udp: IdentId,
    table: impl FnOnce() -> Table,
    constants: &[Option<Level>],
    input_mode: impl Fn(usize) -> LogicMode,
    state_mode: LogicMode,
) -> Result<Rc<UdpPlan>, Table> {
    let num_free = constants.iter().filter(|c| c.is_none()).count();
    if constants.len() > MAX_INPUTS || num_free > MAX_ANALYZED_INPUTS {
        let mut table = table();
        table.fold_constants(constants);
        return Err(table);
    }
    let input_is_fv: BitsetU32 = (0..constants.len())
        .filter(|&i| constants[i].is_none() && input_mode(i).is_four_value())
        .collect();
    let key = UdpPlanKey {
        udp,
        constants: Values::new(constants),
        input_is_fv,
        state_mode,
    };
    Ok(cache
        .entry(key)
        .or_insert_with(|| {
            Rc::new(UdpPlan::compute(
                table(),
                constants,
                input_is_fv,
                state_mode,
            ))
        })
        .clone())
}

impl Values {
    fn new(constants: &[Option<Level>]) -> Self {
        debug_assert!(constants.len() <= 32);
        Self(
            constants
                .iter()
                .enumerate()
                .fold(0, |values, (i, constant)| {
                    let bits = constant.map_or(0b11, |level| level as u64);
                    values | bits << (2 * i)
                }),
        )
    }

    fn get(self, input: usize) -> Level {
        match self.0 >> (2 * input) & 0b11 {
            0 => Level::X,
            1 => Level::L0,
            bits => {
                debug_assert_eq!(bits, Level::L1 as u64, "input {input} has no value");
                Level::L1
            }
        }
    }

    fn with(self, input: usize, value: Level) -> Self {
        Self(self.0 & !(0b11 << (2 * input)) | (value as u64) << (2 * input))
    }
}

impl Domain {
    fn new(constants: &[Option<Level>], input_is_fv: BitsetU32, state_mode: LogicMode) -> Self {
        debug_assert!(constants.len() <= MAX_INPUTS);
        Self {
            constants: Values::new(constants),
            free: (0..constants.len())
                .filter(|&i| constants[i].is_none())
                .collect(),
            input_is_fv,
            state_mode,
        }
    }

    fn input_mode(self, input: usize) -> LogicMode {
        if self.input_is_fv.get(input) {
            LogicMode::FourValue
        } else {
            LogicMode::TwoValue
        }
    }

    /// Calls `f(before, after, state)` for every transition of `input` and every value of the other
    /// inputs and of the state, until `f` breaks. The events with the same `after` come one after
    /// another.
    fn for_each_event(
        self,
        input: usize,
        mut f: impl FnMut(Level, Values, Level) -> ControlFlow<()>,
    ) {
        let input_values = Level::values(self.input_mode(input));
        let state_values = Level::values(self.state_mode);
        // Each free input counts from its first value up to `1`: from `x` if it can be `x`, and
        // from `0` otherwise.
        let first = |i: usize| match self.input_mode(i) {
            LogicMode::TwoValue => Level::L0 as u64,
            LogicMode::FourValue => Level::X as u64,
        };
        let last = Level::L1 as u64;
        let free = self.free;

        // Every assignment of values to the inputs, with the constants in place, the first free
        // input counting fastest.
        let mut after = self.constants.0;
        for i in free.iter() {
            after = after & !(0b11 << (2 * i)) | first(i) << (2 * i);
        }
        loop {
            // The value of `input` is iterated over as part of the assignment.
            for &before in input_values {
                if before == Values(after).get(input) {
                    continue;
                }
                for &state in state_values {
                    if f(before, Values(after), state).is_break() {
                        return;
                    }
                }
            }

            let mut rest = free.iter();
            loop {
                let Some(i) = rest.next() else {
                    return;
                };
                let shift = 2 * i;
                if after >> shift & 0b11 < last {
                    after += 1 << shift;
                    break;
                }
                after = after & !(0b11 << shift) | first(i) << shift;
            }
        }
    }
}

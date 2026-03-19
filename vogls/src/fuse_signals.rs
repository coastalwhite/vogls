use slotmap::SlotMap;
use vogls_ir::token_range::TokenRange;
use vogls_ir::{
    BasicBlockBuilder, Bits, GlobalContext, INTEGER_VSIZE, Signal, SignalKey, SignalSlice,
    new_process,
};
use vogls_utils::VgHashMap;
use vogls_verilog::lower::Edge;

slotmap::new_key_type! { struct SignalEquivClassKey; }

pub struct SignalEquivClass {
    signals: Vec<SignalKey>,
}
type EquivClasses = SlotMap<SignalEquivClassKey, SignalEquivClass>;

fn print(signals: &SlotMap<SignalKey, Signal>, eqclasses: &EquivClasses) {
    for (i, eqclass) in eqclasses.values().enumerate() {
        println!("Equivalence Class {i}:");
        for signal in &eqclass.signals {
            println!(" - {}", &signals[*signal].name);
        }
    }
}

pub fn fuse_signals(
    gl: &mut GlobalContext,
    connections: &[Edge],
) -> VgHashMap<SignalKey, (SignalKey, Option<SignalSlice>)> {
    let mut eqclasses = EquivClasses::default();
    let mut lookup = VgHashMap::<SignalKey, SignalEquivClassKey>::default();
    for edge in connections.iter() {
        let from_ec = lookup.get(&edge.from).copied();
        let to_ec = lookup.get(&edge.to).copied();

        match (from_ec, to_ec) {
            (None, None) => {
                let key = eqclasses.insert(SignalEquivClass {
                    signals: vec![edge.from, edge.to],
                });
                lookup.insert(edge.from, key);
                lookup.insert(edge.to, key);
            }
            (Some(eqclass), None) => {
                eqclasses[eqclass].signals.push(edge.to);
                lookup.insert(edge.to, eqclass);
            }
            (None, Some(eqclass)) => {
                eqclasses[eqclass].signals.push(edge.from);
                lookup.insert(edge.from, eqclass);
            }
            (Some(from_ec), Some(to_ec)) => {
                let to_ec = eqclasses.remove(to_ec).unwrap();
                for signal in &to_ec.signals {
                    *lookup.get_mut(signal).unwrap() = from_ec;
                }
                eqclasses[from_ec].signals.extend(to_ec.signals);
            }
        }
    }

    print(&gl.signals, &eqclasses);

    let mut replacement_signals =
        VgHashMap::<SignalKey, (SignalKey, Option<SignalSlice>)>::default();
    replacement_signals.reserve(lookup.len() - eqclasses.len());
    for (_, eqclass) in eqclasses.into_iter() {
        let eqclass_to_signal = eqclass.signals[0];
        replacement_signals.extend(
            eqclass
                .signals
                .into_iter()
                .skip(1)
                .map(|k| (k, (eqclass_to_signal, None))),
        );
    }

    // TODO: This is stupid.
    let keys = gl.bbs.keys().collect::<Vec<_>>();
    for key in keys {
        let mut builder = BasicBlockBuilder::continue_from(Vec::new(), key);
        use vogls_ir::Instruction as I;
        for i in std::mem::take(&mut gl.bbs[key].instrs) {
            match &i {
                I::Probe(dst, signal) => {
                    if let Some((to, slice)) = replacement_signals.get(signal) {
                        match slice {
                            None => builder.push_raw_instruction(I::Probe(*dst, *to)),
                            Some(slice) => {
                                builder.probe_slice_constant(gl, *to, slice.lsb(), slice.width());
                                builder
                                    .instrs
                                    .last_mut()
                                    .unwrap()
                                    .get_destination_variable_mut()
                                    .map(|d| *d = *dst);
                            }
                        }
                        continue;
                    }
                }
                I::Drive(signal, src, partial) => {
                    if let Some((to, slice)) = replacement_signals.get(signal) {
                        match (*partial, slice) {
                            (partial, None) => builder.drive_opt_partial(gl, *to, *src, partial),
                            (None, Some(slice)) => builder.drive_partial_constant(
                                gl,
                                *to,
                                *src,
                                slice.lsb(),
                                slice.width(),
                            ),
                            (Some((offset, width)), Some(slice)) => {
                                let offset = builder.plus_constant(
                                    gl,
                                    offset,
                                    Bits::from_u64(INTEGER_VSIZE, slice.lsb() as u64),
                                );
                                builder.drive_partial(gl, *to, *src, offset, width);
                            }
                        }

                        continue;
                    }
                }

                // @TODO: VCD
                I::Intrinsic(_, _, _) => {}
                _ => {}
            }
            builder.push_raw_instruction(i);
        }
        gl.bbs[key].instrs = builder.into_instructions();
        gl.bbs[key]
            .terminator
            .map_signal(|s| replacement_signals.get(&s).map_or(s, |(s, _)| *s));
    }

    replacement_signals
}


use hashbrown::hash_map::Entry;
use vogls_fuse_signals::InputEdge;
use vogls_ir::bits::arithmetic::FvLogicValue;
use vogls_ir::token_range::TokenRange;
use vogls_ir::{
    Bits, GlobalContext, LogicMode, ProcessBuilder, ProcessKind, Signal, SignalFlags, SignalKey,
    SignalSlice, VectorSize, WatchCondition,
};
use vogls_utils::{IterSliceContinguous, VgHashMap};

#[derive(Clone)]
struct MultiDriver {
    drivee: SignalKey,
    proxy: SignalKey,
}

#[derive(Default, Clone)]
pub struct MultidriverResolution {
    driven: VgHashMap<SignalKey, Mask>,
    multidrivers: Vec<MultiDriver>,
}

#[must_use]
pub enum MultiDriverResult {
    Keep,
    NeedsRemap(SignalKey),
}

#[derive(Clone)]
pub enum Mask {
    Full,
    Partial(Bits),
}

fn signal_proxy(
    gl: &mut GlobalContext,
    name: String,
    bitlength: VectorSize,
    mode: LogicMode,
) -> SignalKey {
    gl.signals.insert(Signal {
        name,
        size: bitlength,
        initialize: match mode {
            LogicMode::TwoValue => None,
            LogicMode::FourValue => Some(Bits::new_high_impedance(bitlength)),
        },
        mode,
        flags: SignalFlags::EMPTY,
        origin: TokenRange::default(),
    })
}

impl MultidriverResolution {
    pub fn push_new_driver(
        &mut self,
        gl: &mut GlobalContext,
        signal: SignalKey,
        slice: SignalSlice,
    ) -> MultiDriverResult {
        let s = &gl.signals[signal];
        let entire_signal_slice = SignalSlice::with_end(s.size);
        let Some(drive_slice) = entire_signal_slice.overlapping_slice(slice) else {
            // The drive is entirely outside the signal range. Nothing has to be written.
            return MultiDriverResult::Keep;
        };

        let mask = match self.driven.entry(signal) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let mask = if drive_slice == entire_signal_slice {
                    Mask::Full
                } else {
                    let mut bits = Bits::new_zeroed(s.size);
                    bits.fill_range(drive_slice.to_range_inclusive(), FvLogicValue::L1);
                    Mask::Partial(bits)
                };
                entry.insert(mask);
                return MultiDriverResult::Keep;
            }
        };

        let has_driver = match mask {
            Mask::Full => true,
            Mask::Partial(mask) => {
                let range = drive_slice.to_range_inclusive();
                let has_driver = mask.range_reduce_or(range.clone());
                mask.fill_range(range, FvLogicValue::L1);
                has_driver == FvLogicValue::L1
            }
        };

        if has_driver {
            let proxy = signal_proxy(gl, format!("::PROXY({})", s.name), s.size, s.mode);
            self.multidrivers.push(MultiDriver {
                drivee: signal,
                proxy,
            });
            return MultiDriverResult::NeedsRemap(proxy);
        }

        MultiDriverResult::Keep
    }

    pub fn finalize(mut self, gl: &mut GlobalContext, connections: &mut Vec<InputEdge>) {
        if self.multidrivers.is_empty() {
            return;
        }

        drop(self.driven);

        let mut remap = VgHashMap::default();
        self.multidrivers.sort_unstable_by_key(|d| d.drivee);

        for segment in IterSliceContinguous::new(&self.multidrivers, |d| &d.drivee) {
            let drivee = segment[0].drivee;
            let s = &gl.signals[drivee];
            let fst_proxy = signal_proxy(gl, format!("::PROXY({})", s.name), s.size, s.mode);
            remap.insert(drivee, fst_proxy);
        }

        for bb in gl.bbs.values_mut() {
            bb.instrs.iter_mut().for_each(|instr| {
                instr.map_drive_signals(|s| remap.get(&s).copied().unwrap_or(s));
            });
        }
        for connection in connections {
            connection.drivee = remap
                .get(&connection.drivee)
                .copied()
                .unwrap_or(connection.drivee);
        }

        for segment in IterSliceContinguous::new(&self.multidrivers, |d| &d.drivee) {
            let drivee = segment[0].drivee;
            let fst_proxy = remap[&drivee];

            let (proc_builder, mut bb_builder) =
                ProcessBuilder::new(gl, ProcessKind::Fuse, TokenRange::default());
            let entry = proc_builder.entry();
            let mut watchers = Vec::with_capacity(segment.len() + 1);

            let mut acc = bb_builder.probe(gl, fst_proxy);
            watchers.push(WatchCondition::entire_signal(gl, fst_proxy));
            for d in segment {
                let proxy_v = bb_builder.probe(gl, d.proxy);
                acc = bb_builder.resolve_wire(gl, acc, proxy_v);
                watchers.push(WatchCondition::entire_signal(gl, d.proxy));
            }

            bb_builder.drive(gl, drivee, acc);
            bb_builder.watch_conditions_to(gl, watchers, entry);
            proc_builder.finalize(gl);
        }
    }

    pub fn push_connections(&mut self, gl: &mut GlobalContext, connections: &mut [InputEdge]) {
        for connection in connections {
            let bitlength = gl.signals[connection.drivee].size;
            match self.push_new_driver(
                gl,
                connection.drivee,
                connection
                    .drivee_slice
                    .unwrap_or(SignalSlice::with_end(bitlength)),
            ) {
                MultiDriverResult::Keep => {}
                MultiDriverResult::NeedsRemap(proxy) => connection.drivee = proxy,
            }
        }
    }
}

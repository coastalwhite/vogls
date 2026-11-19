//! Rewriting of all references to aliased signals.

use hashbrown::hash_map::Entry;
use vogls_ir::vcd::VcdValue;
use vogls_ir::{
    BasicBlockBuilder, BasicBlockTerminator, Bits, GlobalContext, IntrinsicOp, WatchCondition,
};

use vogls_ir::SignalKey;

use super::{FuseTarget, ProbeFuseMap, WriteFuseMap};

/// Rewrites all probes, drives, watches and VCD references in `gl` according to the fuse maps.
///
/// - `fused_signals` maps a signal that is no longer needed to the signal (slice) or constant that
///   should be probed instead.
/// - `write_map` maps a signal to the signal slice that should be written instead. Only the
///   entries of variables, which `is_variable` tells, apply to the drives in the IR (see
///   [`WriteFuseMap`]).
pub(super) fn apply_fuse_maps(
    gl: &mut GlobalContext,
    fused_signals: &ProbeFuseMap,
    write_map: &WriteFuseMap,
    is_variable: impl Fn(SignalKey) -> bool,
) {
    let drive_map = |signal: &SignalKey| write_map.get(signal).filter(|_| is_variable(*signal));
    // TODO: This is stupid.
    let keys = gl.bbs.keys().collect::<Vec<_>>();
    for key in keys {
        let mut builder = BasicBlockBuilder::continue_from(Vec::new(), gl.bbs[key].region, key);
        use vogls_ir::Instruction as I;
        for i in std::mem::take(&mut gl.bbs[key].instrs) {
            match &i {
                I::Probe(dst, signal, offset) => {
                    if let Some(tgt) = fused_signals.get(signal) {
                        match tgt {
                            FuseTarget::Signal(to, slice) => match slice {
                                None => builder.push_raw_instruction(I::Probe(*dst, *to, *offset)),
                                Some(slice) => {
                                    builder.probe_slice_constant(
                                        gl,
                                        *to,
                                        offset + slice.lsb(),
                                        slice.width(),
                                    );
                                    if let Some(d) = builder
                                        .instrs
                                        .last_mut()
                                        .unwrap()
                                        .get_destination_variable_mut()
                                    {
                                        *d = *dst;
                                    }
                                }
                            },
                            FuseTarget::Constant(value) => {
                                let mut value = value.slicex(*offset, gl.vars.size(*dst));
                                // Lower mode when needed.
                                if dst.mode().is_two_value() {
                                    value = value.special_to_zero();
                                }
                                builder.push_raw_instruction(I::Constant(*dst, value));
                            }
                        }
                        continue;
                    }
                }
                I::ProbeSlice(dst, signal, offset) => {
                    if let Some(tgt) = fused_signals.get(signal) {
                        match tgt {
                            FuseTarget::Signal(to, slice) => match slice {
                                None => {
                                    builder.push_raw_instruction(I::ProbeSlice(*dst, *to, *offset))
                                }
                                Some(slice) => {
                                    let offset = builder.plus_constant(
                                        gl,
                                        *offset,
                                        Bits::new_u32(slice.lsb()),
                                    );
                                    builder.probe_slice(gl, *to, offset, slice.width());
                                    if let Some(d) = builder
                                        .instrs
                                        .last_mut()
                                        .unwrap()
                                        .get_destination_variable_mut()
                                    {
                                        *d = *dst;
                                    }
                                }
                            },
                            FuseTarget::Constant(value) => {
                                builder.rev_imm_slice_x(
                                    gl,
                                    value.clone(),
                                    *offset,
                                    gl.vars.size(*dst),
                                );
                                if let Some(d) = builder
                                    .instrs
                                    .last_mut()
                                    .unwrap()
                                    .get_destination_variable_mut()
                                {
                                    *d = *dst;
                                }
                            }
                        }
                        continue;
                    }
                }
                I::Drive(dst, signal, src, offset) => {
                    if let Some((to, slice)) = drive_map(signal) {
                        let offset = offset + slice.lsb();
                        builder.push_raw_instruction(I::Drive(*dst, *to, *src, offset));
                        continue;
                    }
                }
                I::DriveSlice(dst, signal, src, partial) => {
                    if let Some((to, slice)) = drive_map(signal) {
                        let offset =
                            builder.plus_constant(gl, *partial, Bits::new_u32(slice.lsb()));
                        builder.push_raw_instruction(I::DriveSlice(*dst, *to, *src, offset));
                        continue;
                    }
                }
                I::LastUpdateTime(dst, signal) => {
                    if let Some(tgt) = fused_signals.get(signal) {
                        match tgt {
                            FuseTarget::Signal(signal, slice) => {
                                // Fusing is only allowed when there is a LastUpdateTime
                                // instruction if the whole signal is fused.
                                assert!(slice.is_none());
                                builder.push_raw_instruction(I::LastUpdateTime(*dst, *signal));
                            }
                            FuseTarget::Constant(_) => {
                                // Constant signals are only assigned at the start.
                                builder.push_raw_instruction(I::Constant(*dst, Bits::new_u64(0)));
                            }
                        }
                        continue;
                    }
                }
                I::Intrinsic(dst, op, srcs) => match op.as_ref() {
                    IntrinsicOp::ReadMem(readmem) => {
                        // The memory of a variable that is inverted into a net is a slice of the
                        // net.
                        if let Some((to, slice)) = drive_map(&readmem.signal) {
                            let mut readmem = readmem.as_ref().clone();
                            readmem.signal = *to;
                            readmem.offset += slice.lsb();
                            builder.push_raw_instruction(I::Intrinsic(
                                *dst,
                                Box::new(IntrinsicOp::ReadMem(Box::new(readmem))),
                                srcs.clone(),
                            ));
                            continue;
                        }
                        // A `$readmem` target is a variable (the frontend rejects nets), and a
                        // variable is never aliased away.
                        assert!(
                            !fused_signals.contains_key(&readmem.signal),
                            "a `$readmem` target was aliased"
                        );
                    }
                    IntrinsicOp::VcdAppendModule(module) => {
                        let mut module = module.clone();
                        for vcd_var in module.table.values_mut() {
                            let VcdValue::Signal(vcd_signal, vcd_signal_slice) = &vcd_var.value
                            else {
                                continue;
                            };
                            if let Some(tgt) = fused_signals.get(vcd_signal) {
                                match tgt {
                                    FuseTarget::Signal(to, slice) => {
                                        vcd_var.value = VcdValue::Signal(
                                            *to,
                                            match (*vcd_signal_slice, *slice) {
                                                (None, None) => None,
                                                (Some(s), None) | (None, Some(s)) => Some(s),
                                                (Some(vs), Some(os)) => {
                                                    Some(os.subslice(vs).unwrap())
                                                }
                                            },
                                        );
                                    }
                                    FuseTarget::Constant(bits) => {
                                        vcd_var.value = VcdValue::Constant(bits.clone());
                                    }
                                }
                            }
                        }
                        for (s, tgt) in fused_signals.iter() {
                            let Some(items) = module.signal_map.remove(s) else {
                                continue;
                            };

                            match tgt {
                                FuseTarget::Signal(to, _) => match module.signal_map.entry(*to) {
                                    Entry::Occupied(mut e) => e.get_mut().extend(items),
                                    Entry::Vacant(e) => _ = e.insert(items),
                                },
                                FuseTarget::Constant(_) => {}
                            }
                        }
                        builder.push_raw_instruction(I::Intrinsic(
                            *dst,
                            Box::new(IntrinsicOp::VcdAppendModule(module)),
                            srcs.clone(),
                        ));
                        continue;
                    }
                    _ => {}
                },

                _ => {}
            }
            builder.push_raw_instruction(i);
        }
        gl.bbs[key].instrs = builder.into_instructions();
        if let BasicBlockTerminator::Watch(_, conditions) = &mut gl.bbs[key].terminator {
            conditions.retain_mut(|c| match fused_signals.get(&c.signal) {
                None => true,
                Some(FuseTarget::Signal(to, part_select)) => {
                    *c = c.fused_into(*to, *part_select);
                    true
                }
                Some(FuseTarget::Constant(_)) => false,
            });
            WatchCondition::canonicalize(conditions);

            if conditions.is_empty() {
                gl.bbs[key].terminator = BasicBlockTerminator::Halt;
            }
        }
    }

    for (_, process) in gl.processes.iter_mut() {
        let Some(standing) = process.standing.as_mut() else {
            continue;
        };

        let mut standing_vec = standing.to_vec();
        standing_vec.retain_mut(|c| match fused_signals.get(&c.signal) {
            None => true,
            Some(FuseTarget::Signal(to, part_select)) => {
                *c = c.fused_into(*to, *part_select);
                true
            }
            Some(FuseTarget::Constant(_)) => false,
        });
        WatchCondition::canonicalize(&mut standing_vec);
        *standing = standing_vec.into_boxed_slice();
    }
}

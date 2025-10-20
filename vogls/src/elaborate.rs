use std::collections::{HashMap, HashSet};

use vogls_ir::{
    BasicBlock, BasicBlockTerminator, GlobalContext, Instruction, Section, SectionKey,
    SectionVariant, Signal, SignalKey,
};

pub fn elaborate(
    top_level_entity: SectionKey,
    gl: &mut GlobalContext,
    processes: &mut Vec<SectionKey>,
) {
    let mut entity_stack = Vec::new();
    entity_stack.push((0, top_level_entity));
    let mut next_entity_idx = 1u64;

    let mut entity_ids = Vec::new();

    // Recursively Instantiate the entities and processes in
    let mut signals_map = HashMap::<(u64, SignalKey), SignalKey>::new();
    while let Some((entity_key, entity_section_key)) = entity_stack.pop() {
        let entity = gl.sections.get(entity_section_key).unwrap();
        assert_eq!(entity.variant, SectionVariant::Entity);
        let bb = gl.bbs.get(entity.entry).unwrap();
        if !matches!(bb.terminator, BasicBlockTerminator::Halt) {
            todo!("evaluation with vogls-sim");
        }

        for instr in &bb.instrs {
            match instr {
                Instruction::Signal(signal_key) => {
                    signals_map
                        .entry((entity_key, *signal_key))
                        .or_insert_with(|| {
                            let signal = &gl.signals[*signal_key];
                            gl.signals.insert(Signal {
                                name: format!("{}-{entity_key}", signal.name),
                                ty: signal.ty.clone(),
                            })
                        });
                }
                Instruction::Instantiate(section_key, ports) => {
                    let section = gl.sections.get(*section_key).unwrap();

                    assert_eq!(section.ins.len() + section.outs.len(), ports.len());

                    let target_entity_idx = match section.variant {
                        SectionVariant::Entity => {
                            let target = next_entity_idx;
                            next_entity_idx += 1;
                            target
                        }
                        _ => entity_key,
                    };

                    for (entity_port, inst_port) in
                        (section.ins.iter().chain(section.outs.iter())).zip(ports)
                    {
                        signals_map.insert(
                            (target_entity_idx, *entity_port),
                            signals_map.get(&(entity_key, *inst_port)).unwrap().clone(),
                        );
                    }

                    match section.variant {
                        SectionVariant::Entity => {
                            entity_stack.push((target_entity_idx, *section_key))
                        }
                        SectionVariant::Function => todo!(),
                        SectionVariant::Process => {
                            entity_ids.push(target_entity_idx);
                            processes.push(*section_key);
                        }
                    }
                }
                _ => todo!("evaluation with vogls-sim"),
            }
        }
    }

    for (entity_id, process_key) in entity_ids.into_iter().zip(processes.iter_mut()) {
        let process = elaborate_process(*process_key, gl, entity_id, &signals_map);
        *process_key = gl.sections.insert(process);
    }
}

pub fn elaborate_process(
    section_key: SectionKey,
    gl: &mut GlobalContext,
    entity_id: u64,
    signals_map: &HashMap<(u64, SignalKey), SignalKey>,
) -> Section {
    let section = gl.sections.get(section_key).unwrap();
    assert_eq!(section.variant, SectionVariant::Process);

    let mut bb_stack = Vec::new();
    let mut bb_map = HashMap::new();

    bb_stack.clear();
    bb_map.clear();

    // Lower the IR instructions to VM instructions.
    bb_stack.push(section.entry);
    while let Some(bb_key) = bb_stack.pop() {
        let bb = gl.bbs.get(bb_key).unwrap();

        use Instruction as I;
        let instrs = bb
            .instrs
            .iter()
            .map(|instr| match instr {
                I::Probe(dst, signal) => {
                    let src = signals_map.get(&(entity_id, *signal)).unwrap();
                    I::Probe(*dst, *src)
                }
                I::Drive(signal, src) => {
                    let dst = signals_map.get(&(entity_id, *signal)).unwrap();
                    I::Drive(*dst, *src)
                }
                I::Instantiate(_, _) | I::Signal(_) => unreachable!(),
                instr => instr.clone(),
            })
            .collect();

        use BasicBlockTerminator as T;
        let terminator = match &bb.terminator {
            t @ (T::Wait(bb, _) | T::Jump(bb)) => {
                if !bb_map.contains_key(bb) {
                    bb_stack.push(*bb);
                }
                t.clone()
            }
            T::Watch(bb, signals) => {
                let signals = signals
                    .iter()
                    .map(|s| signals_map[&(entity_id, *s)])
                    .collect();
                if !bb_map.contains_key(bb) {
                    bb_stack.push(*bb);
                }
                T::Watch(*bb, signals)
            }
            t @ T::Branch(_, true_bb, false_bb) => {
                if !bb_map.contains_key(true_bb) {
                    bb_stack.push(*true_bb);
                }
                if !bb_map.contains_key(false_bb) {
                    bb_stack.push(*false_bb);
                }
                t.clone()
            }
            T::Halt => T::Halt,
        };

        let new_bb = BasicBlock {
            name: bb.name.clone(),
            instrs,
            terminator,
        };

        bb_map.insert(bb_key, gl.bbs.insert(new_bb));
    }

    let mut bb_seen = HashSet::with_capacity(bb_map.len());
    bb_stack.push(bb_map[&section.entry]);
    while let Some(bb_key) = bb_stack.pop() {
        let bb = gl.bbs.get_mut(bb_key).unwrap();

        use BasicBlockTerminator as T;
        match &mut bb.terminator {
            T::Wait(bb, _) | T::Jump(bb) | T::Watch(bb, _) => {
                *bb = bb_map[bb];
                if !bb_seen.contains(bb) {
                    bb_stack.push(*bb);
                }
            }
            T::Branch(_, true_bb, false_bb) => {
                *true_bb = bb_map[true_bb];
                *false_bb = bb_map[false_bb];
                if !bb_seen.contains(true_bb) {
                    bb_stack.push(*true_bb);
                }
                if !bb_seen.contains(false_bb) {
                    bb_stack.push(*false_bb);
                }
            }
            T::Halt => {}
        }

        bb_seen.insert(bb_key);
    }

    Section {
        variant: SectionVariant::Process,
        name: format!("{}-{entity_id}", section.name),
        entry: *bb_map.get(&section.entry).unwrap(),
        ins: section
            .ins
            .iter()
            .map(|v| signals_map[&(entity_id, *v)])
            .collect(),
        outs: section
            .outs
            .iter()
            .map(|v| signals_map[&(entity_id, *v)])
            .collect(),
    }
}

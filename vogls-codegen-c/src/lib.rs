use std::collections::HashMap;
use std::io;

use vogls_bits::format::{BitsFormatBase, BitsFormatOptions, BitsFormatWidth};
use vogls_codegen::{
    HeapBuilder, HeapOffset, HeapRef, resolve_heap_map, resolve_var_logic_mode_map,
};
use vogls_ir::{
    BasicBlockKey, BasicBlockTerminator, GlobalContext, Instruction, LogicMode, ProcessKey,
    SignalKey, UnaryOp, VariableKey, VectorSize,
};
use vogls_utils::{IndexMap, IndexSet, VgHashMap, VgHashSet};

const INDENT: &str = "    ";

pub fn lower_process(
    f: &mut impl io::Write,
    process: ProcessKey,
    process_idx: usize,
    gl: &GlobalContext,
    heap_builder: &mut HeapBuilder,
    io_signals: &HashMap<SignalKey, HeapRef>,
) -> io::Result<()> {
    use Instruction as I;

    let process = &gl.processes[process];

    let mut bb_stack = Vec::new();
    let mut bb_seen = VgHashSet::<BasicBlockKey>::default();
    let mut bb_phis = VgHashMap::<BasicBlockKey, Vec<(VariableKey, VariableKey)>>::default();

    let mut var_mode = VgHashMap::<VariableKey, LogicMode>::default();
    let mut conv_map = VgHashMap::<VariableKey, HeapOffset>::default();
    let mut heap_map = VgHashMap::default();

    resolve_var_logic_mode_map(
        process.entry,
        gl,
        &mut bb_stack,
        &mut bb_seen,
        &mut var_mode,
        &mut conv_map,
    );
    resolve_heap_map(
        process.entry,
        gl,
        &mut bb_stack,
        &mut bb_seen,
        &var_mode,
        &mut conv_map,
        heap_builder,
        &mut heap_map,
        &mut bb_phis,
    );

    writeln!(
        f,
        "void vogls_proc_{}_{}(int state, ) {{",
        process_idx, &process.name
    )?;

    let mut bb_ident = IndexSet::<BasicBlockKey>::new();
    let mut states_set = IndexSet::<BasicBlockKey>::new();

    bb_stack.push(process.entry);
    bb_ident.insert(process.entry);
    states_set.insert(process.entry);
    while let Some(bb_key) = bb_stack.pop() {
        let bb = &gl.bbs[bb_key];
        bb.terminator.for_each_bb(|bb| {
            if bb_ident.insert(bb) {
                bb_stack.push(bb);
            }
        });

        match bb.terminator {
            BasicBlockTerminator::Wait(target, _)
            | BasicBlockTerminator::VariableWait(target, _)
            | BasicBlockTerminator::WaitRegion(target, _)
            | BasicBlockTerminator::Watch(target, _) => {
                states_set.insert(target);
            }
            BasicBlockTerminator::Jump(..)
            | BasicBlockTerminator::Branch(..)
            | BasicBlockTerminator::Halt => {}
        }
    }

    if states_set.len() > 1 {
        writeln!(f, "{INDENT}switch (state) {{")?;
        for (i, state) in states_set.iter().enumerate() {
            let bb_ident = bb_ident.get_index(state).unwrap();
            writeln!(f, "{INDENT}{INDENT}case {i}: goto L{bb_ident};")?;
        }
        writeln!(f, "{INDENT}}}")?;
        writeln!(f)?;
    }

    let mut tmp_counter = 0u64;
    macro_rules! claim_tmp {
        () => {{
            let t = tmp_counter;
            tmp_counter += 1;
            t
        }};
    }

    bb_seen.clear();
    bb_seen.insert(process.entry);
    bb_stack.push(process.entry);
    while let Some(bb_key) = bb_stack.pop() {
        let bb = &gl.bbs[bb_key];
        bb.terminator.for_each_bb(|bb| {
            if bb_seen.insert(bb) {
                bb_stack.push(bb);
            }
        });

        let ident = bb_ident.get_index(&bb_key).unwrap();

        writeln!(f, "L{ident}:")?;

        for i in &bb.instrs {
            match i {
                I::Constant(dst, bits) => {
                    if bits.size().get() > 16 {
                        todo!();
                    }
                    let t = claim_tmp!();

                    writeln!(
                        f,
                        "{INDENT}int t{t} = 0x{};",
                        bits.display(&BitsFormatOptions {
                            prefix: false,
                            base: BitsFormatBase::UpperHex,
                            separator: None,
                            align: None,
                            fill: '0',
                            width: BitsFormatWidth::Shrink,
                        })
                    )?;
                    store(
                        f,
                        heap_map[dst].to_ref(gl.vars[*dst].size),
                        var_mode[dst],
                        t,
                    )?;
                }
                I::Unary(dst, op, src) => {
                    let mut t = claim_tmp!();

                    let size = gl.vars[*src].size;
                    let msrc = var_mode[src];
                    let mdst = var_mode[dst];
                    load(f, *src, size, var_mode[src], &heap_map, t)?;
                    if msrc != mdst {
                        let unconverted_t = t;
                        t = claim_tmp!();
                        convert(f, size, msrc, mdst, unconverted_t, t)?;
                    }

                    let dst_t = claim_tmp!();
                    use UnaryOp as O;
                    match op {
                        O::Neg => {
                            writeln!(f, "{INDENT}int t{dst_t} = t{t} ^ 0x{:x};", mask(size.get()))?;
                        }
                        O::ReduceOr => {
                            writeln!(f, "{INDENT}int t{dst_t} = t{t} != 0;")?;
                        }
                        O::ReduceAnd => {
                            writeln!(
                                f,
                                "{INDENT}int t{dst_t} = t{t} == 0x{:x};",
                                mask(size.get())
                            )?;
                        }
                        O::ReduceXor => todo!(),
                        O::ContainsX => todo!(),
                    }
                    store(f, *dst, size, mdst, &heap_map, dst_t)?;
                }
                I::Resize(dst, op, src) => todo!(),
                I::Binary(dst, op, lhs, rhs) => todo!(),
                I::Intrinsic(dst, op, items) => todo!(),
                I::LastUpdateTime(dst, signal) => todo!(),
                I::Probe(dst, signal) => todo!(),
                I::Drive(signal, src, partial) => {
                    if partial.is_some() {
                        todo!()
                    }

                    let mut t = claim_tmp!();
                    let size = gl.vars[*src].size;
                    let msrc = var_mode[src];
                    load(f, *src, size, var_mode[src], &heap_map, t)?;
                    if msrc != gl.logic_mode {
                        let unconverted_t = t;
                        t = claim_tmp!();
                        convert(f, size, msrc, gl.logic_mode, unconverted_t, t)?;
                    }

                    store(f, var, size, mode, heap_map, t)?;
                }
                I::Phi(_, _) => todo!(),
            }
        }

        match &bb.terminator {
            BasicBlockTerminator::Wait(basic_block_key, time) => todo!(),
            BasicBlockTerminator::VariableWait(basic_block_key, variable_key) => todo!(),
            BasicBlockTerminator::WaitRegion(basic_block_key, _) => todo!(),
            BasicBlockTerminator::Watch(basic_block_key, items) => todo!(),
            BasicBlockTerminator::Jump(basic_block_key) => todo!(),
            BasicBlockTerminator::Branch(variable_key, basic_block_key, basic_block_key1) => {
                todo!()
            }
            BasicBlockTerminator::Halt => {
                writeln!(f, "{INDENT}return;")?;
            }
        }
    }

    writeln!(f, "}}")?;

    Ok(())
}

const fn mask(num_bits: u32) -> u64 {
    (1u64 << num_bits) - 1
}

fn load(f: &mut impl io::Write, heap_ref: HeapRef, mode: LogicMode, t: u64) -> io::Result<()> {
    if heap_ref.size.get() > 16 {
        todo!()
    }

    let mut num_bits = heap_ref.size.get();
    if mode == LogicMode::FourValue {
        num_bits *= 2;
    }

    let offset = heap_ref.offset;
    let word = offset.bit_offset / 64;
    let shift = offset.bit_offset % 64;
    let mask = mask(num_bits) << shift;

    writeln!(
        f,
        "{INDENT}int t{t} = (heap[{word}] >> {shift}) & 0x{mask:x};"
    )
}

fn store(f: &mut impl io::Write, heap_ref: HeapRef, mode: LogicMode, t: u64) -> io::Result<()> {
    if heap_ref.size.get() > 16 {
        todo!()
    }

    let mut num_bits = heap_ref.size.get();
    if mode == LogicMode::FourValue {
        num_bits *= 2;
    }

    let offset = heap_ref.offset;
    let word = offset.bit_offset / 64;
    let shift = offset.bit_offset % 64;
    let mask = ((1u64 << num_bits) - 1) << shift;

    writeln!(
        f,
        "{INDENT}heap[{word}] = (heap[word] & 0x{mask:x}) | (t{t} << {shift});"
    )
}

fn convert(
    f: &mut impl io::Write,
    size: std::num::NonZero<u32>,
    msrc: LogicMode,
    mdst: LogicMode,
    unconverted_t: u64,
    t: u64,
) -> io::Result<()> {
    todo!()
}

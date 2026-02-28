use std::collections::HashMap;
use std::io;

use vogls_bits::format::{BitsFormatBase, BitsFormatOptions, BitsFormatWidth};
use vogls_codegen::{
    HeapBuilder, HeapOffset, HeapRef, resolve_heap_map, resolve_var_logic_mode_map,
};
use vogls_ir::{
    BasicBlockKey, BasicBlockTerminator, GlobalContext, Instruction, LogicMode, Process,
    ProcessKey, SignalKey, UnaryOp, VariableKey, VectorSize,
};
use vogls_utils::{IndexMap, IndexSet, VgHashMap, VgHashSet};

const INDENT: &str = "    ";

pub fn process_to_procedure_name(process: &Process, idx: usize) -> String {
    format!("vogls_proc_{idx}_{}", &process.name)
}

pub struct Listener {
    pub offset: usize,
    pub process_idx: usize,
    pub process_key: ProcessKey,
    pub state: u32,
}

#[derive(Default)]
pub struct ListenerBuilder {
    pub map: VgHashMap<SignalKey, Vec<Listener>>,
    pub top: usize,
}

impl ListenerBuilder {
    pub fn insert(
        &mut self,
        signal: SignalKey,
        process_idx: usize,
        process_key: ProcessKey,
        state: u32,
    ) {
        let offset = self.top;
        self.top += 1;
        self.map.entry(signal).or_default().push(Listener {
            offset,
            process_idx,
            process_key,
            state,
        });
    }
}

pub fn lower_process(
    f: &mut impl io::Write,
    process_key: ProcessKey,
    process_idx: usize,
    gl: &GlobalContext,
    heap_builder: &mut HeapBuilder,
    listener_builder: &mut ListenerBuilder,
    io_signals: &HashMap<SignalKey, HeapRef>,
) -> io::Result<()> {
    use Instruction as I;

    let process = &gl.processes[process_key];

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

    // @Performance. Amortize buffer.
    let mut buffer = Vec::<u8>::new();
    let procedure = process_to_procedure_name(process, process_idx);

    writeln!(f, "void {procedure}(int state) {{",)?;

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

    use std::io::Write;
    if states_set.len() > 1 {
        writeln!(buffer, "{INDENT}switch (state) {{")?;
        for (i, state) in states_set.iter().enumerate() {
            let bb_ident = bb_ident.get_index(state).unwrap();
            writeln!(buffer, "{INDENT}{INDENT}case {i}: goto L{bb_ident};")?;
        }
        writeln!(buffer, "{INDENT}}}")?;
        writeln!(buffer)?;
    }

    let mut tmp_counter = 0u64;
    macro_rules! claim_tmp {
        () => {{
            let t = tmp_counter;
            writeln!(f, "{INDENT}int t{t};")?;
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

        writeln!(buffer, "L{ident}:")?;

        for i in &bb.instrs {
            match i {
                I::Constant(dst, bits) => {
                    if bits.size().get() > 16 {
                        todo!();
                    }
                    let t = claim_tmp!();

                    writeln!(
                        buffer,
                        "{INDENT}t{t} = 0x{};",
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
                        &mut buffer,
                        heap_map[dst].to_ref(gl.vars[*dst].size),
                        var_mode[dst],
                        t,
                    )?;
                }
                I::Unary(dst, op, src) => {
                    let mut t = claim_tmp!();

                    let src_size = gl.vars[*src].size;
                    let dst_size = gl.vars[*dst].size;
                    let msrc = var_mode[src];
                    let mdst = var_mode[dst];
                    load(
                        &mut buffer,
                        heap_map[src].to_ref(src_size),
                        var_mode[src],
                        t,
                    )?;
                    if msrc != mdst {
                        let unconverted_t = t;
                        t = claim_tmp!();
                        convert(f, src_size, msrc, mdst, unconverted_t, t)?;
                    }

                    let dst_t = claim_tmp!();
                    use UnaryOp as O;
                    match op {
                        O::Neg => {
                            writeln!(
                                buffer,
                                "{INDENT}t{dst_t} = t{t} ^ 0x{:x};",
                                mask(src_size.get())
                            )?;
                        }
                        O::ReduceOr => {
                            writeln!(buffer, "{INDENT}t{dst_t} = t{t} != 0;")?;
                        }
                        O::ReduceAnd => {
                            writeln!(
                                buffer,
                                "{INDENT}t{dst_t} = t{t} == 0x{:x};",
                                mask(src_size.get())
                            )?;
                        }
                        O::ReduceXor => todo!(),
                        O::ContainsX => todo!(),
                    }
                    store(&mut buffer, heap_map[dst].to_ref(dst_size), mdst, dst_t)?;
                }
                I::Resize(dst, op, src) => todo!(),
                I::Binary(dst, op, lhs, rhs) => {
                    let lhs_size = gl.vars[*lhs].size;
                    let rhs_size = gl.vars[*rhs].size;
                    let dst_size = gl.vars[*dst].size;
                    let (mlhs, mrhs, mdst) = (var_mode[lhs], var_mode[rhs], var_mode[dst]);

                    let (mut lhs_t, mut rhs_t) = (claim_tmp!(), claim_tmp!());
                    load(&mut buffer, heap_map[lhs].to_ref(lhs_size), mlhs, lhs_t)?;
                    if mlhs != mdst {
                        let unconverted_t = lhs_t;
                        lhs_t = claim_tmp!();
                        convert(f, lhs_size, mlhs, mdst, unconverted_t, lhs_t)?;
                    }
                    load(&mut buffer, heap_map[rhs].to_ref(rhs_size), mrhs, rhs_t)?;
                    if mrhs != mdst {
                        let unconverted_t = rhs_t;
                        rhs_t = claim_tmp!();
                        convert(f, rhs_size, mrhs, mdst, unconverted_t, rhs_t)?;
                    }

                    let dst_t = claim_tmp!();
                    use vogls_ir::BinaryOp as O;
                    match op {
                        O::And => writeln!(buffer, "{INDENT}t{dst_t} = t{lhs_t} & t{rhs_t};",)?,
                        O::Or => writeln!(buffer, "{INDENT}t{dst_t} = t{lhs_t} | t{rhs_t};",)?,
                        O::Xor => writeln!(buffer, "{INDENT}t{dst_t} = t{lhs_t} & t{rhs_t};",)?,
                        O::Add => todo!(),
                        O::Sub => todo!(),
                        O::Power => todo!(),
                        O::Multiply => todo!(),
                        O::Divide => todo!(),
                        O::Modulus => todo!(),
                        O::UnsignedLessEqual => todo!(),
                        O::SelectBit => todo!(),
                        O::LogicalShiftLeft => todo!(),
                        O::LogicalShiftRight => todo!(),
                        O::ArithmeticShiftRight => todo!(),
                        O::Concat => todo!(),
                        O::CopyX => todo!(),
                        O::CopyZ => todo!(),
                        O::Min => todo!(),
                        O::Max => todo!(),
                        O::CaseEquality => {
                            writeln!(buffer, "{INDENT}t{dst_t} = (int)(t{lhs_t} == t{rhs_t});",)?
                        }
                    }
                    store(&mut buffer, heap_map[dst].to_ref(dst_size), mdst, dst_t)?;
                }
                I::Intrinsic(dst, op, items) => todo!(),
                I::LastUpdateTime(dst, signal) => todo!(),
                I::Probe(dst, signal) => {
                    let mut t = claim_tmp!();
                    let signal = io_signals[signal];
                    let dst_size = gl.vars[*dst].size;
                    let msrc = gl.logic_mode;
                    let mdst = var_mode[dst];
                    load(&mut buffer, signal, gl.logic_mode, t)?;
                    if gl.logic_mode != mdst {
                        let unconverted_t = t;
                        t = claim_tmp!();
                        convert(f, signal.size, msrc, mdst, unconverted_t, t)?;
                    }
                    store(&mut buffer, heap_map[dst].to_ref(dst_size), mdst, t)?;
                }
                I::Drive(signal, src, partial) => {
                    if partial.is_some() {
                        todo!()
                    }

                    let mut t = claim_tmp!();
                    let size = gl.vars[*src].size;
                    let msrc = var_mode[src];
                    load(&mut buffer, heap_map[src].to_ref(size), var_mode[src], t)?;
                    if msrc != gl.logic_mode {
                        let unconverted_t = t;
                        t = claim_tmp!();
                        convert(f, size, msrc, gl.logic_mode, unconverted_t, t)?;
                    }
                    // @TODO: Only wake up if an update occurred.
                    writeln!(buffer, "{INDENT}wake_up_signal_{}_listeners();", io_signals[signal].offset.bit_offset)?;
                    store(&mut buffer, io_signals[signal], gl.logic_mode, t)?;
                }
                I::Phi(_, _) => todo!(),
            }
        }

        match &bb.terminator {
            BasicBlockTerminator::Wait(bb_key, time) => {
                let time = time.0;
                let state = states_set.get_index(bb_key).unwrap();
                writeln!(
                    buffer,
                    "{INDENT}schedule_future_event(time + {time}, (event_t){{.ptr=&{procedure}, .state={state}}});"
                )?;
                writeln!(buffer, "{INDENT}return;",)?;
            }
            BasicBlockTerminator::VariableWait(basic_block_key, variable_key) => todo!(),
            BasicBlockTerminator::WaitRegion(basic_block_key, _) => todo!(),
            BasicBlockTerminator::Watch(bb_key, items) => {
                let state = states_set.get_index(bb_key).unwrap();
                for item in items {
                    let offset = listener_builder.top;
                    writeln!(
                        buffer,
                        "{INDENT}listening[{}] |= 0x{:x};",
                        offset / 64,
                        1 << (offset % 64)
                    )?;
                    listener_builder.insert(*item, process_idx, process_key, state as u32);
                }
                writeln!(
                    buffer,
                    "{INDENT}is_scheduled[{}] &= 0x{:x};",
                    process_idx / 64,
                    !(1 << (process_idx % 64)),
                )?;
                writeln!(buffer, "{INDENT}return;",)?;
            }
            BasicBlockTerminator::Jump(bb_key) => {
                writeln!(
                    buffer,
                    "{INDENT}goto L{};",
                    bb_ident.get_index(bb_key).unwrap()
                )?;
            }
            BasicBlockTerminator::Branch(condition, truthy, falsy) => {
                let truthy = bb_ident.get_index(truthy).unwrap();
                let falsy = bb_ident.get_index(falsy).unwrap();

                let t = claim_tmp!();
                let size = gl.vars[*condition].size;
                let mcondition = var_mode[condition];
                load(&mut buffer, heap_map[condition].to_ref(size), mcondition, t)?;

                writeln!(
                    buffer,
                    "{INDENT}if (t{t} != 0) {{ goto L{truthy}; }} else {{ goto L{falsy}; }}"
                )?;
            }
            BasicBlockTerminator::Halt => {
                writeln!(buffer, "{INDENT}return;")?;
            }
        }
    }

    f.write_all(&buffer)?;
    writeln!(f)?;
    writeln!(f, "}}")?;

    Ok(())
}

const fn mask(num_bits: u32) -> u64 {
    (1u64 << num_bits) - 1
}

fn load(b: &mut impl io::Write, heap_ref: HeapRef, mode: LogicMode, t: u64) -> io::Result<()> {
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
    let mask = mask(num_bits);

    writeln!(b, "{INDENT}t{t} = (heap[{word}] >> {shift}) & 0x{mask:x};")
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
    let mask = !(((1u64 << num_bits) - 1) << shift);

    writeln!(
        f,
        "{INDENT}heap[{word}] = (heap[{word}] & 0x{mask:x}) | (t{t} << {shift});"
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

pub fn lower_signal_listener_waker_header(
    f: &mut impl io::Write,
    signal: SignalKey,
    io_signals: &HashMap<SignalKey, HeapRef>,
) -> io::Result<()> {
    writeln!(
        f,
        "void wake_up_signal_{}_listeners();",
        io_signals[&signal].offset.bit_offset,
    )
}

pub fn lower_signal_listener_waker(
    f: &mut impl io::Write,
    gl: &GlobalContext,
    signal: SignalKey,
    listener_builder: &ListenerBuilder,
    io_signals: &HashMap<SignalKey, HeapRef>,
) -> io::Result<()> {
    writeln!(
        f,
        "void wake_up_signal_{}_listeners() {{",
        io_signals[&signal].offset.bit_offset,
    )?;

    for listener in &listener_builder.map[&signal] {
        writeln!(
            f,
            "{INDENT}if (((listening[{}] >> {}) & 1) != 0 && ((is_scheduled[{}] >> {}) & 1) != 0) {{",
            listener.offset / 64,
            listener.offset % 64,
            listener.process_idx / 64,
            listener.process_idx % 64,
        )?;
        writeln!(
            f,
            "{INDENT}{INDENT}is_scheduled[{}] |= {};",
            listener.process_idx / 64,
            1 << (listener.process_idx % 64),
        )?;
        writeln!(
            f,
            "{INDENT}{INDENT}event_vec_push(&schedule.active_region, (event_t){{.ptr=&{}, .state={}}});",
            process_to_procedure_name(&gl.processes[listener.process_key], listener.process_idx),
            1u64 << (listener.process_idx % 64),
        )?;
        writeln!(f, "{INDENT}}}",)?;
    }

    writeln!(f, "}}")?;
    Ok(())
}

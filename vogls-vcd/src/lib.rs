use std::fmt::Alignment;
use std::path::Path;
use std::sync::Arc;

use vogls_codegen::{Heap, HeapRef};
use vogls_ir::bits::format::{BitsFormatBase, BitsFormatOptions};
use vogls_ir::vcd::{NetType, VcdVariableKey};
use vogls_runtime::RtSignalKey;
use vogls_runtime::plugins::RuntimePlugin;
use vogls_utils::{NonMaxUsize, SecondaryTable, Table, VgHashMap};

use vogls_ir::{SCALAR_VSIZE, SignalKey, SignalSlice, VectorSize};

pub struct RtVcdOutput {
    pub printed_header: bool,
    pub start_ts: u64,
    pub last_ts: u64,
    pub paused: bool,
    pub signal_to_heap: Arc<[HeapRef]>,
    pub children: Vec<VcdScopeItem>,
    pub map: SecondaryTable<RtSignalKey, Box<[(VcdVariableKey, Option<SignalSlice>)]>>,
    pub tracked: SecondaryTable<RtSignalKey, Option<NonMaxUsize>>,
    pub updated_this_time_step: Vec<RtSignalKey>,
    pub writer: Box<dyn std::io::Write + Send + Sync>,
    pub time_scale: u64,
}

#[derive(Debug, Clone)]
pub enum VcdScopeItem {
    Scope(VcdScope),
    Variable(VcdVariable),
}

#[derive(Debug, Clone)]
pub struct VcdScope {
    pub name: String,
    pub items: Vec<VcdScopeItem>,
}

#[derive(Debug, Clone)]
pub struct VcdVariable {
    pub name: String,
    pub variable: VcdVariableKey,
    pub signal: RtSignalKey,
    pub ty: NetType,
    pub msb_lsb: Option<(u32, u32)>,
}

impl RtVcdOutput {
    pub fn new(
        writer: Box<dyn std::io::Write + Send + Sync>,
        signal_to_heap: Arc<[HeapRef]>,
        children: Vec<VcdScopeItem>,
        map: SecondaryTable<RtSignalKey, Box<[(VcdVariableKey, Option<SignalSlice>)]>>,
    ) -> Self {
        let mut tracked = SecondaryTable::new();
        let mut updated_this_time_step = Vec::new();

        for child in &children {
            child.extend_into(&mut tracked, &mut updated_this_time_step);
        }

        Self {
            printed_header: false,
            start_ts: 0,
            last_ts: u64::MAX,
            paused: false,
            signal_to_heap,
            map,
            children,
            tracked,
            updated_this_time_step,
            writer,
            time_scale: 1000,
        }
    }

    pub fn new_path(
        path: &Path,
        signal_to_heap: Arc<[HeapRef]>,
        children: Vec<VcdScopeItem>,
        map: SecondaryTable<RtSignalKey, Box<[(VcdVariableKey, Option<SignalSlice>)]>>,
    ) -> Self {
        // @TODO: Error
        Self::new(
            Box::new(std::fs::File::create(path).unwrap()),
            signal_to_heap,
            children,
            map,
        )
    }
}

impl VcdScopeItem {
    fn lower(
        v: &vogls_ir::vcd::VcdScopeItem,
        table: &Table<VcdVariableKey, vogls_ir::vcd::VcdVariable>,
        map: &VgHashMap<SignalKey, RtSignalKey>,
    ) -> Self {
        match v {
            vogls_ir::vcd::VcdScopeItem::Scope(v) => {
                Self::Scope(VcdScope::lower_scope(v, table, map))
            }
            vogls_ir::vcd::VcdScopeItem::Variable(key) => {
                let v = &table[*key];
                Self::Variable(VcdVariable {
                    name: v.name.clone(),
                    signal: map[&v.signal],
                    variable: *key,
                    ty: v.ty,
                    msb_lsb: v.msb_lsb,
                })
            }
        }
    }
}

impl VcdScope {
    pub fn lower(
        v: &vogls_ir::vcd::VcdOutput,
        signal_map: &VgHashMap<SignalKey, RtSignalKey>,
    ) -> (
        Vec<VcdScopeItem>,
        SecondaryTable<RtSignalKey, Box<[(VcdVariableKey, Option<SignalSlice>)]>>,
    ) {
        let children = v
            .children
            .iter()
            .map(|i| VcdScopeItem::lower(i, &v.table, signal_map))
            .collect();
        let mut vcd_table = SecondaryTable::new();
        for (k, vcd_keys) in &v.signal_map {
            vcd_table.insert(
                signal_map[k],
                vcd_keys
                    .iter()
                    .map(|vcdkey| (*vcdkey, v.table[*vcdkey].signal_slice))
                    .collect(),
            );
        }
        (children, vcd_table)
    }

    fn lower_scope(
        v: &vogls_ir::vcd::VcdScope,
        table: &Table<VcdVariableKey, vogls_ir::vcd::VcdVariable>,
        map: &VgHashMap<SignalKey, RtSignalKey>,
    ) -> VcdScope {
        VcdScope {
            name: v.name.clone(),
            items: v
                .items
                .iter()
                .map(|i| VcdScopeItem::lower(i, table, map))
                .collect(),
        }
    }

    fn write_to(&self, f: &mut impl std::io::Write) -> std::io::Result<()> {
        let Self { name, items } = self;
        write!(f, "$scope module ")?;
        if name.contains(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
            f.write_all(b"\\")?;
        }
        f.write_all(name.as_bytes())?;
        if name.trim().is_empty() {
            f.write_all(b"<anon>")?;
        }
        writeln!(f, " $end")?;
        for item in items {
            item.write_to(f)?;
        }
        writeln!(f, "$upscope $end")?;
        Ok(())
    }

    fn extend_into(
        &self,
        tracked: &mut SecondaryTable<RtSignalKey, Option<NonMaxUsize>>,
        values: &mut Vec<RtSignalKey>,
    ) {
        for i in &self.items {
            i.extend_into(tracked, values);
        }
    }
}

impl VcdScopeItem {
    fn write_to(&self, f: &mut impl std::io::Write) -> std::io::Result<()> {
        match self {
            VcdScopeItem::Scope(scope) => scope.write_to(f),
            VcdScopeItem::Variable(k) => {
                let VcdVariable {
                    name,
                    variable,
                    signal: _,
                    ty,
                    msb_lsb,
                } = k;
                let size = msb_lsb.map_or(SCALAR_VSIZE, |(msb, lsb)| {
                    VectorSize::new((msb.abs_diff(lsb) + 1) as u32).unwrap()
                });
                use vogls_utils::TableKey;
                let idx = variable.get();
                write!(f, "$var ")?;
                f.write_all(
                    match ty {
                        NetType::Integer => "integer",
                        NetType::Register => "reg",
                        NetType::Wire => "wire",
                    }
                    .as_bytes(),
                )?;
                write!(f, " {size} W{idx:X} ")?;
                if name.contains(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
                    f.write_all(b"\\")?;
                }
                f.write_all(name.as_bytes())?;
                f.write_all(b" ")?;
                if let Some((msb, lsb)) = msb_lsb {
                    write!(f, "[{msb}:{lsb}] ")?;
                }
                writeln!(f, "$end")
            }
        }
    }

    pub fn extend_into(
        &self,
        tracked: &mut SecondaryTable<RtSignalKey, Option<NonMaxUsize>>,
        values: &mut Vec<RtSignalKey>,
    ) {
        match self {
            VcdScopeItem::Scope(s) => s.extend_into(tracked, values),
            VcdScopeItem::Variable(k) => {
                tracked.or_insert_with(k.signal, || {
                    let idx = NonMaxUsize::new(values.len()).unwrap();
                    values.push(k.signal);
                    Some(idx)
                });
            }
        }
    }
}

fn dump_time_step(
    printed_header: &mut bool,
    f: &mut impl std::io::Write,
    children: &[VcdScopeItem],
    map: &SecondaryTable<RtSignalKey, Box<[(VcdVariableKey, Option<SignalSlice>)]>>,
    tracked: &mut SecondaryTable<RtSignalKey, Option<NonMaxUsize>>,
    updated_this_time_step: &mut Vec<RtSignalKey>,
    last_ts: &mut u64,
    time_scale: u64,
    time: u64,
    heap: &Heap,
    signals: &[HeapRef],
    finish: bool,
) -> std::io::Result<()> {
    if !*printed_header {
        writeln!(f, "$version Generated by VoGLS $end")?;
        // @TODO
        writeln!(f, "$date @TODO $end")?;
        writeln!(f, "$timescale 1ns $end")?;
        for child in children {
            child.write_to(f)?;
        }
        writeln!(f, "$enddefinitions $end")?;
    }
    *printed_header = true;

    // Only print for the timestamp if something actually happened.
    let mut show_for_timestamp = !updated_this_time_step.is_empty();
    show_for_timestamp |= finish;
    show_for_timestamp &= *last_ts != time;
    if !show_for_timestamp {
        return Ok(());
    }

    *last_ts = time;
    writeln!(f, "#{}", time * time_scale)?;
    for signal in updated_this_time_step.iter() {
        let bits = signals[signal.as_usize()];
        let bits = heap.load_tv_bits(bits);
        for (v, slice) in map[*signal].iter() {
            let bits = match slice {
                None => bits.clone(),
                Some(slice) => bits.slicez(slice.lsb(), slice.width()),
            };
            if bits.size().get() > 1 {
                f.write_all(&[b'b'])?;
            }
            write!(
                f,
                "{}",
                bits.display(&BitsFormatOptions {
                    prefix: false,
                    base: BitsFormatBase::Binary,
                    separator: None,
                    align: Some(Alignment::Right),
                    fill: '0',
                    width: vogls_bits::format::BitsFormatWidth::Expand
                })
            )?;
            if bits.size().get() > 1 {
                f.write_all(&[b' '])?;
            }
            use vogls_utils::TableKey;
            writeln!(f, "W{:X}", v.get())?;
        }
        tracked[*signal] = None;
    }

    updated_this_time_step.clear();
    Ok(())
}

impl RuntimePlugin for RtVcdOutput {
    fn clone(&self) -> vogls_runtime::plugins::RuntimePluginState {
        todo!()
    }

    fn poke_signal(&mut self, signal: RtSignalKey) {
        if !self.paused
            && let Some(idx) = self.tracked.get_mut(signal)
        {
            idx.get_or_insert_with(|| {
                let idx = NonMaxUsize::new(self.updated_this_time_step.len()).unwrap();
                self.updated_this_time_step.push(signal);
                idx
            });
        }
    }

    fn timestep(&mut self, state: &mut vogls_runtime::RuntimeState) {
        dump_time_step(
            &mut self.printed_header,
            &mut self.writer,
            &self.children,
            &self.map,
            &mut self.tracked,
            &mut self.updated_this_time_step,
            &mut self.last_ts,
            self.time_scale,
            state.time,
            &state.heap,
            &self.signal_to_heap,
            false,
        )
        .unwrap();
    }

    fn finish(&mut self, state: &mut vogls_runtime::RuntimeState) {
        dump_time_step(
            &mut self.printed_header,
            &mut self.writer,
            &self.children,
            &self.map,
            &mut self.tracked,
            &mut self.updated_this_time_step,
            &mut self.last_ts,
            self.time_scale,
            state.time,
            &state.heap,
            &self.signal_to_heap,
            true,
        )
        .unwrap();
        self.writer.flush().unwrap();
    }
}

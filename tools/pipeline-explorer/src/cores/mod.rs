//! The processors the explorer can trace, each behind the [`Core`] trait.
//!
//! Every core is simulated inside a testbench of its own (`instances/<core>/tb.v`)
//! that declares the memory arrays the program is loaded into and an active-low
//! reset. The plumbing they share lives in [`Testbench`].

use vogls::design::{Arena, Macro, RtSignal};
use vogls::frontend::symbol_table::SymbolId;
use vogls::ir::Mode;
use vogls::{
    Bits, Design, DesignState, ElaboratedDesign, LogicMode, NeverWorld, OptFlags, Optimizations,
    SignalHandle, VectorSize,
};

use crate::TraceError;
use crate::program::{MemoryMap, ProgramSections};

pub mod hazard3;
pub mod ibex;
pub mod neorv32;
pub mod picorv32;

pub use self::hazard3::{Hazard3, Hazard3Config};
pub use self::ibex::{Ibex, IbexConfig};
pub use self::neorv32::Neorv32;
pub use self::picorv32::{PicoRV32Config, PicoRv32};

/// Properties of a core.
pub struct Description {
    /// Name of each pipeline stage.
    pub pipeline_stages: Vec<&'static str>,

    pub imem_size: u32,
    pub dmem_size: u32,
}

/// What every pipeline stage held, cycle by cycle.
pub struct PipelineTrace {
    boot: u32,
    /// One list per stage, one entry per cycle. An entry is the slot of the
    /// instruction in the stage: `0` when the stage is idle, and `n` for the
    /// instruction at `boot + 4 * (n - 1)` otherwise.
    pub stage_pcs: Vec<Vec<u32>>,
}

impl PipelineTrace {
    pub fn new(boot: u32, num_stages: usize, num_cycles: u32) -> Self {
        Self {
            boot,
            stage_pcs: vec![Vec::with_capacity(num_cycles as usize); num_stages],
        }
    }

    /// Records the program counter of the instruction `stage` holds this
    /// cycle, or `None` when the stage is idle.
    pub fn push(&mut self, stage: usize, pc: Option<u32>) {
        // Slots are numbered from the reset vector, one per instruction word.
        let slot = match pc.and_then(|pc| pc.checked_sub(self.boot)) {
            Some(offset) => offset / 4 + 1,
            None => 0,
        };
        self.stage_pcs[stage].push(slot);
    }
}

pub trait Core {
    type Design;
    type State;
    /// Configuration of the core.
    type Params;

    /// Where the program goes in the memory of the core.
    fn memory_map(&self) -> MemoryMap;

    fn description(&self, params: &Self::Params) -> Description;

    /// Compile the design and get handles to signals.
    fn compile(&self, params: &Self::Params) -> Result<(Self::Design, Self::State), TraceError>;

    /// Run the prologue to start running.
    /// - Reset the RTL design.
    /// - Write the `sections` into memory.
    fn prologue(
        &self,
        design: &Self::Design,
        state: &mut Self::State,
        sections: &ProgramSections,
    ) -> Result<(), TraceError>;

    /// Iterate a cycle and push the program counters the pipeline stages hold
    /// at its end.
    fn iterate_cycle(
        &self,
        design: &Self::Design,
        state: &mut Self::State,
        pt: &mut PipelineTrace,
    ) -> Result<(), TraceError>;

    /// Did the device trap?
    fn is_trapped(&self, design: &Self::Design, state: &Self::State) -> bool;
}

/// Simulation time of one clock cycle of every testbench.
const CYCLE: u64 = 2;
/// How long the core is held in reset.
const RESET_CYCLES: u64 = 10;

/// The instruction memory of every testbench: `reg [31:0] memory [0:255]`.
const INSTRUCTION_MEMORY: u32 = 256 * 4;
/// The data memory of every testbench: 1001 words.
const DATA_MEMORY: u32 = 1001 * 4;

/// The memory map every testbench shares, for a core booting at `boot`.
///
/// `.text` and `.rodata` share the instruction memory; `.data` and `.bss`
/// share the data memory.
const fn memory_map(boot: u32) -> MemoryMap {
    MemoryMap {
        boot,
        text: boot..0x300,
        rodata: 0x300..INSTRUCTION_MEMORY,
        data: INSTRUCTION_MEMORY..0x800,
        bss: 0x800..INSTRUCTION_MEMORY + DATA_MEMORY,
    }
}

/// `bytes` as a vector of exactly their size.
fn to_bits(bytes: &[u8]) -> Bits {
    let size = VectorSize::new((bytes.len() * 8) as u32).expect("empty memory region");

    // `Bits` is backed by 64-bit words, so the array is padded up to a whole
    // number of words; `from_boxed_slice` masks off the padding.
    let mut words = vec![0u64; bytes.len().div_ceil(8)];
    bytemuck::cast_slice_mut(&mut words)[..bytes.len()].copy_from_slice(bytes);

    Bits::from_boxed_slice(Mode::TwoValue, size, words.into_boxed_slice())
}

/// The simulation state of a [`Testbench`].
pub struct Sim {
    state: DesignState,
    world: NeverWorld,
}

/// A compiled testbench, with the signals every testbench has.
pub struct Testbench {
    design: Design,
    imem_size: u32,
    dmem_size: u32,
    instruction_memory: RtSignal,
    data_memory: RtSignal,
    reset_n: RtSignal,
}

impl Testbench {
    /// Compiles the testbench `tb` from `sources`, with each of `macros`
    /// defined.
    ///
    /// `memories` names the instruction and data memory arrays of the
    /// testbench, which have to be the sizes `description` gives.
    ///
    /// `probe` looks up the signals the core reads, while the design is still
    /// elaborated; resolve them with [`Testbench::resolve`] afterwards.
    fn compile<H>(
        sources: &[&'static str],
        macros: impl IntoIterator<Item = &'static str>,
        reset_n: &str,
        memories: [&str; 2],
        description: &Description,
        probe: impl FnOnce(&mut ElaboratedDesign<'_>, SymbolId) -> H,
    ) -> Result<(Self, Sim, H), TraceError> {
        let mut builder = vogls::DesignBuilder::new();
        let arena = Arena::default();
        let mut world = NeverWorld::new();
        for name in macros {
            builder.define_macro(name, Macro::default());
        }
        for source in sources {
            builder
                .add_source_str_in_world(&mut world, *source)
                .map_err(|_| TraceError::Build("failed to tokenize"))?;
        }
        let parsed = builder
            .parse(&arena)
            .map_err(|_| TraceError::Build("failed to parse"))?;
        let mut design = parsed
            .elaborate(LogicMode::TwoValue, Some("tb"))
            .map_err(|_| TraceError::Build("failed to elaborate"))?;

        let root = design.table().roots()[0];
        let [instruction_memory, data_memory] =
            memories.map(|name| signal(&mut design, root, name));
        let reset_n = signal(&mut design, root, reset_n);
        let handles = probe(&mut design, root);

        let mut design = design
            .lower(Vec::new())
            .map_err(|_| TraceError::Build("failed to lower"))?;
        design.optimize(Optimizations {
            rounds: 2,
            flags: OptFlags::ALL,
        });
        let (design, state) = design
            .to_bytecode()
            .map_err(|_| TraceError::Build("failed to convert to bytecode"))?;

        let instruction_memory = design.resolve_handle(instruction_memory);
        let data_memory = design.resolve_handle(data_memory);

        // The testbench declares its memories itself, so check them against
        // the sizes the rest of the explorer lays the program out for. The
        // declared type only gives the width of one word; the value the
        // simulator holds covers the whole array.
        let bytes = |memory| design.get_signal(&state, memory).size().get() / 8;
        if bytes(instruction_memory) != description.imem_size {
            return Err(TraceError::Build(
                "the instruction memory of the testbench is not the size the core describes",
            ));
        }
        if bytes(data_memory) != description.dmem_size {
            return Err(TraceError::Build(
                "the data memory of the testbench is not the size the core describes",
            ));
        }

        let testbench = Self {
            imem_size: description.imem_size,
            dmem_size: description.dmem_size,
            instruction_memory,
            data_memory,
            reset_n: design.resolve_handle(reset_n),
            design,
        };
        Ok((testbench, Sim { state, world }, handles))
    }

    pub fn resolve(&self, handle: SignalHandle) -> RtSignal {
        self.design.resolve_handle(handle)
    }

    fn run_until(&self, sim: &mut Sim, time: u64) -> Result<(), TraceError> {
        self.design
            .run(&mut sim.state, &mut sim.world, time)
            .map_err(|err| TraceError::Run(err.to_string()))
    }

    fn reset(
        &self,
        sim: &mut Sim,
        map: &MemoryMap,
        sections: &ProgramSections,
    ) -> Result<(), TraceError> {
        // `to_sections` keeps every section within its range, and every range
        // lies within one memory (see the tests), so the copies below stay in
        // bounds.
        let instruction_size = self.imem_size as usize;
        let mut instruction = vec![0u8; instruction_size];
        let mut data = vec![0u8; self.dmem_size as usize];
        for (range, bytes) in [
            (&map.text, &sections.text),
            (&map.rodata, &sections.rodata),
            (&map.data, &sections.data),
        ] {
            let start = range.start as usize;
            let (memory, offset) = if start < instruction_size {
                (&mut instruction, start)
            } else {
                (&mut data, start - instruction_size)
            };
            memory[offset..offset + bytes.len()].copy_from_slice(bytes);
        }
        // `.bss` is zero, as is all memory no section writes.

        self.design.set_signal(
            &mut sim.state,
            self.instruction_memory,
            &to_bits(&instruction),
        );
        self.design
            .set_signal(&mut sim.state, self.data_memory, &to_bits(&data));
        self.design
            .set_signal(&mut sim.state, self.reset_n, &false.into());
        let time = sim.state.runtime().time;
        self.run_until(sim, time + CYCLE * RESET_CYCLES)?;
        self.design
            .set_signal(&mut sim.state, self.reset_n, &true.into());
        Ok(())
    }

    /// Advances the simulation by one clock cycle.
    fn step(&self, sim: &mut Sim) -> Result<(), TraceError> {
        let time = sim.state.runtime().time;
        self.run_until(sim, time + CYCLE)
    }

    fn get(&self, sim: &Sim, signal: RtSignal) -> Bits {
        self.design.get_signal(&sim.state, signal)
    }

    fn is_one(&self, sim: &Sim, signal: RtSignal) -> bool {
        self.get(sim, signal).is_one()
    }

    /// The program counter `pc` of a stage, when its `active` flag is set.
    fn stage_pc(&self, sim: &Sim, pc: RtSignal, active: RtSignal) -> Option<u32> {
        if self.is_one(sim, active) {
            self.get(sim, pc).extract_exact_u32()
        } else {
            None
        }
    }
}

/// The signal `ident` declared in `scope`.
fn signal(design: &mut ElaboratedDesign<'_>, scope: SymbolId, ident: &str) -> SignalHandle {
    let sid = lookup(design, scope, ident);
    design.get_signal_handle(sid).unwrap()
}

/// The instance or other symbol `ident` declared in `scope`.
fn lookup(design: &ElaboratedDesign<'_>, scope: SymbolId, ident: &str) -> SymbolId {
    design
        .table()
        .resolve(scope, design.ident_table().get(ident).unwrap())
        .unwrap()
}

/// The names of the `(enabled, name)` macros that are enabled.
fn enabled<const N: usize>(
    macros: [(bool, &'static str); N],
) -> impl Iterator<Item = &'static str> {
    macros
        .into_iter()
        .filter_map(|(enabled, name)| enabled.then_some(name))
}

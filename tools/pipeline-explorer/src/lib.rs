use std::fmt;

use js_sys::{Array, Object, Reflect};
use trva::error::AssembleError;
use trva::isa::{ExtensionSet, Isa};
use trva::{Assembler, SectionPositions};
use vogls::design::{Arena, Macro};
use vogls::frontend::symbol_table::SymbolId;
use vogls::ir::Mode;
use vogls::{
    Bits, ElaboratedDesign, LogicMode, NeverWorld, OptFlags, Optimizations, SignalHandle,
    VectorSize,
};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};

const RV32IM: Isa = Isa {
    exts: ExtensionSet::INTEGER_MULDIV,
    xlen: trva::isa::XLen::Rv32,
};

fn get_signal(design: &mut ElaboratedDesign<'_>, scope: SymbolId, ident: &str) -> SignalHandle {
    let sid = design
        .table()
        .resolve(scope, design.ident_table().get(ident).unwrap())
        .unwrap();
    design.get_signal_handle(sid).unwrap()
}

#[derive(Debug)]
pub enum TraceError {
    Assemble(AssembleError),
    Build(&'static str),
}

impl fmt::Display for TraceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TraceError::Assemble(err) => write!(f, "trace-err: {err}"),
            TraceError::Build(err) => write!(f, "trace-err[build]: {err}"),
        }
    }
}
impl std::error::Error for TraceError {}
impl From<AssembleError> for TraceError {
    fn from(err: AssembleError) -> Self {
        Self::Assemble(err)
    }
}
impl From<Box<AssembleError>> for TraceError {
    fn from(err: Box<AssembleError>) -> Self {
        Self::Assemble(*err)
    }
}

pub struct ReturnValue {
    pub instructions: Vec<String>,
    pub pipeline: Pipeline,
}
pub struct Pipeline {
    pub traces: Vec<Vec<u32>>,
    pub keys: &'static [&'static str],
    pub cycles: usize,
}

impl ReturnValue {
    fn into_object(self) -> Object {
        let traces: js_sys::Array = self
            .pipeline
            .traces
            .into_iter()
            .map(|pc| js_sys::Uint32Array::new_from_slice(&pc))
            .collect();
        let pipeline = obj(&[
            ("traces", traces.into()),
            ("keys", str_array(self.pipeline.keys).into()),
            ("cycles", self.pipeline.cycles.into()),
        ]);
        obj(&[
            (
                "instructions",
                self.instructions
                    .iter()
                    .map(|s| JsValue::from_str(s))
                    .collect::<Array>()
                    .into(),
            ),
            ("pipeline", pipeline.into()),
        ])
    }
}

pub struct IbexConfig {
    pub wb_stage: bool,
}
pub struct PicoRV32Config {
    pub enable_mul: bool,
    pub enable_div: bool,
    pub two_stage_shift: bool,
    pub barrel_shifter: bool,
    pub two_cycle_compare: bool,
    pub two_cycle_alu: bool,
    pub enable_fast_mul: bool,
}

pub fn get_neorv32_trace(assembly: &str, num_cycles: u32) -> Result<ReturnValue, TraceError> {
    let mut builder = vogls::DesignBuilder::new();
    let mut arena = Arena::default();
    builder
        .add_source_str(include_str!(
            "../instances/neorv32/neorv32_verilog_wrapper.v"
        ))
        .map_err(|_| TraceError::Build("failed to tokenize"))?
        .add_source_str(include_str!("../instances/neorv32/tb.v"))
        .map_err(|_| TraceError::Build("failed to tokenize"))?;
    let parsed = builder
        .parse(&arena)
        .map_err(|_| TraceError::Build("failed to parse"))?;
    let mut design = parsed
        .elaborate(LogicMode::TwoValue, Some("tb"))
        .map_err(|_| TraceError::Build("failed to elaborate"))?;

    let root = design.table().roots()[0];
    let memory_signal = get_signal(&mut design, root, "memory");
    let rstn = get_signal(&mut design, root, "rstn");
    let trap_h = get_signal(&mut design, root, "trap_q");
    let if_q_h = get_signal(&mut design, root, "if_q");
    let if_pc_q_h = get_signal(&mut design, root, "if_pc_q");
    let is_q_h = get_signal(&mut design, root, "is_q");
    let ex_q_h = get_signal(&mut design, root, "ex_q");
    let alu_q_h = get_signal(&mut design, root, "alu_q");
    let br_q_h = get_signal(&mut design, root, "br_q");
    let ma_q_h = get_signal(&mut design, root, "ma_q");
    let pc_q_h = get_signal(&mut design, root, "pc_q");

    let mut design = design
        .lower(Vec::new())
        .map_err(|_| TraceError::Build("failed to lower"))?;
    design.optimize(Optimizations {
        rounds: 2,
        flags: OptFlags::ALL,
    });
    let (design, mut state) = design
        .to_bytecode()
        .map_err(|_| TraceError::Build("failed to convert to bytecode"))?;
    arena.reset();

    const CYCLE: u64 = 2;
    const BOOT_OFFSET: usize = 0x0000_0000;
    const MEMORY_SIZE: usize = 256 * 4;

    let positions = SectionPositions {
        text: 0x0000_0000u32,
        data: 1024u32,
        rodata: 0u32,
        bss: 1024u32,
    };
    let program = Assembler::new(RV32IM, positions)
        .with_source(assembly)?
        .assemble();

    let mut instructions = Vec::<String>::new();
    let mut islice = &program.text[..];
    while !islice.is_empty() {
        let i = trva::encoding::Instruction::decode(&mut islice).unwrap();
        instructions.push(match i {
            None => "unknown".into(),
            Some(i) => i.to_string(),
        });
    }

    assert!(BOOT_OFFSET + program.text.len() <= MEMORY_SIZE);
    let mut target_memory = vec![0u64; MEMORY_SIZE / 8];
    bytemuck::cast_slice_mut(&mut target_memory)[BOOT_OFFSET..BOOT_OFFSET + program.text.len()]
        .copy_from_slice(&program.text);
    let memory_bits = Bits::from_boxed_slice(
        Mode::TwoValue,
        VectorSize::new((MEMORY_SIZE * 8) as u32).unwrap(),
        target_memory.into_boxed_slice(),
    );

    let memory_signal = design.resolve_handle(memory_signal);
    let rstn = design.resolve_handle(rstn);
    let trap = design.resolve_handle(trap_h);
    // Registered stage-active flags and PCs from tb.v
    let if_q = design.resolve_handle(if_q_h);
    let if_pc_q = design.resolve_handle(if_pc_q_h);
    let is_q = design.resolve_handle(is_q_h);
    let ex_q = design.resolve_handle(ex_q_h);
    let alu_q = design.resolve_handle(alu_q_h);
    let br_q = design.resolve_handle(br_q_h);
    let ma_q = design.resolve_handle(ma_q_h);
    let pc_q = design.resolve_handle(pc_q_h);

    design.set_signal(&mut state, memory_signal, &memory_bits);
    design.set_signal(&mut state, rstn, &false.into());

    design
        .run(&mut state, &mut NeverWorld::new(), CYCLE * 10)
        .expect("failed to run");
    design.set_signal(&mut state, rstn, &true.into());

    const STAGES: &[&str] = &["IF", "IS", "EX", "ALU", "BR", "MA"];

    let to_slot = |pc: u32, active: bool| -> u32 {
        if active && pc >= BOOT_OFFSET as u32 {
            (pc - BOOT_OFFSET as u32) / 4 + 1
        } else {
            0
        }
    };

    let mut output_cycles = num_cycles;
    let mut traces = vec![Vec::with_capacity(num_cycles as usize); STAGES.len()];
    for i in 0..num_cycles {
        let time = state.runtime().time;
        design
            .run(&mut state, &mut NeverWorld::new(), time + CYCLE)
            .expect("failed to run");

        let pc = design.get_signal(&state, pc_q).extract_exact_u32().unwrap();
        let if_pc = design
            .get_signal(&state, if_pc_q)
            .extract_exact_u32()
            .unwrap();

        traces[0].push(to_slot(if_pc, design.get_signal(&state, if_q).is_one()));
        traces[1].push(to_slot(pc, design.get_signal(&state, is_q).is_one()));
        traces[2].push(to_slot(pc, design.get_signal(&state, ex_q).is_one()));
        traces[3].push(to_slot(pc, design.get_signal(&state, alu_q).is_one()));
        traces[4].push(to_slot(pc, design.get_signal(&state, br_q).is_one()));
        traces[5].push(to_slot(pc, design.get_signal(&state, ma_q).is_one()));

        if design.get_signal(&state, trap).is_one() {
            output_cycles = i;
            break;
        }
    }

    Ok(ReturnValue {
        instructions,
        pipeline: Pipeline {
            traces,
            keys: STAGES,
            cycles: output_cycles as usize,
        },
    })
}

pub fn get_ibex_trace(
    assembly: &str,
    num_cycles: u32,
    cfg: &IbexConfig,
) -> Result<ReturnValue, TraceError> {
    let mut builder = vogls::DesignBuilder::new();
    let mut arena = Arena::default();
    if cfg.wb_stage {
        builder.define_macro("WRITEBACK_STAGE", Macro::default());
    }
    builder
        .add_source_str(include_str!("../instances/ibex/ibex_fixed.v"))
        .map_err(|_| TraceError::Build("failed to tokenize"))?
        .add_source_str(include_str!("../instances/ibex/tb.v"))
        .map_err(|_| TraceError::Build("failed to tokenize"))?;
    let parsed = builder
        .parse(&arena)
        .map_err(|_| TraceError::Build("failed to parse"))?;
    let mut design = parsed
        .elaborate(LogicMode::TwoValue, Some("tb"))
        .map_err(|_| TraceError::Build("failed to elaborate"))?;

    let root = design.table().roots()[0];
    let rst_n = get_signal(&mut design, root, "rst_n");
    let memory_signal_h = get_signal(&mut design, root, "memory");

    // Navigate: tb -> u_top (ibex_top) -> u_ibex_core (ibex_core)
    let u_top = design
        .table()
        .resolve(root, design.ident_table().get("u_top").unwrap())
        .unwrap();
    let u_ibex_core = design
        .table()
        .resolve(u_top, design.ident_table().get("u_ibex_core").unwrap())
        .unwrap();

    // IF stage: pc_if is the current fetch PC; it is valid when instr_req_int is high.
    let pc_if_h = get_signal(&mut design, u_ibex_core, "pc_if");
    let if_valid_h = get_signal(&mut design, u_ibex_core, "instr_req_int");
    // ID/EX stage: pc_id is the PC of the instruction in decode/execute.
    let pc_id_h = get_signal(&mut design, u_ibex_core, "pc_id");
    let id_valid_h = get_signal(&mut design, u_ibex_core, "instr_valid_id");
    // WB stage: pc_wb + instr_done_wb are the proper signals with WritebackStage=1.
    let pc_wb_h = cfg
        .wb_stage
        .then(|| get_signal(&mut design, u_ibex_core, "pc_wb"));
    let wb_valid_h = cfg
        .wb_stage
        .then(|| get_signal(&mut design, u_ibex_core, "instr_done_wb"));
    let trap_h = get_signal(&mut design, u_ibex_core, "csr_save_cause");

    let mut design = design
        .lower(Vec::new())
        .map_err(|_| TraceError::Build("failed to lower"))?;
    design.optimize(Optimizations {
        rounds: 2,
        flags: OptFlags::ALL,
    });
    let (design, mut state) = design
        .to_bytecode()
        .map_err(|_| TraceError::Build("failed to convert to bytecode"))?;
    arena.reset();

    const CYCLE: u64 = 2;
    // Boot address is 0x80 = 128 bytes, so prefix program with 128 zero bytes.
    const BOOT_OFFSET: usize = 0x80;
    const MEMORY_SIZE: usize = 256 * 4;

    let positions = SectionPositions {
        text: 0x0000_0080u32,
        data: 1024u32,
        rodata: 0u32,
        bss: 0u32,
    };
    let program = Assembler::new(RV32IM, positions)
        .with_source(assembly)?
        .assemble();

    let mut instructions = Vec::<String>::new();
    let mut islice = &program.text[..];
    while !islice.is_empty() {
        let i = trva::encoding::Instruction::decode(&mut islice).unwrap();
        instructions.push(match i {
            None => "unknown".into(),
            Some(i) => i.to_string(),
        });
    }

    assert!(BOOT_OFFSET + program.text.len() <= MEMORY_SIZE);
    let mut target_memory = vec![0u64; MEMORY_SIZE / 8];
    bytemuck::cast_slice_mut(&mut target_memory)[BOOT_OFFSET..BOOT_OFFSET + program.text.len()]
        .copy_from_slice(&program.text);
    let memory_bits = Bits::from_boxed_slice(
        Mode::TwoValue,
        VectorSize::new((MEMORY_SIZE * 8) as u32).unwrap(),
        target_memory.into_boxed_slice(),
    );

    let memory_signal = design.resolve_handle(memory_signal_h);
    let rst_n = design.resolve_handle(rst_n);
    // IF stage: pc_if is the current fetch PC; it is valid when instr_req_int is high.
    let pc_if = design.resolve_handle(pc_if_h);
    let if_valid = design.resolve_handle(if_valid_h);
    // ID/EX stage: pc_id is the PC of the instruction in decode/execute.
    let pc_id = design.resolve_handle(pc_id_h);
    let id_valid = design.resolve_handle(id_valid_h);
    // WB stage: pc_wb + instr_done_wb are the proper signals with WritebackStage=1.
    let pc_wb = pc_wb_h.map(|h| design.resolve_handle(h));
    let wb_valid = wb_valid_h.map(|h| design.resolve_handle(h));
    let trap = design.resolve_handle(trap_h);

    design.set_signal(&mut state, memory_signal, &memory_bits);
    design.set_signal(&mut state, rst_n, &false.into());
    design
        .run(&mut state, &mut NeverWorld::new(), CYCLE * 10)
        .expect("failed to run");
    design.set_signal(&mut state, rst_n, &true.into());

    let stages: &[&str] = if cfg.wb_stage {
        &["IF", "ID", "WB"]
    } else {
        &["IF", "ID"]
    };

    let mut output_cycles = num_cycles;
    let mut traces = vec![Vec::with_capacity(num_cycles as usize); stages.len()];
    for i in 0..num_cycles {
        let time = state.runtime().time;
        design
            .run(&mut state, &mut NeverWorld::new(), time + CYCLE)
            .expect("failed to run");

        let to_slot = |pc: u32, active: bool| -> u32 {
            if active && pc >= BOOT_OFFSET as u32 {
                (pc - BOOT_OFFSET as u32) / 4 + 1
            } else {
                0
            }
        };

        // IF
        let pc = design
            .get_signal(&state, pc_if)
            .extract_exact_u32()
            .unwrap();
        let active = design.get_signal(&state, if_valid).is_one();
        traces[0].push(to_slot(pc, active));

        // ID
        let pc = design
            .get_signal(&state, pc_id)
            .extract_exact_u32()
            .unwrap();
        let active = design.get_signal(&state, id_valid).is_one();
        traces[1].push(to_slot(pc, active));

        if cfg.wb_stage {
            let pc = design
                .get_signal(&state, pc_wb.unwrap())
                .extract_exact_u32()
                .unwrap();
            let active = design.get_signal(&state, wb_valid.unwrap()).is_one();
            traces[2].push(to_slot(pc, active));
        }

        if design.get_signal(&state, trap).is_one() {
            output_cycles = i;
            break;
        }
    }

    Ok(ReturnValue {
        instructions,
        pipeline: Pipeline {
            traces,
            keys: stages,
            cycles: output_cycles as usize,
        },
    })
}

pub fn get_trace(
    assembly: &str,
    config: &PicoRV32Config,
    num_cycles: u32,
) -> Result<ReturnValue, TraceError> {
    let mut builder = vogls::DesignBuilder::new();
    let mut arena = Arena::default();
    if config.enable_mul {
        builder.define_macro("ENABLE_MUL", Macro::default());
    }
    if config.enable_div {
        builder.define_macro("ENABLE_DIV", Macro::default());
    }
    if config.two_stage_shift {
        builder.define_macro("TWO_STAGE_SHIFT", Macro::default());
    }
    if config.barrel_shifter {
        builder.define_macro("BARREL_SHIFTER", Macro::default());
    }
    if config.two_cycle_compare {
        builder.define_macro("TWO_CYCLE_COMPARE", Macro::default());
    }
    if config.two_cycle_alu {
        builder.define_macro("TWO_CYCLE_ALU", Macro::default());
    }
    if config.enable_fast_mul {
        builder.define_macro("ENABLE_FAST_MUL", Macro::default());
    }
    builder
        .add_source_str(include_str!("../../../submodules/picorv32/picorv32.v"))
        .map_err(|_| TraceError::Build("failed to tokenize"))?
        .add_source_str(include_str!("../instances/picorv32/tb.v"))
        .map_err(|_| TraceError::Build("failed to tokenize"))?;
    let parsed = builder
        .parse(&arena)
        .map_err(|_| TraceError::Build("failed to parse"))?;
    let mut design = parsed
        .elaborate(LogicMode::TwoValue, Some("tb"))
        .map_err(|_| TraceError::Build("failed to elaborate"))?;

    let root = design.table().roots()[0];
    let nrst_h = get_signal(&mut design, root, "resetn");
    let memory_signal_h = get_signal(&mut design, root, "memory");
    let trap_h = get_signal(&mut design, root, "trap");
    let proc = design
        .table()
        .resolve(root, design.ident_table().get("proc").unwrap())
        .unwrap();
    let reg_pc_h = get_signal(&mut design, proc, "reg_pc");
    let cpu_state_h = get_signal(&mut design, proc, "cpu_state");

    let mut design = design
        .lower(Vec::new())
        .map_err(|_| TraceError::Build("failed to lower"))?;
    design.optimize(Optimizations {
        rounds: 2,
        flags: OptFlags::ALL,
    });
    let (design, mut state) = design
        .to_bytecode()
        .map_err(|_| TraceError::Build("failed to convert to bytecode"))?;
    arena.reset();

    const CYCLE: u64 = 2;
    const MEMORY_SIZE: usize = 256 * 4;

    let nrst = design.resolve_handle(nrst_h);
    let memory_signal = design.resolve_handle(memory_signal_h);
    let trap = design.resolve_handle(trap_h);
    let reg_pc = design.resolve_handle(reg_pc_h);
    let cpu_state = design.resolve_handle(cpu_state_h);

    let positions = SectionPositions {
        text: 0x0000_0000u32,
        data: 1024u32,
        rodata: 0u32,
        bss: 1024u32,
    };
    let program = Assembler::new(RV32IM, positions)
        .with_source(assembly)?
        .assemble();

    let mut instructions = Vec::<String>::new();
    let mut islice = &program.text[..];
    while !islice.is_empty() {
        let i = trva::encoding::Instruction::decode(&mut islice).unwrap();
        instructions.push(match i {
            None => "unknown".into(),
            Some(i) => i.to_string(),
        });
    }

    assert!(program.text.len() <= MEMORY_SIZE);
    let mut target_memory = vec![0u64; MEMORY_SIZE / 8];
    bytemuck::cast_slice_mut(&mut target_memory)[..program.text.len()]
        .copy_from_slice(&program.text);
    let memory = Bits::from_boxed_slice(
        Mode::TwoValue,
        VectorSize::new((MEMORY_SIZE * 8) as u32).unwrap(),
        target_memory.into_boxed_slice(),
    );
    design.set_signal(&mut state, memory_signal, &memory);
    design.set_signal(&mut state, nrst, &false.into());
    design
        .run(&mut state, &mut NeverWorld::new(), CYCLE * 10)
        .expect("failed to run");
    design.set_signal(&mut state, nrst, &true.into());

    let stages = &["TR", "IF", "L1", "L2", "EX", "SH", "ST", "LD"];

    let mut output_cycles = num_cycles;
    let mut traces = (0..8)
        .map(|_| Vec::with_capacity(num_cycles as usize))
        .collect::<Vec<_>>();
    for i in 0..num_cycles {
        let time = state.runtime().time;
        design
            .run(&mut state, &mut NeverWorld::new(), time + CYCLE)
            .expect("failed to run");

        let pc = design
            .get_signal(&state, reg_pc)
            .extract_exact_u32()
            .unwrap();
        let cpu_state = design.get_signal(&state, cpu_state).extract_u32().unwrap();
        let cpu_state = 7 - cpu_state.trailing_zeros();

        eprintln!("0x{pc:08x}: {cpu_state:08b}");

        #[expect(clippy::needless_range_loop)]
        for i in 0..stages.len() {
            let value = if cpu_state == i as u32 {
                (pc / 4) + 1
            } else {
                0
            };
            traces[i].push(value);
        }

        if design.get_signal(&state, trap).is_one() {
            output_cycles = i;
            println!("TRAP!");
            break;
        }
    }

    Ok(ReturnValue {
        instructions,
        pipeline: Pipeline {
            traces,
            keys: stages,
            cycles: output_cycles as usize,
        },
    })
}

macro_rules! get_bool {
    ($config:expr, $option:literal) => {{
        Reflect::get(&$config, &JsValue::from_str($option))
            .map_err(|_| JsError::new(concat!("failed to access key '", $option, "'")))?
            .as_bool()
            .ok_or_else(|| JsError::new(concat!("key '", $option, "' is not a bool")))?
    }};
}

#[wasm_bindgen]
pub fn get_js_ibex_trace(
    assembly: &str,
    config: Object,
    num_cycles: u32,
) -> Result<Object, JsError> {
    let config = IbexConfig {
        wb_stage: get_bool!(config, "wb_stage"),
    };
    let trace = get_ibex_trace(assembly, num_cycles, &config)?;
    Ok(trace.into_object())
}

#[wasm_bindgen]
pub fn get_js_neorv32_trace(
    assembly: &str,
    _config: Object,
    num_cycles: u32,
) -> Result<Object, JsError> {
    let trace = get_neorv32_trace(assembly, num_cycles)?;
    Ok(trace.into_object())
}

#[wasm_bindgen]
pub fn get_js_trace(assembly: &str, config: Object, num_cycles: u32) -> Result<Object, JsError> {
    let config = PicoRV32Config {
        enable_mul: get_bool!(config, "enable_mul"),
        enable_div: get_bool!(config, "enable_div"),
        two_stage_shift: get_bool!(config, "two_stage_shift"),
        barrel_shifter: get_bool!(config, "barrel_shifter"),
        two_cycle_compare: get_bool!(config, "two_cycle_compare"),
        two_cycle_alu: get_bool!(config, "two_cycle_alu"),
        enable_fast_mul: get_bool!(config, "enable_fast_mul"),
    };
    let trace = get_trace(assembly, &config, num_cycles)?;
    Ok(trace.into_object())
}

fn obj(entries: &[(&str, JsValue)]) -> Object {
    let o = Object::new();
    for (k, v) in entries {
        Reflect::set(&o, &JsValue::from_str(k), v).unwrap();
    }
    o
}

fn str_array(items: &[&str]) -> Array {
    items.iter().map(|s| JsValue::from_str(s)).collect()
}

use std::fmt;

use js_sys::{Array, Object, Reflect};
use trva::isa::{ExtensionSet, Isa};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};

pub mod cores;
pub mod disassembly;
pub mod program;

pub use self::cores::{
    Core, DataBuilder, DataPoints, Description, Hazard3, Hazard3Config, Ibex, IbexConfig, Neorv32,
    PicoRV32Config, PicoRv32, PipelineTrace, REGISTER_NAMES,
};
use self::program::LoadError;

pub const RV32IM: Isa = Isa {
    exts: ExtensionSet::INTEGER_MULDIV,
    xlen: trva::isa::XLen::Rv32,
};

#[derive(Debug)]
pub enum TraceError {
    /// The program could not be assembled or fitted into the memory of the
    /// chip.
    Load(LoadError),
    Build(&'static str),
    /// The simulation failed.
    Run(String),
}

impl fmt::Display for TraceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TraceError::Load(err) => write!(f, "trace-err: {err}"),
            TraceError::Build(err) => write!(f, "trace-err[build]: {err}"),
            TraceError::Run(err) => write!(f, "trace-err[run]: {err}"),
        }
    }
}
impl std::error::Error for TraceError {}
impl From<LoadError> for TraceError {
    fn from(err: LoadError) -> Self {
        Self::Load(err)
    }
}

pub struct ReturnValue {
    pub instructions: Vec<String>,
    /// Only what the run actually collected, in no particular order.
    pub data: Vec<ReturnData>,
}

/// One collected data point. Each arm is tagged by `kind` on the JS side.
pub enum ReturnData {
    Pipeline(Pipeline),
    Registers(Registers),
    Symbols(Symbols),
}

pub struct Pipeline {
    pub traces: Vec<Vec<u32>>,
    pub keys: Vec<&'static str>,
    pub cycles: usize,
    /// Address of slot 1, so a slot can be mapped back to a symbol.
    pub text_addr: u32,
}

pub struct Registers {
    /// One list per register, one entry per cycle.
    pub values: Vec<Vec<u32>>,
    pub names: Vec<&'static str>,
    pub cycles: usize,
}

/// Where the assembler put every label. Parallel arrays, sorted by address.
pub struct Symbols {
    pub names: Vec<String>,
    pub addresses: Vec<u32>,
    pub sections: Vec<&'static str>,
    /// `1` for a label a `.global` directive exported.
    pub globals: Vec<u32>,
}

fn u32_arrays(rows: Vec<Vec<u32>>) -> Array {
    rows.into_iter()
        .map(|row| js_sys::Uint32Array::new_from_slice(&row))
        .collect()
}

impl ReturnData {
    fn into_object(self) -> Object {
        match self {
            ReturnData::Pipeline(p) => obj(&[
                ("kind", JsValue::from_str("pipeline")),
                ("traces", u32_arrays(p.traces).into()),
                ("keys", str_array(&p.keys).into()),
                ("cycles", p.cycles.into()),
                ("textAddr", p.text_addr.into()),
            ]),
            ReturnData::Symbols(s) => obj(&[
                ("kind", JsValue::from_str("symbols")),
                (
                    "names",
                    s.names
                        .iter()
                        .map(|n| JsValue::from_str(n))
                        .collect::<Array>()
                        .into(),
                ),
                (
                    "addresses",
                    js_sys::Uint32Array::new_from_slice(&s.addresses).into(),
                ),
                ("sections", str_array(&s.sections).into()),
                (
                    "globals",
                    js_sys::Uint32Array::new_from_slice(&s.globals).into(),
                ),
            ]),
            ReturnData::Registers(r) => obj(&[
                ("kind", JsValue::from_str("registers")),
                ("values", u32_arrays(r.values).into()),
                ("names", str_array(&r.names).into()),
                ("cycles", r.cycles.into()),
            ]),
        }
    }
}

impl ReturnValue {
    fn into_object(self) -> Object {
        obj(&[
            (
                "instructions",
                self.instructions
                    .iter()
                    .map(|s| JsValue::from_str(s))
                    .collect::<Array>()
                    .into(),
            ),
            (
                "data",
                self.data
                    .into_iter()
                    .map(|d| JsValue::from(d.into_object()))
                    .collect::<Array>()
                    .into(),
            ),
        ])
    }
}

pub fn evaluate<C: Core>(
    core: &C,
    params: &C::Params,
    source: &str,
    num_cycles: u32,
    points: DataPoints,
) -> Result<ReturnValue, TraceError> {
    evaluate_with_progress(core, params, source, num_cycles, points, &mut |_| {})
}

/// Like [`evaluate`], reporting the cycle count as it goes.
///
/// `progress` is called about a hundred times over the run, and once more with
/// the final count, so a long simulation can show how far along it is.
pub fn evaluate_with_progress<C: Core>(
    core: &C,
    params: &C::Params,
    source: &str,
    num_cycles: u32,
    points: DataPoints,
    progress: &mut dyn FnMut(u32),
) -> Result<ReturnValue, TraceError> {
    // Convert the program first: a bad program is reported far quicker than
    // the design compiles.
    let description = core.description(params);
    let sections = program::assemble(source, RV32IM, &description.memory_map())?;
    let instructions = program::disassemble(&sections.text);

    let (design, mut state) = core.compile(params, points)?;
    core.prologue(&design, &mut state, &sections)?;

    let mut builders = DataBuilder::for_points(
        points,
        description.pipeline_stages.len(),
        sections.text_addr,
        num_cycles,
    );
    // Often enough to look live, rarely enough not to dominate the run.
    let every = (num_cycles / 100).max(1);
    let mut cycles = num_cycles;
    for i in 0..num_cycles {
        core.iterate_cycle(&design, &mut state, &mut builders)?;
        if core.is_trapped(&design, &state) {
            cycles = i;
            break;
        }
        if i % every == 0 {
            progress(i);
        }
    }
    progress(cycles);

    let cycles = cycles as usize;
    let mut data: Vec<ReturnData> = builders
        .into_iter()
        .filter_map(|builder| match builder {
            DataBuilder::Pipeline(pt) => Some(ReturnData::Pipeline(Pipeline {
                traces: pt.stage_pcs,
                keys: description.pipeline_stages.clone(),
                cycles,
                text_addr: sections.text_addr,
            })),
            // A core that cannot read its register file never pushed a cycle,
            // so the point is dropped rather than reported as zeroes.
            DataBuilder::Registers(rt) if rt.values.iter().all(|v| v.is_empty()) => None,
            DataBuilder::Registers(rt) => Some(ReturnData::Registers(Registers {
                values: rt.values,
                names: REGISTER_NAMES.to_vec(),
                cycles,
            })),
        })
        .collect();

    // Static: it comes out of the assembler, not out of running the design.
    if points.contains(DataPoints::SYMBOLS) && !sections.symbols.is_empty() {
        data.push(ReturnData::Symbols(Symbols {
            names: sections.symbols.iter().map(|s| s.name.clone()).collect(),
            addresses: sections.symbols.iter().map(|s| s.addr).collect(),
            sections: sections.symbols.iter().map(|s| s.section).collect(),
            globals: sections
                .symbols
                .iter()
                .map(|s| u32::from(s.global))
                .collect(),
        }));
    }

    Ok(ReturnValue { instructions, data })
}

macro_rules! get_bool {
    ($config:expr, $option:literal) => {{
        Reflect::get(&$config, &JsValue::from_str($option))
            .map_err(|_| JsError::new(concat!("failed to access key '", $option, "'")))?
            .as_bool()
            .ok_or_else(|| JsError::new(concat!("key '", $option, "' is not a bool")))?
    }};
}

fn ibex_config(config: &Object) -> Result<IbexConfig, JsError> {
    Ok(IbexConfig {
        wb_stage: get_bool!(config, "wb_stage"),
    })
}

fn hazard3_config(config: &Object) -> Result<Hazard3Config, JsError> {
    Ok(Hazard3Config {
        extension_m: get_bool!(config, "extension_m"),
        mul_fast: get_bool!(config, "mul_fast"),
        mulh_fast: get_bool!(config, "mulh_fast"),
        muldiv_unroll_2: get_bool!(config, "muldiv_unroll_2"),
        reduced_bypass: get_bool!(config, "reduced_bypass"),
        branch_predictor: get_bool!(config, "branch_predictor"),
        fast_branchcmp: get_bool!(config, "fast_branchcmp"),
    })
}

fn picorv32_config(config: &Object) -> Result<PicoRV32Config, JsError> {
    Ok(PicoRV32Config {
        enable_mul: get_bool!(config, "enable_mul"),
        enable_div: get_bool!(config, "enable_div"),
        two_stage_shift: get_bool!(config, "two_stage_shift"),
        barrel_shifter: get_bool!(config, "barrel_shifter"),
        two_cycle_compare: get_bool!(config, "two_cycle_compare"),
        two_cycle_alu: get_bool!(config, "two_cycle_alu"),
        enable_fast_mul: get_bool!(config, "enable_fast_mul"),
    })
}

#[wasm_bindgen]
pub fn elf_to_js_assembly(bytes: &[u8], proc: &str, config: Object) -> Result<String, JsError> {
    let description = match proc {
        "picorv32" => PicoRv32.description(&picorv32_config(&config)?),
        "ibex" => Ibex.description(&ibex_config(&config)?),
        "neorv32" => Neorv32.description(&()),
        "hazard3" => Hazard3.description(&hazard3_config(&config)?),
        _ => return Err(JsError::new(&format!("unknown core '{proc}'"))),
    };
    let assembly = disassembly::elf_to_assembly(bytes, RV32IM, &description.memory_map())?;
    Ok(assembly)
}

#[wasm_bindgen]
pub fn get_js_ibex_trace(
    program: String,
    config: Object,
    num_cycles: u32,
    data_points: u32,
    on_progress: &js_sys::Function,
) -> Result<Object, JsError> {
    let config = ibex_config(&config)?;
    let mut report = |cycle: u32| {
        // A failed call just means no progress is shown.
        let _ = on_progress.call1(&JsValue::NULL, &JsValue::from(cycle));
    };
    let trace = evaluate_with_progress(
        &Ibex,
        &config,
        &program,
        num_cycles,
        DataPoints::from_bits(data_points),
        &mut report,
    )?;
    Ok(trace.into_object())
}

#[wasm_bindgen]
pub fn get_js_neorv32_trace(
    program: String,
    _config: Object,
    num_cycles: u32,
    data_points: u32,
    on_progress: &js_sys::Function,
) -> Result<Object, JsError> {
    let mut report = |cycle: u32| {
        // A failed call just means no progress is shown.
        let _ = on_progress.call1(&JsValue::NULL, &JsValue::from(cycle));
    };
    let trace = evaluate_with_progress(
        &Neorv32,
        &(),
        &program,
        num_cycles,
        DataPoints::from_bits(data_points),
        &mut report,
    )?;
    Ok(trace.into_object())
}

#[wasm_bindgen]
pub fn get_js_hazard3_trace(
    program: String,
    config: Object,
    num_cycles: u32,
    data_points: u32,
    on_progress: &js_sys::Function,
) -> Result<Object, JsError> {
    let config = hazard3_config(&config)?;
    let mut report = |cycle: u32| {
        // A failed call just means no progress is shown.
        let _ = on_progress.call1(&JsValue::NULL, &JsValue::from(cycle));
    };
    let trace = evaluate_with_progress(
        &Hazard3,
        &config,
        &program,
        num_cycles,
        DataPoints::from_bits(data_points),
        &mut report,
    )?;
    Ok(trace.into_object())
}

#[wasm_bindgen]
pub fn get_js_trace(
    program: String,
    config: Object,
    num_cycles: u32,
    data_points: u32,
    on_progress: &js_sys::Function,
) -> Result<Object, JsError> {
    let config = picorv32_config(&config)?;
    let mut report = |cycle: u32| {
        // A failed call just means no progress is shown.
        let _ = on_progress.call1(&JsValue::NULL, &JsValue::from(cycle));
    };
    let trace = evaluate_with_progress(
        &PicoRv32,
        &config,
        &program,
        num_cycles,
        DataPoints::from_bits(data_points),
        &mut report,
    )?;
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

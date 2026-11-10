use std::fmt;

use js_sys::{Array, Object, Reflect};
use trva::isa::{ExtensionSet, Isa};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsError, JsValue};

pub mod cores;
pub mod program;

pub use self::cores::{
    Core, Description, Hazard3, Hazard3Config, Ibex, IbexConfig, Neorv32, PicoRV32Config, PicoRv32,
    PipelineTrace,
};
use self::program::LoadError;
pub use self::program::Program;

pub const RV32IM: Isa = Isa {
    exts: ExtensionSet::INTEGER_MULDIV,
    xlen: trva::isa::XLen::Rv32,
};

#[derive(Debug)]
pub enum TraceError {
    /// The program could not be assembled, read, or fitted into the memory of
    /// the chip.
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
    pub pipeline: Pipeline,
}
pub struct Pipeline {
    pub traces: Vec<Vec<u32>>,
    pub keys: Vec<&'static str>,
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
            ("keys", str_array(&self.pipeline.keys).into()),
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

pub fn evaluate<C: Core>(
    core: &C,
    params: &C::Params,
    program: &Program,
    num_cycles: u32,
) -> Result<ReturnValue, TraceError> {
    // Convert the program first: a bad program is reported far quicker than
    // the design compiles.
    let map = core.memory_map();
    let sections = program.to_sections(RV32IM, &map)?;
    let instructions = program::disassemble(&sections.text);

    let description = core.description(params);
    let (design, mut state) = core.compile(params)?;
    core.prologue(&design, &mut state, &sections)?;

    let mut trace = PipelineTrace::new(map.boot, description.pipeline_stages.len(), num_cycles);
    let mut cycles = num_cycles;
    for i in 0..num_cycles {
        core.iterate_cycle(&design, &mut state, &mut trace)?;
        if core.is_trapped(&design, &state) {
            cycles = i;
            break;
        }
    }

    Ok(ReturnValue {
        instructions,
        pipeline: Pipeline {
            traces: trace.stage_pcs,
            keys: description.pipeline_stages,
            cycles: cycles as usize,
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

/// The program the web app hands over: assembly as a string, or the bytes of
/// an ELF executable as a `Uint8Array`.
fn js_program(program: &JsValue) -> Result<Program, JsError> {
    if let Some(assembly) = program.as_string() {
        return Ok(Program::Assembly(assembly.into()));
    }
    if let Some(bytes) = program.dyn_ref::<js_sys::Uint8Array>() {
        return Ok(Program::Elf(bytes.to_vec().into()));
    }
    Err(JsError::new(
        "the program is neither assembly text nor the bytes of an ELF file",
    ))
}

#[wasm_bindgen]
pub fn get_js_ibex_trace(
    program: JsValue,
    config: Object,
    num_cycles: u32,
) -> Result<Object, JsError> {
    let config = IbexConfig {
        wb_stage: get_bool!(config, "wb_stage"),
    };
    let trace = evaluate(&Ibex, &config, &js_program(&program)?, num_cycles)?;
    Ok(trace.into_object())
}

#[wasm_bindgen]
pub fn get_js_neorv32_trace(
    program: JsValue,
    _config: Object,
    num_cycles: u32,
) -> Result<Object, JsError> {
    let trace = evaluate(&Neorv32, &(), &js_program(&program)?, num_cycles)?;
    Ok(trace.into_object())
}

#[wasm_bindgen]
pub fn get_js_hazard3_trace(
    program: JsValue,
    config: Object,
    num_cycles: u32,
) -> Result<Object, JsError> {
    let config = Hazard3Config {
        extension_m: get_bool!(config, "extension_m"),
        mul_fast: get_bool!(config, "mul_fast"),
        mulh_fast: get_bool!(config, "mulh_fast"),
        muldiv_unroll_2: get_bool!(config, "muldiv_unroll_2"),
        reduced_bypass: get_bool!(config, "reduced_bypass"),
        branch_predictor: get_bool!(config, "branch_predictor"),
        fast_branchcmp: get_bool!(config, "fast_branchcmp"),
    };
    let trace = evaluate(&Hazard3, &config, &js_program(&program)?, num_cycles)?;
    Ok(trace.into_object())
}

#[wasm_bindgen]
pub fn get_js_trace(program: JsValue, config: Object, num_cycles: u32) -> Result<Object, JsError> {
    let config = PicoRV32Config {
        enable_mul: get_bool!(config, "enable_mul"),
        enable_div: get_bool!(config, "enable_div"),
        two_stage_shift: get_bool!(config, "two_stage_shift"),
        barrel_shifter: get_bool!(config, "barrel_shifter"),
        two_cycle_compare: get_bool!(config, "two_cycle_compare"),
        two_cycle_alu: get_bool!(config, "two_cycle_alu"),
        enable_fast_mul: get_bool!(config, "enable_fast_mul"),
    };
    let trace = evaluate(&PicoRv32, &config, &js_program(&program)?, num_cycles)?;
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

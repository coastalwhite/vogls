//! `wasm-bindgen` entry points for the designs bundled into the webapp.
//!
//! Designs that are not bundled reach the webapp as uploaded plugins instead;
//! see the `pipeline-explorer-plugin` crate for that path.

use js_sys::{Array, Object, Reflect};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};

use crate::ReturnValue;

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

macro_rules! get_bool {
    ($config:expr, $option:literal) => {{
        Reflect::get(&$config, &JsValue::from_str($option))
            .map_err(|_| JsError::new(concat!("failed to access key '", $option, "'")))?
            .as_bool()
            .ok_or_else(|| JsError::new(concat!("key '", $option, "' is not a bool")))?
    }};
}

#[cfg(feature = "ibex")]
#[wasm_bindgen]
pub fn get_js_ibex_trace(
    assembly: &str,
    config: Object,
    num_cycles: u32,
) -> Result<Object, JsError> {
    let config = crate::IbexConfig {
        wb_stage: get_bool!(config, "wb_stage"),
    };
    let trace = crate::get_ibex_trace(assembly, num_cycles, &config)?;
    Ok(trace.into_object())
}

#[cfg(feature = "neorv32")]
#[wasm_bindgen]
pub fn get_js_neorv32_trace(
    assembly: &str,
    _config: Object,
    num_cycles: u32,
) -> Result<Object, JsError> {
    let trace = crate::get_neorv32_trace(assembly, num_cycles)?;
    Ok(trace.into_object())
}

#[cfg(feature = "hazard3")]
#[wasm_bindgen]
pub fn get_js_hazard3_trace(
    assembly: &str,
    config: Object,
    num_cycles: u32,
) -> Result<Object, JsError> {
    let config = crate::Hazard3Config {
        extension_m: get_bool!(config, "extension_m"),
        mul_fast: get_bool!(config, "mul_fast"),
        mulh_fast: get_bool!(config, "mulh_fast"),
        muldiv_unroll_2: get_bool!(config, "muldiv_unroll_2"),
        reduced_bypass: get_bool!(config, "reduced_bypass"),
        branch_predictor: get_bool!(config, "branch_predictor"),
        fast_branchcmp: get_bool!(config, "fast_branchcmp"),
    };
    // The bundled entry is the plain design: mnemonics beyond the base ISA are
    // something a plugin brings with it.
    let trace = crate::get_hazard3_trace(assembly, num_cycles, &config, &[])?;
    Ok(trace.into_object())
}

#[cfg(feature = "picorv32")]
#[wasm_bindgen]
pub fn get_js_trace(assembly: &str, config: Object, num_cycles: u32) -> Result<Object, JsError> {
    let config = crate::PicoRV32Config {
        enable_mul: get_bool!(config, "enable_mul"),
        enable_div: get_bool!(config, "enable_div"),
        two_stage_shift: get_bool!(config, "two_stage_shift"),
        barrel_shifter: get_bool!(config, "barrel_shifter"),
        two_cycle_compare: get_bool!(config, "two_cycle_compare"),
        two_cycle_alu: get_bool!(config, "two_cycle_alu"),
        enable_fast_mul: get_bool!(config, "enable_fast_mul"),
    };
    let trace = crate::get_trace(assembly, &config, num_cycles)?;
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

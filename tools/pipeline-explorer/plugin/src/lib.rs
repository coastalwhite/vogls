//! Host ABI for uploadable pipeline-explorer design plugins.
//!
//! A plugin is a `wasm32-unknown-unknown` `cdylib` that carries one design
//! (its Verilog sources plus the code that pulls a pipeline trace out of it)
//! and exports the handful of `pe_*` functions below. The webapp instantiates
//! such a module with an empty import object, so a plugin must not depend on
//! `wasm-bindgen` or any other host glue.
//!
//! # Exports
//!
//! | export | meaning |
//! | --- | --- |
//! | `memory` | the module's linear memory (emitted by rustc) |
//! | `pe_abi_version() -> u32` | must equal [`ABI_VERSION`] |
//! | `pe_alloc(len) -> ptr` | allocate `len` bytes for the host to write into |
//! | `pe_dealloc(ptr, len)` | free a [`pe_alloc`](alloc) block |
//! | `pe_manifest() -> ptr` | a host buffer holding the UTF-8 manifest JSON |
//! | `pe_run(asm_ptr, asm_len, cfg_ptr, cfg_len, cycles) -> ptr` | a host buffer holding an encoded [`Trace`] |
//! | `pe_free_buffer(ptr)` | free a host buffer returned by the two above |
//!
//! [`export_plugin!`] emits all of them; a plugin only supplies a manifest
//! string and a `run` function.
//!
//! # Host buffers
//!
//! Every pointer handed back to the host points at a little-endian `u32`
//! payload length followed by that many payload bytes. The host reads the
//! payload out of `memory` and then returns the pointer to `pe_free_buffer`.
//!
//! # Manifest
//!
//! The manifest is a JSON object describing the design and the knobs the UI
//! should render for it:
//!
//! ```json
//! {
//!   "abi": 1,
//!   "id": "hazard3",
//!   "name": "Hazard3",
//!   "fields": [
//!     { "id": "extension_m", "type": "checkbox", "title": "Enable M Extension", "default": true }
//!   ],
//!   "instructions": ["square", "l1"]
//! }
//! ```
//!
//! `instructions` is optional and names the mnemonics this design accepts on top
//! of the base ISA, so the webapp's editor highlights them like any other
//! instruction. It is cosmetic: what makes them assemble is the plugin
//! registering them with its assembler, which [`custom`] covers.
//!
//! Field types are `checkbox` (a bool) and `number`. The host passes the
//! configuration to `pe_run` as an array of little-endian `u32`s holding one
//! entry per manifest field, in manifest order: `0`/`1` for a `checkbox` and
//! the raw value for a `number`. Keeping the encoding positional is what lets
//! a plugin stay free of a JSON parser.
//!
//! # Trace encoding
//!
//! `pe_run`'s payload is little-endian and byte-packed (the host reads it with
//! a `DataView`, so nothing is alignment-sensitive):
//!
//! ```text
//! u32 status                      // 0 = ok, 1 = error
//! // status == 1:
//! u32 len, len bytes              // UTF-8 error message
//! // status == 0:
//! u32 cycles                      // cycles worth of trace the UI should show
//! u32 n_instructions
//!   repeated: u32 len, len bytes  // disassembly, one per instruction slot
//! u32 n_stages
//!   repeated: u32 len, len bytes  // stage name
//!             u32 trace_len, trace_len * u32
//! ```
//!
//! A trace entry is `0` when the stage is idle that cycle, and otherwise the
//! 1-based index of the instruction slot occupying the stage.

use std::alloc::Layout;

pub mod custom;

/// The assembler, re-exported so a plugin defining a mnemonic of its own does
/// not have to depend on it separately. See [`custom`].
pub use trva;

/// ABI version implemented by this crate.
///
/// The host refuses to load a plugin whose `pe_abi_version` differs.
pub const ABI_VERSION: u32 = 1;

/// One design's pipeline trace, as handed back to the webapp.
pub struct Trace {
    /// Disassembly of the program, indexed by instruction slot.
    pub instructions: Vec<String>,
    /// Stage names, outermost (fetch) first.
    pub stages: Vec<String>,
    /// Per stage, the occupying instruction slot for each cycle (`0` = idle).
    pub traces: Vec<Vec<u32>>,
    /// Number of cycles the UI should display.
    pub cycles: u32,
}

/// Signature of a plugin's entry point: assembly, the positional config
/// described by the manifest, and a cycle budget.
pub type RunFn = fn(&str, &[u32], u32) -> Result<Trace, String>;

/// Allocates `len` bytes for the host to write into.
///
/// The block must be released with [`dealloc`] using the same `len`.
pub fn alloc(len: usize) -> usize {
    if len == 0 {
        // `Layout` rejects a zero size, and there is nothing to write anyway.
        return 1;
    }
    let layout = Layout::from_size_align(len, 1).unwrap();
    // SAFETY: `layout` has a non-zero size.
    let ptr = unsafe { std::alloc::alloc(layout) };
    if ptr.is_null() {
        std::alloc::handle_alloc_error(layout);
    }
    ptr as usize
}

/// Releases a block obtained from [`alloc`].
///
/// # Safety
///
/// `ptr` must come from [`alloc`] and `len` must be the length passed to it.
pub unsafe fn dealloc(ptr: usize, len: usize) {
    if len == 0 {
        return;
    }
    let layout = Layout::from_size_align(len, 1).unwrap();
    // SAFETY: the caller guarantees `ptr`/`len` describe a live `alloc` block.
    unsafe { std::alloc::dealloc(ptr as *mut u8, layout) }
}

/// Wraps `payload` in the length-prefixed layout the host expects and leaks it.
///
/// The host reads the payload and then hands the pointer to [`free_buffer`].
pub fn into_host_buffer(payload: &[u8]) -> usize {
    let mut buf = Vec::with_capacity(4 + payload.len());
    buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    buf.extend_from_slice(payload);
    // `with_capacity` reserved exactly what we filled, so this does not
    // reallocate and the block's size stays recoverable from the prefix.
    debug_assert_eq!(buf.len(), buf.capacity());
    Box::into_raw(buf.into_boxed_slice()) as *mut u8 as usize
}

/// Releases a buffer produced by [`into_host_buffer`].
///
/// # Safety
///
/// `ptr` must come from [`into_host_buffer`] and must not have been freed yet.
pub unsafe fn free_buffer(ptr: usize) {
    let ptr = ptr as *mut u8;
    // SAFETY: the caller guarantees `ptr` still points at a host buffer, whose
    // first four bytes are the payload length.
    let len = unsafe { std::slice::from_raw_parts(ptr, 4) };
    let len = u32::from_le_bytes(len.try_into().unwrap()) as usize;
    // SAFETY: as above; the allocation is exactly the prefix plus payload.
    drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, 4 + len)) });
}

fn push_str(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u32).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
}

/// Encodes a successful run in the format documented at the crate root.
pub fn encode_trace(trace: &Trace) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&trace.cycles.to_le_bytes());

    out.extend_from_slice(&(trace.instructions.len() as u32).to_le_bytes());
    for instruction in &trace.instructions {
        push_str(&mut out, instruction);
    }

    out.extend_from_slice(&(trace.stages.len() as u32).to_le_bytes());
    for (stage, values) in trace.stages.iter().zip(&trace.traces) {
        push_str(&mut out, stage);
        out.extend_from_slice(&(values.len() as u32).to_le_bytes());
        for value in values {
            out.extend_from_slice(&value.to_le_bytes());
        }
    }

    out
}

/// Encodes a failed run in the format documented at the crate root.
pub fn encode_error(message: &str) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&1u32.to_le_bytes());
    push_str(&mut out, message);
    out
}

/// Body of the generated `pe_run`: decodes the host's arguments, calls `run`,
/// and encodes whatever comes back.
///
/// # Safety
///
/// `asm_ptr`/`asm_len` and `cfg_ptr`/`cfg_len` must describe blocks the host
/// obtained from [`alloc`] and filled in; `cfg_len` counts `u32`s, not bytes.
pub unsafe fn run_abi(
    asm_ptr: usize,
    asm_len: usize,
    cfg_ptr: usize,
    cfg_len: usize,
    num_cycles: u32,
    run: RunFn,
) -> usize {
    // SAFETY: the caller guarantees this is a live block of `asm_len` bytes.
    let asm = unsafe { std::slice::from_raw_parts(asm_ptr as *const u8, asm_len) };
    // SAFETY: as above, for `4 * cfg_len` bytes. Read byte-wise so the block
    // does not have to be `u32`-aligned.
    let cfg = unsafe { std::slice::from_raw_parts(cfg_ptr as *const u8, 4 * cfg_len) };
    let cfg: Vec<u32> = cfg
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect();

    let payload = match std::str::from_utf8(asm) {
        Err(_) => encode_error("assembly is not valid UTF-8"),
        Ok(asm) => match run(asm, &cfg, num_cycles) {
            Ok(trace) => encode_trace(&trace),
            Err(err) => encode_error(&err),
        },
    };
    into_host_buffer(&payload)
}

/// Reads manifest field `index` as a bool, defaulting to `false` when the host
/// sent a shorter configuration than the manifest declares.
pub fn cfg_bool(cfg: &[u32], index: usize) -> bool {
    cfg.get(index).is_some_and(|&v| v != 0)
}

/// Reads manifest field `index` as a number, defaulting to `0`.
pub fn cfg_u32(cfg: &[u32], index: usize) -> u32 {
    cfg.get(index).copied().unwrap_or(0)
}

/// Emits the `pe_*` exports for a plugin.
///
/// `manifest` is a `&'static str` holding the manifest JSON and `run` is a
/// [`RunFn`].
///
/// ```ignore
/// pipeline_explorer_plugin::export_plugin!(
///     manifest = MANIFEST,
///     run = run,
/// );
/// ```
///
/// The exports only exist on `wasm32`, where a pointer is exactly the `u32`
/// the ABI passes around. That keeps a plugin crate buildable for the host —
/// useful for unit tests and for tools that call `run` directly — without
/// exposing entry points whose pointers would be truncated there.
#[macro_export]
macro_rules! export_plugin {
    (manifest = $manifest:expr, run = $run:expr $(,)?) => {
        #[cfg(target_arch = "wasm32")]
        #[unsafe(no_mangle)]
        pub extern "C" fn pe_abi_version() -> u32 {
            $crate::ABI_VERSION
        }

        #[cfg(target_arch = "wasm32")]
        #[unsafe(no_mangle)]
        pub extern "C" fn pe_alloc(len: u32) -> u32 {
            $crate::alloc(len as usize) as u32
        }

        /// # Safety
        ///
        /// `ptr`/`len` must describe a live `pe_alloc` block.
        #[cfg(target_arch = "wasm32")]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn pe_dealloc(ptr: u32, len: u32) {
            unsafe { $crate::dealloc(ptr as usize, len as usize) }
        }

        /// # Safety
        ///
        /// `ptr` must be a host buffer that has not been freed yet.
        #[cfg(target_arch = "wasm32")]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn pe_free_buffer(ptr: u32) {
            unsafe { $crate::free_buffer(ptr as usize) }
        }

        #[cfg(target_arch = "wasm32")]
        #[unsafe(no_mangle)]
        pub extern "C" fn pe_manifest() -> u32 {
            $crate::into_host_buffer(($manifest as &str).as_bytes()) as u32
        }

        /// # Safety
        ///
        /// Both pointers must describe live `pe_alloc` blocks; `cfg_len`
        /// counts `u32`s.
        #[cfg(target_arch = "wasm32")]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn pe_run(
            asm_ptr: u32,
            asm_len: u32,
            cfg_ptr: u32,
            cfg_len: u32,
            num_cycles: u32,
        ) -> u32 {
            unsafe {
                $crate::run_abi(
                    asm_ptr as usize,
                    asm_len as usize,
                    cfg_ptr as usize,
                    cfg_len as usize,
                    num_cycles,
                    $run as $crate::RunFn,
                ) as u32
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads a host buffer back the way the webapp's loader does.
    fn take_host_buffer(ptr: usize) -> Vec<u8> {
        let base = ptr as *const u8;
        let len = u32::from_le_bytes(
            unsafe { std::slice::from_raw_parts(base, 4) }
                .try_into()
                .unwrap(),
        );
        let payload = unsafe { std::slice::from_raw_parts(base.add(4), len as usize) }.to_vec();
        unsafe { free_buffer(ptr) };
        payload
    }

    #[test]
    fn host_buffer_round_trips() {
        let ptr = into_host_buffer(b"hello");
        assert_eq!(take_host_buffer(ptr), b"hello");
    }

    #[test]
    fn empty_host_buffer_round_trips() {
        let ptr = into_host_buffer(b"");
        assert_eq!(take_host_buffer(ptr), b"");
    }

    #[test]
    fn alloc_round_trips() {
        let ptr = alloc(8);
        unsafe { std::ptr::write_bytes(ptr as *mut u8, 0xab, 8) };
        assert_eq!(
            unsafe { std::slice::from_raw_parts(ptr as *const u8, 8) },
            [0xab; 8]
        );
        unsafe { dealloc(ptr, 8) };
        // A zero-length request is legal and must survive being freed.
        unsafe { dealloc(alloc(0), 0) };
    }

    #[test]
    fn trace_encoding_is_laid_out_as_documented() {
        let trace = Trace {
            instructions: vec!["addi x1, x0, 1".into()],
            stages: vec!["F".into(), "X".into()],
            traces: vec![vec![1, 0], vec![0, 1]],
            cycles: 2,
        };
        let encoded = encode_trace(&trace);
        let u32_at = |off: usize| u32::from_le_bytes(encoded[off..off + 4].try_into().unwrap());

        assert_eq!(u32_at(0), 0, "status");
        assert_eq!(u32_at(4), 2, "cycles");
        assert_eq!(u32_at(8), 1, "instruction count");
        assert_eq!(u32_at(12), 14, "instruction length");
        assert_eq!(&encoded[16..30], b"addi x1, x0, 1");
        assert_eq!(u32_at(30), 2, "stage count");
        assert_eq!(u32_at(34), 1, "stage name length");
        assert_eq!(&encoded[38..39], b"F");
        assert_eq!(u32_at(39), 2, "trace length");
        assert_eq!(u32_at(43), 1);
        assert_eq!(u32_at(47), 0);
        assert_eq!(encoded.len(), 51 + 4 + 1 + 4 + 8);
    }

    #[test]
    fn error_encoding_is_tagged() {
        let encoded = encode_error("boom");
        assert_eq!(u32::from_le_bytes(encoded[0..4].try_into().unwrap()), 1);
        assert_eq!(&encoded[8..12], b"boom");
    }

    /// Drives `run_abi` exactly as the webapp's loader does: stage the inputs
    /// in `alloc` blocks, call in, read the host buffer back out.
    fn call_run_abi(asm: &str, cfg: &[u32], num_cycles: u32, run: RunFn) -> Vec<u8> {
        let asm_ptr = alloc(asm.len());
        unsafe { std::ptr::copy_nonoverlapping(asm.as_ptr(), asm_ptr as *mut u8, asm.len()) };

        let cfg_bytes: Vec<u8> = cfg.iter().flat_map(|v| v.to_le_bytes()).collect();
        let cfg_ptr = alloc(cfg_bytes.len());
        unsafe {
            std::ptr::copy_nonoverlapping(cfg_bytes.as_ptr(), cfg_ptr as *mut u8, cfg_bytes.len())
        };

        let out = unsafe { run_abi(asm_ptr, asm.len(), cfg_ptr, cfg.len(), num_cycles, run) };
        unsafe { dealloc(asm_ptr, asm.len()) };
        unsafe { dealloc(cfg_ptr, cfg_bytes.len()) };
        take_host_buffer(out)
    }

    #[test]
    fn run_abi_forwards_arguments_and_encodes_the_trace() {
        fn run(asm: &str, cfg: &[u32], num_cycles: u32) -> Result<Trace, String> {
            assert_eq!(asm, "nop");
            assert_eq!(cfg, [1, 0, 5]);
            assert_eq!(num_cycles, 42);
            Ok(Trace {
                instructions: vec!["nop".into()],
                stages: vec!["F".into()],
                traces: vec![vec![1]],
                cycles: 1,
            })
        }
        let payload = call_run_abi("nop", &[1, 0, 5], 42, run);
        assert_eq!(payload, encode_trace(&run("nop", &[1, 0, 5], 42).unwrap()));
    }

    #[test]
    fn run_abi_encodes_a_failed_run() {
        fn run(_: &str, _: &[u32], _: u32) -> Result<Trace, String> {
            Err("no such signal".into())
        }
        let payload = call_run_abi("nop", &[], 1, run);
        assert_eq!(payload, encode_error("no such signal"));
    }

    #[test]
    fn config_accessors_tolerate_a_short_config() {
        assert!(cfg_bool(&[1], 0));
        assert!(!cfg_bool(&[1], 3));
        assert_eq!(cfg_u32(&[7], 0), 7);
        assert_eq!(cfg_u32(&[7], 3), 0);
    }
}

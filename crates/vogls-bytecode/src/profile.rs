#[derive(Default)]
pub struct BytecodeDebugInfo {
    pub instrs: Vec<VirInstr>,
    pub regions: Vec<RegionInfo>,
}

pub struct VirInstr {
    pub region: u32,
    pub text: String,
    pub start: u32,
    pub len: u32,
}

pub struct RegionInfo {
    pub label: String,
}

impl BytecodeDebugInfo {
    pub fn push_region(&mut self, label: String) -> u32 {
        let id = self.regions.len() as u32;
        self.regions.push(RegionInfo { label });
        id
    }

    pub fn push_instr(&mut self, region: u32, text: String, start: usize, end: usize) {
        self.instrs.push(VirInstr {
            region,
            text,
            start: start as u32,
            len: (end - start) as u32,
        });
    }
}

#[cfg(feature = "profile")]
pub use profiler::SamplingProfiler;

#[cfg(feature = "profile")]
mod profiler {
    use std::fmt::Write as _;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;

    use crate::{Bytecode, BytecodeListeners, BytecodeOpcode, Regs, Schedule, Tracer};
    use vogls_runtime::RuntimeState;

    static SAMPLES_PENDING: AtomicU32 = AtomicU32::new(0);
    extern "C" fn sigprof_handler(_sig: libc::c_int) {
        SAMPLES_PENDING.fetch_add(1, Ordering::Relaxed);
    }

    impl BytecodeDebugInfo {
        fn vir_at(&self, pc: u64) -> Option<usize> {
            if pc > u32::MAX as u64 {
                return None;
            }
            let pc = pc as u32;
            // First instruction whose `start` is strictly greater than `pc`; the
            // candidate that may contain `pc` is the one just before it.
            let idx = self.instrs.partition_point(|i| i.start <= pc);
            let cand = idx.checked_sub(1)?;
            let instr = &self.instrs[cand];
            (pc < instr.start.saturating_add(instr.len)).then_some(cand)
        }
    }

    pub struct SamplingProfiler {
        current_pc: u64,
        samples: Vec<u64>,

        debug_info: Option<Arc<BytecodeDebugInfo>>,
        output: PathBuf,

        /// Sampling period in microseconds of process CPU time.
        interval_us: i64,
    }

    impl SamplingProfiler {
        pub fn new(debug_info: Option<Arc<BytecodeDebugInfo>>, output: &Path) -> Self {
            let interval_us = std::env::var("VOGLS_PROFILE_INTERVAL_US")
                .ok()
                .and_then(|v| v.parse::<i64>().ok())
                .filter(|&v| v > 0)
                .unwrap_or(1000);

            let output = output.to_path_buf();
            Self {
                current_pc: u64::MAX,
                debug_info,
                output,
                samples: Vec::new(),
                interval_us,
            }
        }

        fn start(&mut self) {
            SAMPLES_PENDING.store(0, Ordering::Relaxed);

            // SAFETY: `sigaction` with a zeroed struct plus our async-signal-safe
            // handler is a standard POSIX installation.
            unsafe {
                let mut sa: libc::sigaction = std::mem::zeroed();
                sa.sa_sigaction = sigprof_handler as *const () as usize;
                libc::sigemptyset(&mut sa.sa_mask);
                sa.sa_flags = libc::SA_RESTART;
                libc::sigaction(libc::SIGPROF, &sa, std::ptr::null_mut());
            }

            let it = libc::itimerval {
                it_interval: libc::timeval {
                    tv_sec: 0,
                    tv_usec: self.interval_us as libc::suseconds_t,
                },
                it_value: libc::timeval {
                    tv_sec: 0,
                    tv_usec: self.interval_us as libc::suseconds_t,
                },
            };

            // SAFETY: `it` is a fully-initialised itimerval.
            unsafe {
                libc::setitimer(libc::ITIMER_PROF, &it, std::ptr::null_mut());
            }
        }

        /// Disarm the timer and detach the sample buffer from the handler.
        fn stop(&mut self) {
            // SAFETY: a zeroed itimerval disarms the timer.
            unsafe {
                let it: libc::itimerval = std::mem::zeroed();
                libc::setitimer(libc::ITIMER_PROF, &it, std::ptr::null_mut());
            }
        }

        /// Aggregate everything and produce (stderr summary, full report text).
        fn report(&self, code: &[Bytecode]) {
            let total = self.samples.len() as u64;
            let mut unattributed = 0u64;
            let mut counts = vec![0u64; code.len()];
            for sample in &self.samples {
                match counts.get_mut(*sample as usize) {
                    None => unattributed += 1,
                    Some(c) => *c += 1,
                }
            }
            let attributed = total - unattributed;

            // --- per opcode ---------------------------------------------------
            let mut per_opcode = [0u64; 256];
            for (pc, &c) in counts.iter().enumerate() {
                if c != 0 {
                    per_opcode[code[pc].opcode() as usize] += c;
                }
            }
            let mut opcode_rows: Vec<(u8, u64)> = (0u16..256)
                .map(|op| op as u8)
                .filter(|&op| per_opcode[op as usize] != 0)
                .map(|op| (op, per_opcode[op as usize]))
                .collect();
            opcode_rows.sort_by_key(|a| std::cmp::Reverse(a.1));

            // --- per bytecode instruction (PC) --------------------------------
            let mut pc_rows: Vec<(usize, u64)> = counts
                .iter()
                .enumerate()
                .filter(|&(_, &c)| c != 0)
                .map(|(pc, &c)| (pc, c))
                .collect();
            pc_rows.sort_by_key(|a| std::cmp::Reverse(a.1));

            // --- per VIR instruction & per temporal region --------------------
            let mut vir_counts: Vec<u64> = Vec::new();
            let mut region_counts: Vec<u64> = Vec::new();
            if let Some(dbg) = &self.debug_info {
                vir_counts = vec![0u64; dbg.instrs.len()];
                region_counts = vec![0u64; dbg.regions.len()];
                for (pc, &c) in counts.iter().enumerate() {
                    if c == 0 {
                        continue;
                    }
                    if let Some(vi) = dbg.vir_at(pc as u64) {
                        vir_counts[vi] += c;
                        region_counts[dbg.instrs[vi].region as usize] += c;
                    }
                }
            }

            // Build the full textual report.
            let mut out = String::new();
            let _ = writeln!(out, "; VOGLS bytecode sampling profile");
            let _ = writeln!(
                out,
                "; samples: {total} total, {attributed} attributed, {unattributed} unattributed",
            );
            let _ = writeln!(out, "; sampling interval: {}us CPU time", self.interval_us);
            let _ = writeln!(out, ";");

            // Opcode table.
            let _ = writeln!(out, "; ===== per opcode =====");
            let _ = writeln!(out, "; {:>10}  {:>7}  opcode", "samples", "percent");
            for (op, c) in &opcode_rows {
                let name = BytecodeOpcode::try_from(*op)
                    .map(|o| o.into_static_str())
                    .unwrap_or("<unknown>");
                let _ = writeln!(out, "; {:>10}  {:>6.2}%  {}", c, pct(*c, total), name);
            }
            let _ = writeln!(out, ";");

            // Temporal region table.
            if let Some(dbg) = &self.debug_info {
                let mut rows: Vec<(usize, u64)> = region_counts
                    .iter()
                    .enumerate()
                    .filter(|&(_, &c)| c != 0)
                    .map(|(i, &c)| (i, c))
                    .collect();
                rows.sort_by_key(|a| std::cmp::Reverse(a.1));
                let _ = writeln!(out, "; ===== per temporal region =====");
                let _ = writeln!(out, "; {:>10}  {:>7}  region", "samples", "percent");
                for (i, c) in &rows {
                    let _ = writeln!(
                        out,
                        "; {:>10}  {:>6.2}%  {}",
                        c,
                        pct(*c, total),
                        dbg.regions[*i].label
                    );
                }
                let _ = writeln!(out, ";");
            }

            // Per VIR instruction table.
            if let Some(dbg) = &self.debug_info {
                let mut rows: Vec<(usize, u64)> = vir_counts
                    .iter()
                    .enumerate()
                    .filter(|&(_, &c)| c != 0)
                    .map(|(i, &c)| (i, c))
                    .collect();
                rows.sort_by_key(|a| std::cmp::Reverse(a.1));
                let _ = writeln!(out, "; ===== per VIR instruction (hottest first) =====");
                let _ = writeln!(out, "; {:>10}  {:>7}  vir", "samples", "percent");
                for (i, c) in &rows {
                    let _ = writeln!(
                        out,
                        "; {:>10}  {:>6.2}%  {}",
                        c,
                        pct(*c, total),
                        dbg.instrs[*i].text.trim()
                    );
                }
                let _ = writeln!(out, ";");
            }

            // Per bytecode instruction table.
            let _ = writeln!(
                out,
                "; ===== per bytecode instruction (hottest first) ====="
            );
            let _ = writeln!(
                out,
                "; {:>8}  {:>10}  {:>7}  instruction",
                "pc", "samples", "percent"
            );
            for (pc, c) in &pc_rows {
                let _ = writeln!(
                    out,
                    "; {:>8}  {:>10}  {:>6.2}%  {}",
                    pc,
                    c,
                    pct(*c, total),
                    code[*pc]
                );
            }
            let _ = writeln!(out, ";");

            // Annotated VIR listing.
            if let Some(dbg) = &self.debug_info {
                let _ = writeln!(out, "; ===== annotated VIR =====");
                let mut cur_region: Option<u32> = None;
                for (i, instr) in dbg.instrs.iter().enumerate() {
                    if cur_region != Some(instr.region) {
                        cur_region = Some(instr.region);
                        let rc = region_counts
                            .get(instr.region as usize)
                            .copied()
                            .unwrap_or(0);
                        let _ = writeln!(out);
                        let _ = writeln!(
                            out,
                            "; --- region: {}  [{} samples, {:.2}%] ---",
                            dbg.regions[instr.region as usize].label,
                            rc,
                            pct(rc, total),
                        );
                    }
                    let c = vir_counts.get(i).copied().unwrap_or(0);
                    let end = instr.start.saturating_add(instr.len);
                    let _ = writeln!(
                        out,
                        "  {:<50} ; {:>8} ({:>5.2}%)  pc {}..{}",
                        instr.text.trim(),
                        c,
                        pct(c, total),
                        instr.start,
                        end,
                    );
                }
                let _ = writeln!(out);
            }

            // Write the full report to the output file.
            let path = &self.output;
            match std::fs::write(path, out.as_bytes()) {
                Ok(()) => eprintln!("vogls: wrote sampling profile to {}", path.display()),
                Err(e) => eprintln!("vogls: failed to write profile to {}: {e}", path.display()),
            }

            // Print the per-opcode summary table (plus a compact TR table) to
            // stderr so the headline result is visible without opening the file.
            eprintln!();
            eprintln!(
                "=== sampling profile: {total} samples ({attributed} attributed, {unattributed} unattributed) ==="
            );
            let name_w = opcode_rows
                .iter()
                .map(|(op, _)| {
                    BytecodeOpcode::try_from(*op)
                        .map(|o| o.into_static_str().len())
                        .unwrap_or(9)
                })
                .max()
                .unwrap_or(6)
                .max(6);
            eprintln!(
                "| {0:<1$} | {2:>10} | {3:>7} |",
                "opcode", name_w, "samples", "percent"
            );
            for (op, c) in &opcode_rows {
                let name = BytecodeOpcode::try_from(*op)
                    .map(|o| o.into_static_str())
                    .unwrap_or("<unknown>");
                eprintln!(
                    "| {0:<1$} | {2:>10} | {3:>6.2}% |",
                    name,
                    name_w,
                    c,
                    pct(*c, total)
                );
            }
        }
    }

    impl Tracer for SamplingProfiler {
        fn pre_exec(
            &mut self,
            _i: Bytecode,
            _code: &[Bytecode],
            _regs: &Regs,
            pc: u64,
            _state: &RuntimeState,
            _schedule: &Schedule,
            _listeners: &BytecodeListeners,
        ) {
            // Save the PC for the post_exec.
            self.current_pc = pc;
        }

        fn post_exec(
            &mut self,
            _i: Bytecode,
            _code: &[Bytecode],
            _regs: &Regs,
            _pc: u64,
            _state: &RuntimeState,
            _schedule: &Schedule,
            _listeners: &BytecodeListeners,
        ) {
            let n = SAMPLES_PENDING.swap(0, Ordering::Relaxed);
            if n != 0 {
                self.samples.push(self.current_pc);
            }
        }

        fn start(
            &mut self,
            _code: &[Bytecode],
            _regs: &Regs,
            _pc: u64,
            _state: &RuntimeState,
            _schedule: &Schedule,
            _listeners: &BytecodeListeners,
        ) {
            self.start();
        }

        fn finish(
            &mut self,
            code: &[Bytecode],
            _regs: &Regs,
            _pc: u64,
            _state: &RuntimeState,
            _schedule: &Schedule,
            _listeners: &BytecodeListeners,
        ) {
            self.stop();
            self.report(code);
        }
    }

    impl Drop for SamplingProfiler {
        fn drop(&mut self) {
            // Safety net: make sure the timer is disarmed and the handler detached
            // even if `finish` was never called.
            self.stop();
        }
    }

    /// Percentage of `n` relative to `total`, guarding against divide-by-zero.
    fn pct(n: u64, total: u64) -> f64 {
        if total == 0 {
            0.0
        } else {
            n as f64 * 100.0 / total as f64
        }
    }
}

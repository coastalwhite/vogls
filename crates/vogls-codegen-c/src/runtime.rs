use std::ffi::{c_int, c_void};
use std::io::{stderr, stdout};
use std::ops::Deref as _;
use std::path::Path;
use std::ptr::NonNull;

use vogls_codegen::HeapRef;
use vogls_ir::dyn_format_string::DynFormatString;
use vogls_ir::{Bits, GlobalContext, LogicMode, Mode, ReadMem, VectorSize};
use vogls_runtime::plugins::{RuntimePlugin, RuntimePluginState};
use vogls_runtime::{RtSignalKey, SimulationIo};
use vogls_utils::TableKey;

use crate::StateBuilder;

type Time = u64;
type HeapPtr = Option<NonNull<u64>>;
type Listening = Option<NonNull<u64>>;
type LastActiveTime = Option<NonNull<u64>>;

/// 0: No exit
/// 1-255: Exited with `value-1` exit code
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct ReturnValue(c_int);

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct StatePtr(c_int);

impl ReturnValue {
    pub const CONTINUE: Self = Self(0);
    pub const STOP: Self = Self(1);

    fn should_exit(self) -> bool {
        self.0 > 0
    }

    fn is_ok(self) -> bool {
        self.0 <= 1
    }
}

type EmptyActiveEventQueueFn = extern "C" fn(
    HeapPtr,
    NonNull<ScheduleT>,
    Time,
    Listening,
    LastActiveTime,
    NonNull<ColdContextT>,
) -> ReturnValue;

#[repr(C)]
pub struct BitsRefT {
    size: u32,
    mode: u8,
    ptr: NonNull<u64>,
}

#[repr(C)]
pub struct ColdContextT {
    fmt: extern "C" fn(
        NonNull<Box<dyn std::io::Write + Send + Sync>>,
        *const DynFormatString,
        *const BitsRefT,
    ),
    fmt_strs: *const DynFormatString,

    plugins: *mut Box<dyn RuntimePlugin>,
    plugin_poke_signal: extern "C" fn(NonNull<RuntimePluginState>, usize),

    heap_len: usize,
    readmems: *const (HeapRef, ReadMem),
    readmem: extern "C" fn(*mut u64, usize, u8, NonNull<(HeapRef, ReadMem)>),

    fst_poke: *mut u64,

    icount: u64,

    stdout: NonNull<Box<dyn std::io::Write + Send + Sync>>,
    stderr: NonNull<Box<dyn std::io::Write + Send + Sync>>,
}
#[repr(C)]
#[derive(Debug, Clone)]
pub struct EventT {
    ptr: *const c_void,
    state: StatePtr,
}

unsafe impl Sync for EventT {}
unsafe impl Send for EventT {}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct TimedEventT {
    event: EventT,
    time: Time,
}

#[repr(C)]
pub struct VecT<T> {
    ptr: *mut T,
    length: usize,
    capacity: usize,
    grow: extern "C" fn(NonNull<Self>),
}

impl<T> Default for VecT<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> VecT<T> {
    fn new() -> Self {
        Vec::new().into()
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.length == 0 {
            None
        } else {
            unsafe {
                self.length -= 1;
                core::hint::assert_unchecked(self.length < self.capacity);
                Some(std::ptr::read(self.ptr.add(self.length)))
            }
        }
    }
}

impl<T> From<Vec<T>> for VecT<T> {
    fn from(mut value: Vec<T>) -> Self {
        extern "C" fn grow<T>(mut slf: NonNull<VecT<T>>) {
            let slf = unsafe { slf.as_mut() };
            let mut v = unsafe { Vec::from_raw_parts(slf.ptr, slf.length, slf.capacity) };
            v.reserve(slf.capacity.max(1));
            slf.ptr = v.as_mut_ptr();
            slf.length = v.len();
            slf.capacity = v.capacity();
            std::mem::forget(v);
        }

        let slf = Self {
            ptr: value.as_mut_ptr(),
            length: value.len(),
            capacity: value.capacity(),
            grow,
        };
        std::mem::forget(value);
        slf
    }
}

impl<T: std::fmt::Debug> Into<Vec<T>> for VecT<T> {
    fn into(self) -> Vec<T> {
        let v = unsafe { Vec::from_raw_parts(self.ptr, self.length, self.capacity) };
        std::mem::forget(self);
        v
    }
}

impl<T> Drop for VecT<T> {
    fn drop(&mut self) {
        _ = unsafe { Vec::from_raw_parts(self.ptr, self.length, self.capacity) }
    }
}

#[repr(C)]
pub struct ScheduleT {
    active_region: VecT<EventT>,
    regions: *mut VecT<EventT>,
    future: VecT<TimedEventT>,
    next_time: Time,
}

#[derive(Clone)]
pub struct Schedule {
    active_region: Vec<EventT>,
    regions: Box<[Vec<EventT>]>,
    future: Vec<TimedEventT>,
    next_time: Time,
}

impl Schedule {
    fn with_t<T>(&mut self, mut f: impl FnMut(&mut ScheduleT) -> T) -> T {
        let active_region = std::mem::take(&mut self.active_region);
        let regions = std::mem::take(&mut self.regions);
        let future = std::mem::take(&mut self.future);

        let mut tregions = regions
            .into_iter()
            .map(|r| r.into())
            .collect::<Vec<VecT<EventT>>>();

        let mut t = ScheduleT {
            active_region: active_region.into(),
            regions: tregions.as_mut_ptr(),
            future: future.into(),
            next_time: self.next_time,
        };
        let result = f(&mut t);
        self.active_region = t.active_region.into();
        self.regions = tregions.into_iter().map(|r| r.into()).collect();
        self.future = t.future.into();
        self.next_time = t.next_time;
        result
    }
}

pub struct CDesignState {
    schedule: Schedule,
    listening: Vec<u64>,
    fst_poke: Vec<u64>,
    pub runtime: vogls_runtime::RuntimeState,
    pub plugins: Vec<RuntimePluginState>,
}

impl Clone for CDesignState {
    fn clone(&self) -> Self {
        Self {
            schedule: self.schedule.clone(),
            listening: self.listening.clone(),
            fst_poke: self.fst_poke.clone(),
            runtime: self.runtime.clone(),
            plugins: self.plugins.iter().map(|p| p.as_ref().clone()).collect(),
        }
    }
}

pub struct CDesign {
    lib: libloading::Library,
    dyn_fmt_strs: Vec<DynFormatString>,
    read_mems: Vec<(HeapRef, ReadMem)>,
    num_regions: u8,
}

impl CDesign {
    pub fn new_state(
        &self,
        heap: vogls_codegen::Heap,
        num_listening: usize,
        num_regions: u8,
        lupdt_updated: &[bool],
        gl: &GlobalContext,
    ) -> CDesignState {
        let procs = unsafe {
            self.lib
                .get::<*const *const std::ffi::c_void>("PROCS")
                .unwrap()
        };
        let mut active_region = Vec::new();
        active_region.extend((0..gl.processes.len()).map(|i| EventT {
            ptr: unsafe { *procs.add(i) },
            state: StatePtr(0),
        }));
        let schedule = Schedule {
            active_region,
            regions: std::iter::repeat_n(Vec::new(), num_regions.into()).collect(),
            future: Vec::new(),
            next_time: u64::MAX,
        };
        let listening = vec![0u64; num_listening.div_ceil(64)];
        let fst_poke = match gl.logic_mode {
            LogicMode::TwoValue => vec![0u64; gl.signals.len().div_ceil(64)],
            LogicMode::FourValue => Vec::new(),
        };

        CDesignState {
            schedule,
            listening,
            fst_poke,
            runtime: vogls_runtime::RuntimeState::new(
                gl.logic_mode,
                heap,
                gl.signals.len(),
                lupdt_updated,
            ),
            plugins: Vec::new(),
        }
    }
}

extern "C" fn plugin_poke_signal(mut plugin: NonNull<RuntimePluginState>, signal: usize) {
    unsafe { plugin.as_mut() }.poke_signal(RtSignalKey::from_usize(signal).unwrap());
}

extern "C" fn read_mem(
    heap: *mut u64,
    heap_len: usize,
    mode: u8,
    ptr: NonNull<(HeapRef, ReadMem)>,
) {
    let (heap_ref, read_mem) = unsafe { ptr.as_ref() };
    let heap = unsafe { std::slice::from_raw_parts_mut(heap, heap_len) };
    let mode = if mode == 0 {
        Mode::TwoValue
    } else {
        Mode::FourValue
    };
    vogls_runtime::readmem::read_mem(
        &read_mem.path,
        heap,
        *heap_ref,
        mode,
        read_mem.offset,
        read_mem.limit,
        read_mem.stride,
        read_mem.binary,
    )
    .unwrap();
}

extern "C" fn fmt(
    mut file: NonNull<Box<dyn std::io::Write + Send + Sync>>,
    dyn_fmt: *const DynFormatString,
    bits: *const BitsRefT,
) {
    // @TODO: Catch unwind
    let file = unsafe { file.as_mut() };
    let dyn_fmt = unsafe { dyn_fmt.as_ref() }.unwrap();
    let args = (0..dyn_fmt.arguments().len()).map(|i| {
        let ref_t = unsafe { bits.add(i).as_ref() }.unwrap();
        let size = VectorSize::new(ref_t.size).unwrap();
        if ref_t.mode == 0 && size.get() <= 64 {
            Bits::from_u64(size, *unsafe { ref_t.ptr.as_ref() })
        } else if ref_t.mode == 1 && size.get() <= 32 {
            Bits::from_four_value_u64(
                size,
                *unsafe { ref_t.ptr.as_ref() } as u32,
                (*unsafe { ref_t.ptr.as_ref() } >> size.get()) as u32,
            )
        } else if ref_t.mode == 1 {
            Bits::from_boxed_slice(
                vogls_ir::Mode::FourValue,
                size,
                (0..2 * size.get().div_ceil(64))
                    .map(|j| *unsafe { ref_t.ptr.add(j as usize).as_ref() })
                    .collect(),
            )
        } else {
            Bits::from_boxed_slice(
                vogls_ir::Mode::TwoValue,
                size,
                (0..size.get().div_ceil(64))
                    .map(|j| *unsafe { ref_t.ptr.add(j as usize).as_ref() })
                    .collect(),
            )
        }
    });
    dyn_fmt.write_to(file, args).unwrap();
}

pub trait SharedObjectContainer {
    fn as_path(&self) -> &Path;
}

impl CDesign {
    pub fn new(
        shared_object_container: Box<dyn SharedObjectContainer>,
        state_builder: StateBuilder,
        num_regions: u8,
    ) -> Self {
        let lib = unsafe { libloading::Library::new(shared_object_container.as_path()) }.unwrap();
        Self {
            lib,
            dyn_fmt_strs: state_builder.dyn_fmt_strs.take_keys(),
            read_mems: state_builder.read_mems,
            num_regions,
        }
    }

    pub fn run(
        &self,
        state: &mut CDesignState,
        io: &mut SimulationIo,
        max_time: u64,
    ) -> Result<(), ()> {
        let empty_active_event_queue_fn = unsafe {
            self.lib
                .get::<EmptyActiveEventQueueFn>("empty_active_event_queue")
        }
        .unwrap();
        let mut cldctx = ColdContextT {
            fmt,
            fmt_strs: self.dyn_fmt_strs.as_ptr(),
            plugins: state.plugins.as_mut_ptr(),
            plugin_poke_signal: plugin_poke_signal,
            heap_len: state.runtime.heap.0.len(),
            readmems: self.read_mems.as_ptr(),
            readmem: read_mem,
            fst_poke: state.fst_poke.as_mut_ptr(),
            icount: state.runtime.instruction_count,
            stdout: NonNull::from_mut(&mut io.stdout),
            stderr: NonNull::from_mut(&mut io.stderr),
        };
        let return_value = state.schedule.with_t(|schedule| {
            'main_loop: loop {
                let return_value = empty_active_event_queue_fn(
                    NonNull::new(state.runtime.heap.0.as_mut_ptr()),
                    NonNull::new(schedule as *mut ScheduleT).unwrap(),
                    state.runtime.time,
                    NonNull::new(state.listening.as_mut_ptr()),
                    NonNull::new(state.runtime.last_active_time.as_mut_ptr()),
                    NonNull::new(&mut cldctx as *mut ColdContextT).unwrap(),
                );
                // while let Some(e) = schedule.active_region.pop() {
                //     // state.runtime.event_count += 1;
                //     let return_value = (e.ptr)(
                //         e.state,
                //         NonNull::new(state.runtime.heap.0.as_mut_ptr()),
                //         NonNull::new(schedule as *mut ScheduleT).unwrap(),
                //         state.runtime.time,
                //         NonNull::new(state.listening.as_mut_ptr()),
                //         NonNull::new(state.runtime.last_active_time.as_mut_ptr()),
                //         NonNull::new(&mut cldctx as *mut ColdContextT).unwrap(),
                //     );

                if return_value.should_exit() {
                    return return_value;
                }

                for i in 0..self.num_regions as usize {
                    let region = unsafe { schedule.regions.add(i).as_mut() }.unwrap();
                    if region.length > 0 {
                        std::mem::swap(&mut schedule.active_region, region);
                        continue 'main_loop;
                    }
                }

                for plugin in state.plugins.iter_mut() {
                    plugin.timestep(&mut state.runtime);
                }

                if schedule.future.length == 0 {
                    return ReturnValue::CONTINUE;
                }

                if schedule.next_time.wrapping_add(1) < state.runtime.time {
                    eprintln!("Time overflow!");
                    return ReturnValue::STOP;
                }

                if schedule.next_time > max_time {
                    state.runtime.time = max_time;
                    break ReturnValue::CONTINUE;
                }

                let mut active: Vec<_> = std::mem::take(&mut schedule.active_region).into();
                let mut future: Vec<_> = std::mem::take(&mut schedule.future).into();

                state.runtime.time = schedule.next_time;
                let mut next_time = Time::MAX;
                active.extend(
                    future
                        .extract_if(.., |te| {
                            let is_next_timestep = te.time == schedule.next_time;
                            if !is_next_timestep {
                                next_time = te.time.min(next_time);
                            }
                            is_next_timestep
                        })
                        .map(|te| te.event),
                );

                schedule.active_region = active.into();
                schedule.future = future.into();
                schedule.next_time = next_time;

                if schedule.active_region.length == 0 {
                    break ReturnValue::CONTINUE;
                }
            }
        });

        state.runtime.instruction_count = cldctx.icount;
        for plugin in state.plugins.iter_mut() {
            plugin.finish(&mut state.runtime);
        }
        if return_value.should_exit() {
            if return_value.is_ok() {
                writeln!(io.stdout, "[FINISH]").unwrap();
                return Ok(());
            } else {
                return Err(());
            }
        }
        Ok(())
    }

    pub fn poke_signal(&self, state: &mut CDesignState, signal: RtSignalKey) {
        type DriveFn = extern "C" fn(
            NonNull<ScheduleT>,
            Time,
            Listening,
            LastActiveTime,
            NonNull<ColdContextT>,
        );
        let drive = unsafe {
            self.lib
                .get::<DriveFn>(&format!("drive_signal_{}", signal.as_u64()))
        }
        .unwrap();
        let mut cldctx = ColdContextT {
            fmt,
            fmt_strs: self.dyn_fmt_strs.as_ptr(),

            plugins: state.plugins.as_mut_ptr(),
            plugin_poke_signal,

            heap_len: state.runtime.heap.0.len(),
            readmems: self.read_mems.as_ptr(),
            readmem: read_mem,

            fst_poke: state.fst_poke.as_mut_ptr(),

            icount: 0,

            // @TODO: Passthrough IO?
            stdout: NonNull::from_mut(&mut (Box::new(stdout()) as _)),
            stderr: NonNull::from_mut(&mut (Box::new(stderr()) as _)),
        };
        state.schedule.with_t(|schedule| {
            (drive.deref())(
                NonNull::from_mut(schedule),
                state.runtime.time,
                NonNull::new(state.listening.as_mut_ptr()),
                NonNull::new(state.runtime.last_active_time.as_mut_ptr()),
                NonNull::from_mut(&mut cldctx),
            );
        });
    }
}

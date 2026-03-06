use std::ffi::c_int;
use std::ops::Deref as _;
use std::path::Path;
use std::ptr::NonNull;

use vogls_ir::dyn_format_string::DynFormatString;
use vogls_ir::{Bits, GlobalContext, VectorSize};
use vogls_runtime::SimulationIo;

type Time = u64;
type HeapPtr = Option<NonNull<u64>>;
type IsScheduled = Option<NonNull<u64>>;
type Listening = Option<NonNull<u64>>;
type LastActiveTime = Option<NonNull<u64>>;

type StartupFn = extern "C" fn(
    HeapPtr,
    NonNull<ScheduleT>,
    Time,
    IsScheduled,
    Listening,
    LastActiveTime,
    NonNull<ColdContextT>,
);

#[repr(C)]
pub struct BitsRefT {
    size: u32,
    mode: u8,
    ptr: NonNull<u64>,
}

#[repr(C)]
pub struct ColdContextT {
    /// 0: No exit
    /// 1-255: Exited with -1 exit code
    exit: u8,

    fmt: extern "C" fn(
        NonNull<Box<dyn std::io::Write + Send + Sync>>,
        NonNull<DynFormatString>,
        *const BitsRefT,
    ),

    stdout: NonNull<Box<dyn std::io::Write + Send + Sync>>,
    stderr: NonNull<Box<dyn std::io::Write + Send + Sync>>,
}
#[repr(C)]
#[derive(Debug, Clone)]
pub struct EventT {
    ptr: extern "C" fn(
        c_int,
        HeapPtr,
        NonNull<ScheduleT>,
        Time,
        IsScheduled,
        Listening,
        LastActiveTime,
        NonNull<ColdContextT>,
    ),
    state: c_int,
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
    fn with_t(&mut self, mut f: impl FnMut(&mut ScheduleT)) {
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
        f(&mut t);
        self.active_region = t.active_region.into();
        self.regions = tregions.into_iter().map(|r| r.into()).collect();
        self.future = t.future.into();
        self.next_time = t.next_time;
    }
}

#[derive(Clone)]
pub struct CDesignState {
    schedule: Schedule,
    is_scheduled: Vec<u64>,
    listening: Vec<u64>,
    started: bool,
    pub runtime: vogls_runtime::RuntimeState,
}

pub struct CDesign {
    lib: libloading::Library,
    num_regions: u8,
}

impl CDesignState {
    pub fn new(
        gl: &GlobalContext,
        heap: vogls_codegen::Heap,
        num_listening: usize,
        num_regions: u8,
    ) -> Self {
        let schedule = Schedule {
            active_region: Vec::new(),
            regions: std::iter::repeat_n(Vec::new(), num_regions.into()).collect(),
            future: Vec::new(),
            next_time: u64::MAX,
        };
        let is_scheduled = vec![0u64; gl.processes.len().div_ceil(64)];
        let listening = vec![0u64; num_listening.div_ceil(64)];

        Self {
            schedule,
            is_scheduled,
            listening,
            started: false,
            runtime: vogls_runtime::RuntimeState {
                heap,
                time: 0,
                last_active_time: vec![0u64; gl.signals.len()],
                event_count: 0,
            },
        }
    }
}

extern "C" fn fmt(
    mut file: NonNull<Box<dyn std::io::Write + Send + Sync>>,
    mut dyn_fmt: NonNull<DynFormatString>,
    bits: *const BitsRefT,
) {
    // @TODO: Catch unwind
    let file = unsafe { file.as_mut() };
    let dyn_fmt = unsafe { dyn_fmt.as_mut() };
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

impl CDesign {
    pub fn new(path: &Path, num_regions: u8) -> Self {
        let lib = unsafe { libloading::Library::new(path) }.unwrap();
        Self { lib, num_regions }
    }

    pub fn start(&self, state: &mut CDesignState, io: &mut SimulationIo) -> Result<(), ()> {
        if !state.started {
            let startup = unsafe { self.lib.get::<StartupFn>("startup") }.unwrap();
            let mut cldctx = ColdContextT {
                exit: 0,
                fmt,
                stdout: NonNull::from_mut(&mut io.stdout),
                stderr: NonNull::from_mut(&mut io.stderr),
            };
            state.schedule.with_t(|schedule| {
                (startup.deref())(
                    NonNull::new(state.runtime.heap.0.as_mut_ptr()),
                    NonNull::new(schedule as *mut ScheduleT).unwrap(),
                    0,
                    NonNull::new(state.is_scheduled.as_mut_ptr()),
                    NonNull::new(state.listening.as_mut_ptr()),
                    NonNull::new(state.runtime.last_active_time.as_mut_ptr()),
                    NonNull::new(&mut cldctx as *mut ColdContextT).unwrap(),
                );
            });

            if cldctx.exit > 0 {
                if cldctx.exit == 1 {
                    return Ok(());
                } else {
                    return Err(());
                }
            }
        }
        state.started = true;
        Ok(())
    }

    pub fn run(
        &self,
        state: &mut CDesignState,
        io: &mut SimulationIo,
        max_time: u64,
    ) -> Result<(), ()> {
        if !state.started {
            self.start(state, io)?;
        }

        let mut cldctx = ColdContextT {
            exit: 0,
            fmt,
            stdout: NonNull::from_mut(&mut io.stdout),
            stderr: NonNull::from_mut(&mut io.stderr),
        };
        state.schedule.with_t(|schedule| {
            'main_loop: loop {
                while let Some(e) = schedule.active_region.pop() {
                    state.runtime.event_count += 1;
                    (e.ptr)(
                        e.state,
                        NonNull::new(state.runtime.heap.0.as_mut_ptr()),
                        NonNull::new(schedule as *mut ScheduleT).unwrap(),
                        state.runtime.time,
                        NonNull::new(state.is_scheduled.as_mut_ptr()),
                        NonNull::new(state.listening.as_mut_ptr()),
                        NonNull::new(state.runtime.last_active_time.as_mut_ptr()),
                        NonNull::new(&mut cldctx as *mut ColdContextT).unwrap(),
                    );

                    if cldctx.exit > 0 {
                        return;
                    }
                }

                for i in 0..self.num_regions as usize {
                    let region = unsafe { schedule.regions.add(i).as_mut() }.unwrap();
                    if region.length > 0 {
                        std::mem::swap(&mut schedule.active_region, region);
                        continue 'main_loop;
                    }
                }

                if schedule.next_time > max_time {
                    state.runtime.time = max_time;
                    break;
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
                    break;
                }
            }
        });

        if cldctx.exit > 0 {
            if cldctx.exit == 1 {
                writeln!(io.stdout, "[FINISH]").unwrap();
                return Ok(());
            } else {
                return Err(());
            }
        }
        Ok(())
    }
}

use std::default;
use std::ffi::c_int;
use std::ops::Deref as _;
use std::path::Path;
use std::ptr::NonNull;

use vogls_ir::GlobalContext;

type Time = u64;
type HeapPtr = Option<NonNull<u64>>;
type IsScheduled = Option<NonNull<u64>>;
type Listening = Option<NonNull<u64>>;
type LastActiveTime = Option<NonNull<u64>>;

type StartupFn =
    extern "C" fn(HeapPtr, NonNull<ScheduleT>, Time, IsScheduled, Listening, LastActiveTime);

#[repr(C)]
pub struct EventT {
    ptr: extern "C" fn(
        c_int,
        HeapPtr,
        NonNull<ScheduleT>,
        Time,
        IsScheduled,
        Listening,
        LastActiveTime,
    ),
    state: c_int,
}
#[repr(C)]
pub struct TimedEventT {
    event: EventT,
    time: Time,
}

#[repr(C)]
pub struct VecT<T> {
    ptr: Option<NonNull<T>>,
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
        let mut v = unsafe {
            Vec::from_raw_parts(
                self.ptr.map_or(std::ptr::null_mut(), |ptr| ptr.as_ptr()),
                self.length,
                self.capacity,
            )
        };
        let item = v.pop();
        std::mem::forget(v);
        item
    }
}

impl<T> From<Vec<T>> for VecT<T> {
    fn from(mut value: Vec<T>) -> Self {
        extern "C" fn grow<T>(mut slf: NonNull<VecT<T>>) {
            let slf = unsafe { slf.as_mut() };
            let mut v = unsafe {
                Vec::from_raw_parts(
                    slf.ptr.map_or(std::ptr::null_mut(), |ptr| ptr.as_ptr()),
                    slf.length,
                    slf.capacity,
                )
            };
            v.reserve((slf.capacity * 2).max(1));
            slf.ptr = NonNull::new(v.as_mut_ptr());
            slf.length = v.len();
            slf.capacity = v.capacity();
            std::mem::forget(v);
        }

        let slf = Self {
            ptr: NonNull::new(value.as_mut_ptr()),
            length: value.len(),
            capacity: value.capacity(),
            grow,
        };
        std::mem::forget(value);
        slf
    }
}

impl<T> Into<Vec<T>> for VecT<T> {
    fn into(self) -> Vec<T> {
        unsafe {
            Vec::from_raw_parts(
                self.ptr.map_or(std::ptr::null_mut(), |ptr| ptr.as_ptr()),
                self.length,
                self.capacity,
            )
        }
    }
}

impl<T> Drop for VecT<T> {
    fn drop(&mut self) {
        _ = unsafe {
            Vec::from_raw_parts(
                self.ptr.map_or(std::ptr::null_mut(), |ptr| ptr.as_ptr()),
                self.length,
                self.capacity,
            )
        }
    }
}

#[repr(C)]
pub struct ScheduleT {
    active_region: VecT<EventT>,
    regions: Option<NonNull<VecT<EventT>>>,
    future: VecT<TimedEventT>,
    next_time: Time,
}

pub struct CDesignState {
    schedule: ScheduleT,
    is_scheduled: Vec<u64>,
    listening: Vec<u64>,
    started: bool,
    runtime: vogls_runtime::RuntimeState,
}

pub struct CDesign {
    lib: libloading::Library,
}

impl CDesignState {
    pub fn new(gl: &GlobalContext, heap: vogls_codegen::Heap, num_listening: usize) -> Self {
        let schedule = ScheduleT {
            active_region: VecT::new(),
            regions: None,
            future: VecT::new(),
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

impl CDesign {
    pub fn new(path: &Path) -> Self {
        let lib = unsafe { libloading::Library::new(path) }.unwrap();
        Self { lib }
    }

    pub fn start(&self, state: &mut CDesignState) {
        if !state.started {
            let startup = unsafe { self.lib.get::<StartupFn>("startup") }.unwrap();
            (startup.deref())(
                NonNull::new(state.runtime.heap.0.as_mut_ptr()),
                NonNull::new(&mut state.schedule as *mut ScheduleT).unwrap(),
                0,
                NonNull::new(state.is_scheduled.as_mut_ptr()),
                NonNull::new(state.listening.as_mut_ptr()),
                NonNull::new(state.runtime.last_active_time.as_mut_ptr()),
            );
        }
        state.started = true;
    }

    pub fn run(&self, state: &mut CDesignState) {
        if !state.started {
            self.start(state);
        }

        while state.schedule.active_region.length > 0 {
            while let Some(e) = state.schedule.active_region.pop() {
                (e.ptr)(
                    e.state,
                    NonNull::new(state.runtime.heap.0.as_mut_ptr()),
                    NonNull::new(&mut state.schedule as *mut ScheduleT).unwrap(),
                    0,
                    NonNull::new(state.is_scheduled.as_mut_ptr()),
                    NonNull::new(state.listening.as_mut_ptr()),
                    NonNull::new(state.runtime.last_active_time.as_mut_ptr()),
                );
            }

            let mut active: Vec<_> = std::mem::take(&mut state.schedule.active_region).into();
            let mut future: Vec<_> = std::mem::take(&mut state.schedule.future).into();

            // @TODO: Regions

            let mut next_time = Time::MAX;
            active.extend(
                future
                    .extract_if(.., |te| {
                        let is_next_timestep = te.time == state.schedule.next_time;
                        if !is_next_timestep {
                            next_time = te.time.min(next_time);
                        }
                        is_next_timestep
                    })
                    .map(|te| te.event),
            );

            state.schedule.active_region = active.into();
            state.schedule.future = future.into();
        }
    }
}

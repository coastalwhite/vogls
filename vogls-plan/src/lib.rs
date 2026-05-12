use vogls::design::DesignState;
use vogls_trace::Trace;

pub mod array;
pub mod compute;
pub mod design;
pub mod dsl;
pub mod output;
pub mod plan;
pub mod run;

pub struct TraceRef(usize);

impl TraceRef {
    pub fn extract(&self, state: &mut DesignState) -> Trace {
        let plugins = match &mut *state {
            DesignState::Interpretted(s) => &mut s.plugins,
            DesignState::Compiled(s) => &mut s.plugins,
        };
        let trace = plugins.remove(self.0);
        let trace = trace as Box<dyn std::any::Any>;
        let trace = trace.downcast::<vogls_trace::TracePlugin>().unwrap();
        Trace {
            trace: trace.trace,
            time_offsets: trace.time_offsets,
        }
    }
}

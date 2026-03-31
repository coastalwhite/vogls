use std::borrow::Cow;
use std::time::{Duration, SystemTime};

/// A stack to keep track of timers in code.
pub struct TimerStack {
    pub enabled: bool,
    pub current: Vec<(Cow<'static, str>, SystemTime, usize)>,
    pub finished: Vec<(usize, Cow<'static, str>, Duration)>,
}

impl TimerStack {
    pub const fn new(enabled: bool) -> Self {
        Self {
            enabled,
            current: Vec::new(),
            finished: Vec::new(),
        }
    }

    #[inline(always)]
    pub fn start(&mut self, name: impl Into<Cow<'static, str>>) {
        if !self.enabled {
            return;
        }

        let name = name.into();
        let i = self.finished.len();
        self.finished
            .push((self.current.len(), Default::default(), Duration::default()));
        self.current
            .push((name.into(), std::time::SystemTime::now(), i));
    }

    #[inline(always)]
    pub fn stop(&mut self) {
        if !self.enabled {
            return;
        }

        let (name, time, i) = self.current.pop().unwrap();
        let time = time.elapsed().unwrap();
        self.finished[i].1 = name;
        self.finished[i].2 = time;
    }

    #[inline(always)]
    pub fn timed<T>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
        mut f: impl FnMut(&mut Self) -> T,
    ) -> T {
        self.start(name);
        let result = f(self);
        self.stop();
        result
    }

    pub fn print(&self) {
        println!("# Timings Overview:");
        for (depth, name, elapsed) in &self.finished {
            let pad = String::from("  ").repeat(*depth);
            println!("{pad}{name}: {:.4}s", elapsed.as_secs_f32());
        }
        println!();
    }
}

use std::io;
use std::ops::Range;

use vogls_ir::Bits;

pub type Timestamp = u64;
pub type FileIdx = u64;
pub type SignalIdx = u64;
pub type ProcessIdx = u64;
pub type DrivenIdx = u64;
pub type WokenIdx = u64;
pub type WatchIdx = u64;

pub struct Trace {
    pub files: Vec<File>,
    pub processes: Vec<Process>,
    pub signals: Vec<Signal>,
    pub driven: Vec<(SignalIdx, Bits, Range<WokenIdx>)>,
    pub woken: Vec<ProcessIdx>,
    pub watches: Vec<SignalIdx>,
    pub events: Vec<Event>,
}

pub struct File {
    pub name: Option<String>,
    pub content: String,
}

pub struct Span {
    pub file: FileIdx,
    pub line_range: Range<u64>,
}

pub struct Process {
    pub name: Option<String>,
    pub location: Option<Span>,
}

pub struct Signal {
    pub name: Option<String>,
    pub location: Option<Span>,
    pub initial: Bits,
}

pub enum EventType {
    Evaluation(ProcessIdx),
    Drive(SignalIdx),
}

pub enum EventStopReason {
    Halt,
    Wait(Timestamp),
    WaitRegion(u8),
    WatchSignals(Range<WatchIdx>),
}

pub enum Event {
    Evaluation(ProcessIdx, Range<DrivenIdx>, EventStopReason),
    Drive(SignalIdx, Option<DrivenIdx>),
    Time(Timestamp),
}

fn dump_opt_str(s: Option<&str>, f: &mut impl io::Write) -> io::Result<()> {
    match s {
        Some(s) => dump_str(s, f),
        None => f.write_all(&u64::MAX.to_le_bytes()),
    }
}
fn dump_str(s: &str, f: &mut impl io::Write) -> io::Result<()> {
    assert!((s.len() as u64) < u64::MAX);
    f.write_all(&(s.len() as u64).to_le_bytes())?;
    f.write_all(s.as_bytes())?;
    Ok(())
}
fn dump_opt_span(s: Option<&Span>, f: &mut impl io::Write) -> io::Result<()> {
    match s {
        Some(s) => dump_span(s, f),
        None => {
            f.write_all(&u64::MAX.to_le_bytes())?;
            f.write_all(&u64::MAX.to_le_bytes())?;
            f.write_all(&u64::MAX.to_le_bytes())
        }
    }
}
fn dump_span(span: &Span, f: &mut impl io::Write) -> io::Result<()> {
    assert!(span.file < u64::MAX);
    f.write_all(&span.file.to_le_bytes())?;
    f.write_all(&span.line_range.start.to_le_bytes())?;
    f.write_all(&span.line_range.end.to_le_bytes())?;
    Ok(())
}
fn dump_bits(bits: &Bits, f: &mut impl io::Write) -> io::Result<()> {
    f.write_all(&bits.size().get().to_le_bytes())?;
    f.write_all(bits.as_slice())?;
    Ok(())
}

impl Trace {
    pub fn dump(&self, f: &mut impl io::Write) -> io::Result<()> {
        f.write_all(b"VGTD")?;

        let Self {
            files,
            processes,
            signals,
            driven,
            woken,
            watches,
            events,
        } = self;

        // Metadata
        f.write_all(&(files.len() as u64).to_le_bytes())?;
        f.write_all(&(processes.len() as u64).to_le_bytes())?;
        f.write_all(&(signals.len() as u64).to_le_bytes())?;
        f.write_all(&(driven.len() as u64).to_le_bytes())?;
        f.write_all(&(woken.len() as u64).to_le_bytes())?;
        f.write_all(&(watches.len() as u64).to_le_bytes())?;
        f.write_all(&(events.len() as u64).to_le_bytes())?;

        dbg!(events.len());

        for file in &self.files {
            dump_opt_str(file.name.as_deref(), f)?;
            dump_str(file.content.as_str(), f)?;
        }

        for process in &self.processes {
            dump_opt_str(process.name.as_deref(), f)?;
            dump_opt_span(process.location.as_ref(), f)?;
        }

        for signal in &self.signals {
            dump_opt_str(signal.name.as_deref(), f)?;
            dump_opt_span(signal.location.as_ref(), f)?;
            dump_bits(&signal.initial, f)?;
        }

        for (signal, value, woken_range) in &self.driven {
            f.write_all(&signal.to_le_bytes())?;
            dump_bits(value, f)?;
            f.write_all(&woken_range.start.to_le_bytes())?;
            f.write_all(&woken_range.end.to_le_bytes())?;
        }

        for process in &self.woken {
            f.write_all(&process.to_le_bytes())?;
        }

        for signal in &self.watches {
            f.write_all(&signal.to_le_bytes())?;
        }

        for e in &self.events {
            match e {
                Event::Evaluation(process, driven, stop_reason) => {
                    f.write_all(&[0])?;
                    f.write_all(&process.to_le_bytes())?;
                    f.write_all(&driven.start.to_le_bytes())?;
                    f.write_all(&driven.end.to_le_bytes())?;
                    match stop_reason {
                        EventStopReason::Halt => f.write_all(&[0])?,
                        EventStopReason::Wait(t) => {
                            f.write_all(&[1])?;
                            f.write_all(&t.to_le_bytes())?;
                        }
                        EventStopReason::WaitRegion(region) => {
                            f.write_all(&[2])?;
                            f.write_all(&region.to_le_bytes())?;
                        }
                        EventStopReason::WatchSignals(range) => {
                            f.write_all(&[3])?;
                            f.write_all(&range.start.to_le_bytes())?;
                            f.write_all(&range.end.to_le_bytes())?;
                        }
                    }
                }
                Event::Drive(s, d) => {
                    f.write_all(&[1])?;
                    f.write_all(&s.to_le_bytes())?;
                    match d {
                        None => f.write_all(&u64::MAX.to_le_bytes())?,
                        Some(d) => f.write_all(&d.to_le_bytes())?,
                    }
                }
                Event::Time(t) => {
                    f.write_all(&[2])?;
                    f.write_all(&t.to_le_bytes())?;
                }
            }
        }

        Ok(())
    }
}

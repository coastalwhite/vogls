//! `$readmemb` and `$readmemh` (IEEE Std 1364-2005, 17.2.9).

use std::fmt;
use std::io::Write;
use std::path::Path;

use vogls_bits::Mode;
use vogls_codegen::{HeapAlignment, HeapRef};
use vogls_ir::{ReadMem, ReadMemVariant};
use vogls_world::{World, WorldError};

use crate::RuntimeError;

#[derive(Debug)]
pub struct ReadMemError {
    variant: ReadMemVariant,
    path: String,
    line: Option<u32>,
    kind: ReadMemErrorKind,
}

#[derive(Debug)]
pub enum ReadMemErrorKind {
    Open(WorldError),
    StartAddressOutOfRange(i64),
    FinishAddressOutOfRange(i64),
    UnterminatedComment,
    EmptyAddress,
    InvalidDigit(char),
    AddressOutOfRange(u64),
}

impl ReadMemError {
    pub fn kind(&self) -> &ReadMemErrorKind {
        &self.kind
    }
}

impl fmt::Display for ReadMemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: `{}`", self.variant.task_name(), self.path)?;
        if let Some(line) = self.line {
            write!(f, ":{line}")?;
        }
        f.write_str(": ")?;
        match &self.kind {
            ReadMemErrorKind::Open(err) => write!(f, "failed to open file: {err}"),
            ReadMemErrorKind::StartAddressOutOfRange(addr) => {
                write!(f, "start address {addr} is outside of the memory")
            }
            ReadMemErrorKind::FinishAddressOutOfRange(addr) => {
                write!(f, "finish address {addr} is outside of the memory")
            }
            ReadMemErrorKind::UnterminatedComment => f.write_str("unterminated comment"),
            ReadMemErrorKind::EmptyAddress => f.write_str("`@` is not followed by an address"),
            ReadMemErrorKind::InvalidDigit(c) => write!(f, "invalid digit `{}`", c.escape_debug()),
            ReadMemErrorKind::AddressOutOfRange(addr) => {
                write!(f, "address {addr:x} is outside of the address range")
            }
        }
    }
}

impl std::error::Error for ReadMemError {}

pub fn read_mem(
    world: &mut dyn World,
    heap: &mut [u64],
    signal: HeapRef,
    mode: Mode,
    readmem: &ReadMem,
) -> Result<(), RuntimeError> {
    let error = |line, kind| {
        RuntimeError::ReadMem(ReadMemError {
            variant: readmem.variant,
            path: readmem.path.clone(),
            line,
            kind,
        })
    };

    let stride = readmem.stride;
    assert!(
        u64::from(readmem.offset) + u64::from(readmem.limit) * u64::from(stride.get())
            <= u64::from(signal.size.get()),
        "memory does not fit in its signal"
    );

    // The address range that is written to, in the order it is written.
    let lowest = readmem.base_address;
    let highest = lowest + i64::from(readmem.limit) - 1;
    let in_memory = |addr: i64| (lowest..=highest).contains(&addr);
    let start = readmem.start_address.unwrap_or(lowest);
    if !in_memory(start) {
        return Err(error(None, ReadMemErrorKind::StartAddressOutOfRange(start)));
    }
    let finish = readmem.finish_address.unwrap_or(highest);
    if !in_memory(finish) {
        return Err(error(
            None,
            ReadMemErrorKind::FinishAddressOutOfRange(finish),
        ));
    }
    let descending = start > finish;
    let (range_lo, range_hi) = (start.min(finish), start.max(finish));

    let data = world
        .read(Path::new(&readmem.path))
        .map_err(|err| error(None, ReadMemErrorKind::Open(err)))?;

    let base_offset = signal.offset.bit_offset as u64;
    let val_offset = match mode {
        Mode::TwoValue => None,
        Mode::FourValue => Some(HeapAlignment::spc_offset_to_val_offset(
            signal.size,
            base_offset,
        )),
    };
    let (bits_per_digit, digit_mask, base) = match readmem.variant {
        ReadMemVariant::Binary => (1, 0b1, 2),
        ReadMemVariant::Hexadecimal => (4, 0b1111, 16),
    };
    let nwords = stride.get().div_ceil(64) as usize;
    let mut scratch_spc = vec![0u64; nwords];
    let mut scratch_val = vec![0u64; nwords];

    let mut addr = Some(start);
    let mut has_address = false;
    let mut num_words = 0u64;
    let mut num_dropped = 0u64;

    let mut line = 1u32;
    let mut i = 0;
    while i < data.len() {
        match data[i] {
            b'\n' => {
                line += 1;
                i += 1;
            }
            c if c.is_ascii_whitespace() => i += 1,

            // Comments
            b'/' if data.get(i + 1) == Some(&b'/') => {
                while i < data.len() && data[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if data.get(i + 1) == Some(&b'*') => {
                let start_line = line;
                i += 2;
                loop {
                    match data.get(i) {
                        None => {
                            return Err(error(
                                Some(start_line),
                                ReadMemErrorKind::UnterminatedComment,
                            ));
                        }
                        Some(b'*') if data.get(i + 1) == Some(&b'/') => break,
                        Some(b'\n') => line += 1,
                        Some(_) => {}
                    }
                    i += 1;
                }
                i += 2;
            }

            // Address annotation
            b'@' => {
                i += 1;
                let token = take_token(&data, &mut i);
                let mut value = 0u64;
                let mut has_digits = false;
                for &c in token {
                    if c == b'_' {
                        continue;
                    }
                    let Some(digit) = (c as char).to_digit(16) else {
                        return Err(error(Some(line), invalid_digit(token, c)));
                    };
                    value = value.saturating_mul(16).saturating_add(u64::from(digit));
                    has_digits = true;
                }
                if !has_digits {
                    return Err(error(Some(line), ReadMemErrorKind::EmptyAddress));
                }
                let new_addr = i64::try_from(value)
                    .ok()
                    .filter(|a| (range_lo..=range_hi).contains(a));
                let Some(new_addr) = new_addr else {
                    return Err(error(
                        Some(line),
                        ReadMemErrorKind::AddressOutOfRange(value),
                    ));
                };
                addr = Some(new_addr);
                has_address = true;
            }
            _ => {
                // Consume one word.
                let token = take_token(&data, &mut i);
                scratch_spc.fill(0);
                scratch_val.fill(0);

                let mut bit = 0u32;
                let mut has_digits = false;
                for &c in token.iter().rev() {
                    let (spc, val) = match c {
                        b'_' => continue,
                        b'x' | b'X' => (0, 0),
                        b'z' | b'Z' => (0, digit_mask),
                        _ => match (c as char).to_digit(base) {
                            Some(d) => (digit_mask, u64::from(d)),
                            None => return Err(error(Some(line), invalid_digit(token, c))),
                        },
                    };
                    has_digits = true;
                    if bit < stride.get() {
                        or_bits(
                            &mut scratch_spc,
                            &mut scratch_val,
                            bit,
                            spc,
                            val,
                            bits_per_digit.min(stride.get() - bit),
                        );
                    }
                    bit = bit.saturating_add(bits_per_digit);
                }
                if !has_digits {
                    return Err(error(Some(line), invalid_digit(token, b'_')));
                }

                // Zero-extend.
                while bit < stride.get() {
                    let n = (stride.get() - bit).min(64 - bit % 64);
                    or_bits(&mut scratch_spc, &mut scratch_val, bit, u64::MAX, 0, n);
                    bit += n;
                }

                let Some(at) = addr else {
                    num_dropped += 1;
                    continue;
                };

                let elem = (at - lowest) as u64;
                let bit_offset = u64::from(readmem.offset) + elem * u64::from(stride.get());
                match val_offset {
                    None => {
                        // Two-valued logic reads `x` and `z` as 0.
                        for (i, (&spc, &val)) in scratch_spc.iter().zip(&scratch_val).enumerate() {
                            let n = (stride.get() - 64 * i as u32).min(64);
                            write_bits(
                                heap,
                                base_offset + bit_offset + 64 * i as u64,
                                spc & val,
                                n,
                            );
                        }
                    }
                    Some(val_base) => {
                        for (i, (&spc, &val)) in scratch_spc.iter().zip(&scratch_val).enumerate() {
                            let n = (stride.get() - 64 * i as u32).min(64);
                            write_bits(heap, base_offset + bit_offset + 64 * i as u64, spc, n);
                            write_bits(heap, val_base + bit_offset + 64 * i as u64, val, n);
                        }
                    }
                }
                num_words += 1;

                addr = match (descending, at) {
                    (false, at) if at == range_hi => None,
                    (true, at) if at == range_lo => None,
                    (false, at) => Some(at + 1),
                    (true, at) => Some(at - 1),
                };
            }
        }
    }

    // IEEE Std 1364-2005, 17.2.9:
    // > A warning message shall be issued if the number of data words in the file differs from the
    // > number of words in the range implied by the start through finish addresses and no address
    // > specifications appear within the data file.
    let range_len = (range_hi - range_lo) as u64 + 1;

    // A warning that cannot be written does not end the simulation.
    if num_dropped > 0
        && let Ok(mut f) = world.runtime_warn()
    {
        _ = writeln!(
            f,
            "{}: `{}`: {num_dropped} word(s) do not fit in the address range",
            readmem.variant.task_name(),
            readmem.path,
        );
    } else if num_dropped == 0
        && !has_address
        && readmem.finish_address.is_some()
        && num_words != range_len
        && let Ok(mut f) = world.runtime_warn()
    {
        _ = writeln!(
            f,
            "{}: `{}`: file has {num_words} word(s), but the address range has {range_len}",
            readmem.variant.task_name(),
            readmem.path,
        );
    }

    Ok(())
}

/// Takes the bytes up to the next whitespace or comment.
fn take_token<'a>(data: &'a [u8], i: &mut usize) -> &'a [u8] {
    let start = *i;
    while let Some(&c) = data.get(*i) {
        let is_comment = c == b'/' && matches!(data.get(*i + 1), Some(b'/' | b'*'));
        if c.is_ascii_whitespace() || is_comment {
            break;
        }
        *i += 1;
    }
    &data[start..*i]
}

fn invalid_digit(token: &[u8], c: u8) -> ReadMemErrorKind {
    // Report the full character for a non-ASCII byte.
    let c = if c.is_ascii() {
        c as char
    } else {
        String::from_utf8_lossy(token)
            .chars()
            .find(|c| !c.is_ascii())
            .unwrap_or(char::REPLACEMENT_CHARACTER)
    };
    ReadMemErrorKind::InvalidDigit(c)
}

fn or_bits(scratch_spc: &mut [u64], scratch_val: &mut [u64], bit: u32, spc: u64, val: u64, n: u32) {
    let mask = 1u64.unbounded_shl(n).wrapping_sub(1);
    let (w, sh) = ((bit / 64) as usize, bit % 64);
    scratch_spc[w] |= (spc & mask) << sh;
    scratch_val[w] |= (val & mask) << sh;
}

/// Writes the lowest `n` bits of `value` at `bit`, leaving the bits around it unchanged.
fn write_bits(heap: &mut [u64], bit: u64, value: u64, n: u32) {
    debug_assert!((1..=64).contains(&n));
    let mask = 1u64.unbounded_shl(n).wrapping_sub(1);
    let value = value & mask;
    let bit = bit as usize;
    let (w, sh) = (bit / 64, (bit % 64) as u32);
    heap[w] = (heap[w] & !(mask << sh)) | (value << sh);
    if sh + n > 64 {
        let rem = sh + n - 64;
        let mask = (1u64 << rem) - 1;
        heap[w + 1] = (heap[w + 1] & !mask) | (value >> (64 - sh));
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::io;
    use std::rc::Rc;

    use proptest::prelude::*;
    use proptest::proptest;
    use vogls_bits::arithmetic::FvLogicValue;
    use vogls_codegen::{Heap, HeapBuilder};
    use vogls_ir::{Bits, LogicMode, SignalKey, VectorSize};
    use vogls_world::WorldResult;
    use vogls_world::file::{FileId, FileOpenOptions};

    use super::*;

    /// A world with a single file, whatever its path, and a captured standard error.
    struct FileWorld {
        file: Option<Vec<u8>>,
        stderr: Rc<RefCell<Vec<u8>>>,
    }

    struct Capture(Rc<RefCell<Vec<u8>>>);

    impl io::Write for Capture {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.borrow_mut().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl World for FileWorld {
        fn stdout(&mut self) -> Box<dyn io::Write> {
            Box::new(io::sink())
        }
        fn stderr(&mut self) -> Box<dyn io::Write> {
            Box::new(Capture(self.stderr.clone()))
        }
        fn stdin(&mut self) -> Box<dyn io::Read> {
            Box::new(io::empty())
        }
        fn file_open(&mut self, _: &Path, _: FileOpenOptions) -> WorldResult<FileId> {
            unreachable!("only `read` is used")
        }
        fn file_close(&mut self, _: FileId) -> WorldResult<()> {
            unreachable!("only `read` is used")
        }
        fn file_write(&mut self, _: FileId) -> Option<&mut dyn io::Write> {
            unreachable!("only `read` is used")
        }
        fn file_read(&mut self, _: FileId) -> Option<&mut dyn io::Read> {
            unreachable!("only `read` is used")
        }
        fn read(&mut self, _: &Path) -> WorldResult<Vec<u8>> {
            self.file
                .clone()
                .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound).into())
        }
    }

    #[derive(Debug, Clone)]
    struct Config {
        mode: LogicMode,
        stride: u32,
        limit: u32,
        /// Bits in the signal below the memory.
        offset: u32,
        /// Bits in the signal above the memory.
        extra: u32,
        base_address: i64,
        variant: ReadMemVariant,
        /// Start and finish address as an index into the memory.
        start_finish: Option<(u32, Option<u32>)>,
    }

    impl Config {
        fn signal_size(&self) -> VectorSize {
            VectorSize::new(self.offset + self.limit * self.stride + self.extra).unwrap()
        }

        fn bits_per_digit(&self) -> u32 {
            match self.variant {
                ReadMemVariant::Binary => 1,
                ReadMemVariant::Hexadecimal => 4,
            }
        }

        fn readmem(&self) -> ReadMem {
            ReadMem {
                path: "mem.txt".to_string(),
                signal: SignalKey::default(),
                offset: self.offset,
                limit: self.limit,
                stride: VectorSize::new(self.stride).unwrap(),
                variant: self.variant,
                base_address: self.base_address,
                start_address: self
                    .start_finish
                    .map(|(s, _)| self.base_address + i64::from(s)),
                finish_address: self
                    .start_finish
                    .and_then(|(_, f)| f)
                    .map(|f| self.base_address + i64::from(f)),
            }
        }

        /// The addresses that are loaded, lowest first.
        fn range(&self) -> (i64, i64) {
            let rm = self.readmem();
            let lowest = self.base_address;
            let highest = lowest + i64::from(self.limit) - 1;
            let start = rm.start_address.unwrap_or(lowest);
            let finish = rm.finish_address.unwrap_or(highest);
            (start.min(finish), start.max(finish))
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum Digit {
        Known(u8),
        X,
        Z,
    }

    #[derive(Debug, Clone)]
    enum Item {
        /// Digits, most significant first, each with whether it is upper case and whether an
        /// underscore follows it.
        Word(Vec<(Digit, bool, bool)>),
        /// An index into the loaded address range.
        Address(u32),
    }

    const SEPARATORS: &[&str] = &[
        " ",
        "\t",
        "\n",
        "\r\n",
        "  \n\n ",
        "// comment @1 zz\n",
        "/* comment */",
        "/* two\nlines */",
        "/**/",
        "\n// two\n// comments\n",
    ];

    fn any_config() -> impl Strategy<Value = Config> {
        // (stride, limit, offset, extra): a signal under 33 bits packs both four-valued planes in
        // one value, so make sure those are generated as well.
        let geometry = prop_oneof![
            (1u32..=4, 1u32..=4, 0u32..=8, 0u32..=8),
            (
                prop_oneof![1u32..=8, 9u32..=70, 120u32..=135],
                1u32..=8,
                0u32..=70,
                0u32..=70,
            ),
        ];
        (
            prop_oneof![Just(LogicMode::TwoValue), Just(LogicMode::FourValue)],
            geometry,
            -3i64..=20,
            prop_oneof![
                Just(ReadMemVariant::Binary),
                Just(ReadMemVariant::Hexadecimal)
            ],
        )
            .prop_flat_map(
                |(mode, (stride, limit, offset, extra), base_address, variant)| {
                    let start_finish =
                        proptest::option::of((0..limit, proptest::option::of(0..limit)));
                    start_finish.prop_map(move |start_finish| Config {
                        mode,
                        stride,
                        limit,
                        offset,
                        extra,
                        base_address,
                        variant,
                        start_finish,
                    })
                },
            )
    }

    fn any_digit(variant: ReadMemVariant, mode: LogicMode) -> impl Strategy<Value = Digit> {
        let max: u8 = match variant {
            ReadMemVariant::Binary => 1,
            ReadMemVariant::Hexadecimal => 15,
        };
        let special = match mode {
            // Two-valued logic reads `x` and `z` as 0, keep them rare.
            LogicMode::TwoValue => 1,
            LogicMode::FourValue => 3,
        };
        prop_oneof![
            10 => (0..=max).prop_map(Digit::Known),
            special => Just(Digit::X),
            special => Just(Digit::Z),
        ]
    }

    fn any_item(config: &Config) -> impl Strategy<Value = Item> + use<> {
        // Words that are too short, exact and too long.
        let max_digits = config.stride.div_ceil(config.bits_per_digit()) as usize + 2;
        let (lo, hi) = config.range();
        let range_len = (hi - lo) as u32 + 1;
        let digit = (
            any_digit(config.variant, config.mode),
            any::<bool>(),
            prop::bool::weighted(0.1),
        );
        prop_oneof![
            8 => prop::collection::vec(digit, 1..=max_digits).prop_map(Item::Word),
            1 => (0..range_len).prop_map(Item::Address),
        ]
    }

    /// A configuration, the items in the file, the separators around the items, and the initial
    /// value of the signal.
    fn any_file() -> impl Strategy<Value = (Config, Vec<Item>, Vec<usize>, Vec<FvLogicValue>)> {
        any_config().prop_flat_map(|config| {
            // A file cannot hold a negative address.
            let (lo, _) = config.range();
            let items = prop::collection::vec(any_item(&config), 0..=config.limit as usize + 3)
                .prop_map(move |mut items| {
                    items.retain(|item| match item {
                        Item::Address(idx) => lo + i64::from(*idx) >= 0,
                        Item::Word(_) => true,
                    });
                    items
                });
            let init = prop::collection::vec(
                prop_oneof![
                    Just(FvLogicValue::L0),
                    Just(FvLogicValue::L1),
                    Just(FvLogicValue::X),
                    Just(FvLogicValue::Z),
                ],
                config.signal_size().get() as usize,
            );
            (Just(config), items, init).prop_flat_map(|(config, items, init)| {
                let seps = prop::collection::vec(0..SEPARATORS.len(), items.len() + 1);
                (Just(config), Just(items), seps, Just(init))
            })
        })
    }

    /// Renders the file, and returns the byte offset of every item.
    fn render(config: &Config, items: &[Item], seps: &[usize]) -> (String, Vec<usize>) {
        let (lo, _) = config.range();
        let mut s = String::new();
        let mut positions = Vec::new();
        for (i, item) in items.iter().enumerate() {
            s.push_str(SEPARATORS[seps[i]]);
            positions.push(s.len());
            match item {
                Item::Word(digits) => {
                    for &(digit, upper, underscore) in digits {
                        let c = match digit {
                            Digit::Known(d) => char::from_digit(u32::from(d), 16).unwrap(),
                            Digit::X => 'x',
                            Digit::Z => 'z',
                        };
                        s.push(if upper { c.to_ascii_uppercase() } else { c });
                        if underscore {
                            s.push('_');
                        }
                    }
                }
                Item::Address(idx) => s.push_str(&format!("@{:x}", lo + i64::from(*idx))),
            }
        }
        s.push_str(SEPARATORS[*seps.last().unwrap()]);
        (s, positions)
    }

    /// The value of a word in the file.
    fn model_word(config: &Config, digits: &[(Digit, bool, bool)]) -> Vec<FvLogicValue> {
        let bpd = config.bits_per_digit();
        (0..config.stride)
            .map(|i| {
                let Some(&(digit, _, _)) = digits.iter().rev().nth((i / bpd) as usize) else {
                    return FvLogicValue::L0;
                };
                let value = match digit {
                    Digit::Known(d) => FvLogicValue::from_bool((d >> (i % bpd)) & 1 != 0),
                    Digit::X => FvLogicValue::X,
                    Digit::Z => FvLogicValue::Z,
                };
                match config.mode {
                    LogicMode::TwoValue if value.is_special() => FvLogicValue::L0,
                    _ => value,
                }
            })
            .collect()
    }

    /// The signal after loading the file, and the number of words that were dropped.
    fn model(config: &Config, items: &[Item], init: &[FvLogicValue]) -> (Vec<FvLogicValue>, usize) {
        let rm = config.readmem();
        let (lo, hi) = config.range();
        let start = rm.start_address.unwrap_or(config.base_address);
        let descending = rm.finish_address.is_some_and(|f| f < start);

        let mut signal = init.to_vec();
        let mut addr = Some(start);
        let mut dropped = 0;
        for item in items {
            match item {
                Item::Address(idx) => addr = Some(lo + i64::from(*idx)),
                Item::Word(digits) => {
                    let Some(at) = addr else {
                        dropped += 1;
                        continue;
                    };
                    let elem = (at - config.base_address) as u32;
                    let base = (config.offset + elem * config.stride) as usize;
                    let word = model_word(config, digits);
                    signal[base..base + word.len()].copy_from_slice(&word);
                    addr = match descending {
                        false if at == hi => None,
                        true if at == lo => None,
                        false => Some(at + 1),
                        true => Some(at - 1),
                    };
                }
            }
        }
        (signal, dropped)
    }

    struct Setup {
        heap: Heap,
        before: HeapRef,
        signal: HeapRef,
        after: HeapRef,
        guard: Bits,
    }

    fn to_bits(mode: LogicMode, values: &[FvLogicValue]) -> Bits {
        let size = VectorSize::new(values.len() as u32).unwrap();
        let mut bits = match mode {
            LogicMode::TwoValue => Bits::new_zeroed(size),
            LogicMode::FourValue => Bits::new_fv_constant(size, FvLogicValue::L0),
        };
        for (i, &v) in values.iter().enumerate() {
            let v = match mode {
                LogicMode::TwoValue if v.is_special() => FvLogicValue::L0,
                _ => v,
            };
            bits.fill_range(i as u32..=i as u32, v);
        }
        bits
    }

    /// A heap with the signal between two guard signals of the same mode.
    fn setup(config: &Config, init: &[FvLogicValue]) -> Setup {
        let mut builder = HeapBuilder::new();
        let guard_values: Vec<_> = (0..13)
            .map(|i| [FvLogicValue::L1, FvLogicValue::Z, FvLogicValue::L0][i % 3])
            .collect();
        let guard = to_bits(config.mode, &guard_values);
        let before = builder.claim(config.mode, guard.size());
        let signal = builder.claim(config.mode, config.signal_size());
        let after = builder.claim(config.mode, guard.size());
        let mut heap = builder.finish();
        heap.store_bits(before, config.mode, &guard);
        heap.store_bits(signal, config.mode, &to_bits(config.mode, init));
        heap.store_bits(after, config.mode, &guard);
        Setup {
            heap,
            before,
            signal,
            after,
            guard,
        }
    }

    fn run(
        config: &Config,
        setup: &mut Setup,
        file: Vec<u8>,
    ) -> (Result<(), RuntimeError>, String) {
        let stderr = Rc::new(RefCell::new(Vec::new()));
        let mut world = FileWorld {
            file: Some(file),
            stderr: stderr.clone(),
        };
        let result = read_mem(
            &mut world,
            &mut setup.heap.0,
            setup.signal,
            config.mode.into(),
            &config.readmem(),
        );
        let stderr = String::from_utf8(stderr.take()).unwrap();
        (result, stderr)
    }

    fn assert_guards(setup: &Setup, mode: LogicMode) -> Result<(), TestCaseError> {
        prop_assert_eq!(
            setup.heap.load_bits(setup.before, mode),
            setup.guard.clone()
        );
        prop_assert_eq!(setup.heap.load_bits(setup.after, mode), setup.guard.clone());
        Ok(())
    }

    proptest! {
        #[test]
        fn proptest_readmem_matches_model((config, items, seps, init) in any_file()) {
            let (file, _) = render(&config, &items, &seps);
            let mut setup = setup(&config, &init);
            let (result, stderr) = run(&config, &mut setup, file.into_bytes());
            prop_assert!(result.is_ok(), "{}", result.unwrap_err());

            let (expected, dropped) = model(&config, &items, &init);
            let gotten = setup.heap.load_bits(setup.signal, config.mode);
            for (i, &expected) in expected.iter().enumerate() {
                let expected = match config.mode {
                    LogicMode::TwoValue if expected.is_special() => FvLogicValue::L0,
                    _ => expected,
                };
                prop_assert_eq!(gotten.select_value(i as u32), expected, "bit {}", i);
            }
            assert_guards(&setup, config.mode)?;
            prop_assert_eq!(stderr.contains("do not fit"), dropped > 0, "{}", stderr);
        }
    }

    proptest! {
        #[test]
        fn proptest_readmem_reports_invalid_digit(
            (config, items, seps, init) in any_file().prop_filter(
                "a word to corrupt",
                |(_, items, _, _)| items.iter().any(|item| matches!(item, Item::Word(_))),
            ),
            which in any::<prop::sample::Index>(),
        ) {
            let (file, positions) = render(&config, &items, &seps);
            let words: Vec<usize> = items
                .iter()
                .zip(&positions)
                .filter_map(|(item, &pos)| matches!(item, Item::Word(_)).then_some(pos))
                .collect();

            // Replace the first digit of a word with one that is not valid in its base.
            let pos = words[which.index(words.len())];
            let invalid = match config.variant {
                ReadMemVariant::Binary => '2',
                ReadMemVariant::Hexadecimal => 'g',
            };
            let mut file = file.into_bytes();
            file[pos] = invalid as u8;

            let mut setup = setup(&config, &init);
            let (result, _) = run(&config, &mut setup, file.clone());
            let err = result.unwrap_err();
            let line = 1 + file[..pos].iter().filter(|&&c| c == b'\n').count() as u32;
            let RuntimeError::ReadMem(err) = err else {
                return Err(TestCaseError::fail("error is incorrect"));
            };
            prop_assert!(matches!(err.kind, ReadMemErrorKind::InvalidDigit(c) if c == invalid), "{}", err);
            prop_assert_eq!(err.line, Some(line));
            assert_guards(&setup, config.mode)?;
        }
    }

    proptest! {
        #[test]
        fn proptest_readmem_arbitrary_file(
            config in any_config(),
            file in prop::collection::vec(
                prop::sample::select(b"0123456789abcdefABCDEFxXzZ_@/*? \t\r\ng\xff".to_vec()),
                0..200,
            ),
        ) {
            let init: Vec<_> = (0..config.signal_size().get())
                .map(|i| [FvLogicValue::Z, FvLogicValue::L1, FvLogicValue::X, FvLogicValue::L0][i as usize % 4])
                .collect();
            let mut setup = setup(&config, &init);
            _ = run(&config, &mut setup, file);

            // Whatever the file holds, only the memory is written.
            let gotten = setup.heap.load_bits(setup.signal, config.mode);
            let init = to_bits(config.mode, &init);
            let memory = config.offset..config.offset + config.limit * config.stride;
            for i in (0..config.signal_size().get()).filter(|i| !memory.contains(i)) {
                prop_assert_eq!(gotten.select_value(i), init.select_value(i), "bit {}", i);
            }
            assert_guards(&setup, config.mode)?;
        }
    }

    #[test]
    fn missing_file() {
        let config = Config {
            mode: LogicMode::FourValue,
            stride: 8,
            limit: 4,
            offset: 0,
            extra: 0,
            base_address: 0,
            variant: ReadMemVariant::Hexadecimal,
            start_finish: None,
        };
        let mut setup = setup(&config, &[FvLogicValue::X; 32]);
        let mut world = FileWorld {
            file: None,
            stderr: Rc::default(),
        };
        let err = read_mem(
            &mut world,
            &mut setup.heap.0,
            setup.signal,
            config.mode.into(),
            &config.readmem(),
        )
        .unwrap_err();
        let RuntimeError::ReadMem(err) = err else {
            panic!("expected a readmem error, got {err:?}");
        };
        assert!(matches!(err.kind, ReadMemErrorKind::Open(_)));
        assert_eq!(
            err.to_string(),
            "$readmemh: `mem.txt`: failed to open file: entity not found"
        );
    }
}

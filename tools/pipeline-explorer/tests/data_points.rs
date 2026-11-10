use pipeline_explorer::{
    DataPoints, Ibex, IbexConfig, PicoRV32Config, PicoRv32, Pipeline, Registers, ReturnData,
    Symbols, evaluate,
};

const PROGRAM: &str = r#"
.section .text
.global _start
_start:
    li a0, 7
    li a1, 9
    add a2, a0, a1
loop:
    j loop
"#;

fn config() -> PicoRV32Config {
    PicoRV32Config {
        enable_mul: true,
        enable_div: true,
        two_stage_shift: false,
        barrel_shifter: false,
        two_cycle_compare: false,
        two_cycle_alu: false,
        enable_fast_mul: false,
    }
}

#[test]
fn picorv32_reports_registers() {
    let out = evaluate(&PicoRv32, &config(), PROGRAM, 300, DataPoints::ALL).expect("run");
    let mut saw_pipeline = false;
    let mut regs = None;
    for d in &out.data {
        match d {
            ReturnData::Pipeline(_) => saw_pipeline = true,
            ReturnData::Registers(r) => regs = Some(r),
            ReturnData::Symbols(_) => {}
        }
    }
    assert!(saw_pipeline, "pipeline missing");
    let regs = regs.expect("registers missing");
    assert_eq!(regs.names.len(), 32);
    assert_eq!(regs.values.len(), 32);

    let last = |i: usize| *regs.values[i].last().expect("a cycle");
    assert_eq!(last(0), 0, "x0 must read zero");
    assert_eq!(last(10), 7, "a0");
    assert_eq!(last(11), 9, "a1");
    assert_eq!(last(12), 16, "a2 = a0 + a1");
}

#[test]
fn pipeline_only_omits_registers() {
    let out = evaluate(&PicoRv32, &config(), PROGRAM, 50, DataPoints::PIPELINE).expect("run");
    assert_eq!(out.data.len(), 1);
    assert!(matches!(out.data[0], ReturnData::Pipeline(_)));
}

fn registers_of(data: &[ReturnData]) -> Option<&Registers> {
    data.iter().find_map(|d| match d {
        ReturnData::Registers(r) => Some(r),
        _ => None,
    })
}

#[test]
fn ibex_reports_registers() {
    let out = evaluate(
        &Ibex,
        &IbexConfig { wb_stage: false },
        PROGRAM,
        400,
        DataPoints::ALL,
    )
    .expect("run");
    let regs = registers_of(&out.data).expect("ibex registers missing");
    let last = |i: usize| *regs.values[i].last().expect("a cycle");
    assert_eq!(last(0), 0, "x0 must read zero");
    assert_eq!(last(10), 7, "a0");
    assert_eq!(last(11), 9, "a1");
    assert_eq!(last(12), 16, "a2 = a0 + a1");
}

fn symbols_of(data: &[ReturnData]) -> Option<&Symbols> {
    data.iter().find_map(|d| match d {
        ReturnData::Symbols(s) => Some(s),
        _ => None,
    })
}

const LABELLED: &str = r#"
.section .data
value:  .word 1

.section .text
.global _start
_start:
    li a0, 7
loop:
    j loop
"#;

fn pipeline_of(data: &[ReturnData]) -> Option<&Pipeline> {
    data.iter().find_map(|d| match d {
        ReturnData::Pipeline(p) => Some(p),
        _ => None,
    })
}

#[test]
fn reports_labels_and_globals() {
    let out = evaluate(&PicoRv32, &config(), LABELLED, 20, DataPoints::SYMBOLS).expect("run");
    // A static point: it needs no pipeline or register collection.
    assert_eq!(out.data.len(), 1);
    let syms = symbols_of(&out.data).expect("symbols missing");

    let find = |name: &str| {
        let i = syms.names.iter().position(|n| n == name)?;
        Some((syms.addresses[i], syms.sections[i], syms.globals[i] == 1))
    };

    let (start_addr, start_sect, start_global) = find("_start").expect("_start");
    let (loop_addr, loop_sect, loop_global) = find("loop").expect("loop");
    let (value_addr, value_sect, value_global) = find("value").expect("value");

    assert_eq!(start_sect, ".text");
    assert_eq!(loop_sect, ".text");
    assert_eq!(value_sect, ".data");
    // `.global _start` marks visibility only; the others stay local.
    assert!(start_global, "_start is global");
    assert!(!loop_global, "loop is local");
    assert!(!value_global, "value is local");
    // `_start:` defines the label, so it sits at the top of .text even though
    // the `.global` directive came first.
    assert!(loop_addr > start_addr, "loop follows _start");
    assert_ne!(value_addr, start_addr, "data is not text");

    // Sorted by address, as documented.
    let mut sorted = syms.addresses.clone();
    sorted.sort_unstable();
    assert_eq!(syms.addresses, sorted);
}

/// Symbol addresses are absolute and must agree with the pipeline's
/// `text_addr`, or nothing can map a traced slot back to a symbol.
///
/// PicoRV32 alone cannot catch this: its `.text` base is 0, so adding the
/// section base twice is invisible. Ibex boots at 0x80, which exposes it.
#[test]
fn symbol_addresses_line_up_with_the_pipeline() {
    let runs: Vec<(&str, Vec<ReturnData>)> = vec![
        (
            "picorv32",
            evaluate(&PicoRv32, &config(), LABELLED, 60, DataPoints::ALL)
                .expect("picorv32")
                .data,
        ),
        (
            "ibex",
            evaluate(&Ibex, &IbexConfig { wb_stage: false }, LABELLED, 60, DataPoints::ALL)
                .expect("ibex")
                .data,
        ),
    ];

    for (name, data) in &runs {
        let syms = symbols_of(data).unwrap_or_else(|| panic!("{name}: symbols"));
        let pipeline = pipeline_of(data).unwrap_or_else(|| panic!("{name}: pipeline"));

        let at = |label: &str| {
            let i = syms.names.iter().position(|n| n == label)?;
            Some(syms.addresses[i])
        };
        let start = at("_start").unwrap_or_else(|| panic!("{name}: _start"));

        // `_start:` is the first thing in `.text`, so it is slot 1's address.
        assert_eq!(
            start, pipeline.text_addr,
            "{name}: _start 0x{start:x} should be the pipeline's text_addr 0x{:x}",
            pipeline.text_addr,
        );

        // Every traced slot must land on an instruction of the program.
        let text_end = pipeline.text_addr + 4 * 64;
        for trace in &pipeline.traces {
            for slot in trace.iter().copied().filter(|s| *s != 0) {
                let addr = pipeline.text_addr + 4 * (slot - 1);
                assert!(addr >= start && addr < text_end, "{name}: slot {slot} out of range");
            }
        }

        // `.data` sits at its own base, not at twice it.
        let value = at("value").unwrap_or_else(|| panic!("{name}: value"));
        assert!(value >= 1024, "{name}: value 0x{value:x} is in data memory");
        assert!(value < 1024 + 1001 * 4, "{name}: value 0x{value:x} within data memory");
    }
}

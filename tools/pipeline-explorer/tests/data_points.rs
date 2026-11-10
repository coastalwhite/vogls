use pipeline_explorer::{
    DataPoints, Ibex, IbexConfig, PicoRV32Config, PicoRv32, Registers, ReturnData, evaluate,
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

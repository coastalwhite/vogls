use crate::design::Arena;
use crate::design::{Design, DesignState};
use crate::elaborated_design::SignalHandle;
use crate::ir::{Bits, LogicMode, VectorSize};
use crate::{DesignBuilder, StdWorld};

#[test]
fn constant_signal_7() -> Result<(), Box<dyn std::error::Error>> {
    macro_rules! bail {
        ($msg:expr) => {
            return Err($msg.to_string().into())
        };
    }
    macro_rules! anyhow {
        ($msg:expr) => {
            <Box<dyn std::error::Error> as From<String>>::from($msg.to_string())
        };
    }

    let arena = Arena::new();

    let mut db = DesignBuilder::new();
    db.add_source_str(
        r#"
module top(output__);
  output [7:0] output__;
  wire [7:0] output__;
  assign output__ = 8'h05;
endmodule
        "#,
    )?;

    let parsed = db.parse(&arena)?;

    let mut elaborated = match parsed.elaborate(LogicMode::FourValue, Some("top")) {
        Ok(elaborated) => elaborated,
        Err(_) => bail!("Elaboration error"),
    };

    let symtab = elaborated.table();
    let identtable = elaborated.ident_table();

    let top = symtab
        .resolve_root(
            identtable
                .get("top")
                .ok_or_else(|| anyhow!("Did not find top"))?,
        )
        .ok_or_else(|| anyhow!("Did not find top in symtab"))?;

    let result_id = symtab
        .resolve(
            top,
            identtable
                .get("output__")
                .ok_or_else(|| anyhow!("Did not find output__"))?,
        )
        .ok_or_else(|| anyhow!("Did not find output__ from ID in symtab"))?;

    let result_handle = elaborated
        .get_signal_handle(result_id)
        .ok_or_else(|| anyhow!("Did not find output__ in elaborated design"))?;

    let lowered = match elaborated.lower(vec![]) {
        Ok(lowered) => lowered,
        Err(_) => bail!("Lowering error"),
    };

    let (design, mut state) = lowered
        .to_bytecode()
        .map_err(|_| anyhow!("to_bytecode error"))?;

    let rt = design.resolve_handle(result_handle);

    design.run(&mut state, &mut StdWorld::new(), 1)?;

    assert_eq!(
        design.get_signal(&state, rt),
        Bits::from_u64(VectorSize::new(8).unwrap(), 0x05)
    );
    Ok(())
}

/// Lowers `source` with a handle on every signal, as the trace plugin takes, and returns the
/// handles of the signals `names` of module `top`.
fn lower_with_handles<const N: usize>(
    source: &str,
    mode: LogicMode,
    names: [&str; N],
) -> Result<(Design, DesignState, [SignalHandle; N]), Box<dyn std::error::Error>> {
    let arena = Arena::new();
    let mut db = DesignBuilder::new();
    db.add_source_str(source)?;
    let parsed = db.parse(&arena)?;
    let Ok(mut elaborated) = parsed.elaborate(mode, Some("top")) else {
        return Err("elaboration error".into());
    };

    let symbols = elaborated.table().symbol_id_iter().collect::<Vec<_>>();
    for symbol in symbols {
        elaborated.get_signal_handle(symbol);
    }
    let symtab = elaborated.table();
    let idents = elaborated.ident_table();
    let top = symtab.resolve_root(idents.get("top").unwrap()).unwrap();
    let symbols = names.map(|name| symtab.resolve(top, idents.get(name).unwrap()).unwrap());
    let handles = symbols.map(|symbol| elaborated.get_signal_handle(symbol).unwrap());

    let Ok(lowered) = elaborated.lower(vec![]) else {
        return Err("lowering error".into());
    };
    let (design, state) = lowered.to_bytecode().map_err(|_| "to_bytecode error")?;
    Ok((design, state, handles))
}

/// The value of `handle` as text, which compares values regardless of their representation.
fn value_of(design: &Design, state: &DesignState, handle: SignalHandle) -> String {
    design
        .get_signal(state, design.resolve_handle(handle))
        .to_string()
}

fn bits(width: u32, value: u64) -> Bits {
    Bits::from_u64(VectorSize::new(width).unwrap(), value)
}

/// A handle on a `tran` member does not drive the closure: in two-valued logic, where nothing is
/// `z`, a closure that resolved the handle as a driver would read `0` instead of its driver's `1`.
#[test]
#[ignore = "two-valued `tran` members drive `0` where they should not drive: their drive proxies need to be four-valued (see the TODO in `lower_bidirectional`)"]
fn two_value_tran_with_handle() -> Result<(), Box<dyn std::error::Error>> {
    let (design, mut state, [t1, t2]) = lower_with_handles(
        r#"
module top;
  wire t1, t2;
  assign t1 = 1'b1;
  tran t (t1, t2);
endmodule
        "#,
        LogicMode::TwoValue,
        ["t1", "t2"],
    )?;
    design.run(&mut state, &mut StdWorld::new(), 1)?;
    assert_eq!(value_of(&design, &state, t1), bits(1, 1).to_string());
    assert_eq!(value_of(&design, &state, t2), bits(1, 1).to_string());
    Ok(())
}

/// A bus that a child drives through an `inout` port reads the child's value in two-valued logic,
/// with a handle on every signal.
#[test]
#[ignore = "two-valued `tran` members drive `0` where they should not drive: their drive proxies need to be four-valued (see the TODO in `lower_bidirectional`)"]
fn two_value_inout_bus_with_handles() -> Result<(), Box<dyn std::error::Error>> {
    let (design, mut state, [bus]) = lower_with_handles(
        r#"
module child (inout [1:0] p);
  assign p = 2'b11;
endmodule

module top;
  wire [1:0] bus;
  child c (bus);
endmodule
        "#,
        LogicMode::TwoValue,
        ["bus"],
    )?;
    design.run(&mut state, &mut StdWorld::new(), 1)?;
    assert_eq!(value_of(&design, &state, bus), bits(2, 0b11).to_string());
    Ok(())
}

/// External writes of `tran` members behave as in Icarus Verilog, where a write sets the value that
/// the member itself drives into the `tran`. The expected values were measured with Icarus Verilog
/// 12 and `vpi_put_value` (both `vpiNoDelay` and `vpiInertialDelay`):
///
/// - A: An undriven `tran`: the write reaches the other side.
/// - B: A write of the undriven side: it is one more driver of the `tran` and keeps conflicting
///   with the driver of the other side, even after the driver changes.
/// - C: A write of the driven side: the driver overwrites it when it changes.
/// - D: The net of a child's `inout` port: the child observes the write.
/// - E: A partially connected net: the driven bit keeps the write until its driver changes, the
///   connected bit keeps it.
///
/// In two-valued logic, `x` reads as `0`.
#[test]
fn external_writes_through_tran_as_icarus() -> Result<(), Box<dyn std::error::Error>> {
    external_writes_through_tran(LogicMode::FourValue)
}

/// See [`external_writes_through_tran_as_icarus`].
#[test]
#[ignore = "two-valued `tran` members drive `0` where they should not drive: their drive proxies need to be four-valued (see the TODO in `lower_bidirectional`)"]
fn two_value_external_writes_through_tran_as_icarus() -> Result<(), Box<dyn std::error::Error>> {
    external_writes_through_tran(LogicMode::TwoValue)
}

fn external_writes_through_tran(mode: LogicMode) -> Result<(), Box<dyn std::error::Error>> {
    const SOURCE: &str = r#"
module child (inout p, output q);
  assign q = p;
endmodule

module top;
  reg y;
  wire a1, a2;
  tran ta (a1, a2);
  wire b1, b2;
  assign b1 = y;
  tran tb (b1, b2);
  wire c1, c2;
  assign c2 = y;
  tran tc (c1, c2);
  wire bus, q;
  child u (bus, q);
  wire [1:0] v;
  wire s;
  assign v[0] = y;
  tran te (v[1], s);
  initial y = 1'b1;
endmodule
"#;
    const NAMES: [&str; 11] = [
        "y", "a1", "a2", "b1", "b2", "c1", "c2", "bus", "q", "v", "s",
    ];

    {
        let (design, mut state, handles) = lower_with_handles(SOURCE, mode, NAMES)?;
        let [y, a1, a2, b1, b2, c1, c2, bus, q, v, s] = handles;
        let observe = |design: &Design, state: &DesignState| {
            [a1, a2, b1, b2, c1, c2, bus, q, v, s]
                .map(|h| value_of(design, state, h))
                .join(" ")
        };
        // The values as `[width]'h..`, or `[width]'b..` with `x` and `z`. `x` is `0` in
        // two-valued logic.
        let row = |values: [&str; 10]| {
            let widths = [1, 1, 1, 1, 1, 1, 1, 1, 2, 1];
            values
                .iter()
                .zip(widths)
                .map(|(value, width)| {
                    let value = match mode {
                        LogicMode::FourValue => value.to_string(),
                        LogicMode::TwoValue => value.replace('x', "0"),
                    };
                    if value.contains(['x', 'z']) {
                        format!("{width}'b{value}")
                    } else {
                        bits(width, u64::from_str_radix(&value, 2).unwrap()).to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        };

        design.run(&mut state, &mut StdWorld::new(), 10)?;
        design.write_handle(&mut state, a2, &bits(1, 1));
        design.write_handle(&mut state, b2, &bits(1, 0));
        design.write_handle(&mut state, c2, &bits(1, 0));
        design.write_handle(&mut state, bus, &bits(1, 1));
        design.write_handle(&mut state, v, &bits(2, 0b10));
        design.run(&mut state, &mut StdWorld::new(), 1)?;
        //                    a1   a2   b1   b2   c1   c2   bus  q    v     s
        let written = row(["1", "1", "x", "x", "0", "0", "1", "1", "10", "1"]);
        assert_eq!(
            observe(&design, &state),
            written,
            "{mode:?}: after the writes"
        );

        design.write_handle(&mut state, y, &bits(1, 0));
        design.run(&mut state, &mut StdWorld::new(), 1)?;
        let y0 = row(["1", "1", "0", "0", "0", "0", "1", "1", "10", "1"]);
        assert_eq!(observe(&design, &state), y0, "{mode:?}: after y = 0");

        design.write_handle(&mut state, y, &bits(1, 1));
        design.run(&mut state, &mut StdWorld::new(), 1)?;
        let y1 = row(["1", "1", "x", "x", "1", "1", "1", "1", "11", "1"]);
        assert_eq!(observe(&design, &state), y1, "{mode:?}: after y = 1");
    }
    Ok(())
}

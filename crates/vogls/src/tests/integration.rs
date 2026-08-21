use crate::design::Arena;
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

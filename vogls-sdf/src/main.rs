use vogls_sdf::{Consume, TokenWalker};

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let s = std::fs::read_to_string(
        "/home/johndoe/Projects/tinytapeout-ihp-canright/runs/wokwi/final/sdf/nom_fast_1p32V_m40C/tt_um_coastalwhite_canright_sbox__nom_fast_1p32V_m40C.sdf",
    )?;
    let mut tkw = TokenWalker::new(&s);
    let s = vogls_sdf::DelayFile::consume(&mut tkw)?;

    Ok(())
}

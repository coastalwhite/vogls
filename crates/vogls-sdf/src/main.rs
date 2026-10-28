use vogls_frontend::ident_table::IdentTable;
use vogls_sdf::{Consume, TokenWalker};

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let s = std::fs::read_to_string("/home/johndoe/Projects/vogls/vogls-sdf/example1.sdf")?;
    let mut ident_table = IdentTable::default();
    let mut tkw = TokenWalker::new(&s, &mut ident_table);
    let _ = vogls_sdf::DelayFile::consume(&mut tkw)?;

    Ok(())
}

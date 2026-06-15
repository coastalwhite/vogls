use vogls_sdf::{Consume, TokenWalker};

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let s = std::fs::read_to_string("/home/johndoe/Projects/vogls/vogls-sdf/example1.sdf")?;
    let mut tkw = TokenWalker::new(&s);
    let _ = vogls_sdf::DelayFile::consume(&mut tkw)?;

    Ok(())
}

use rand_core::{OsRng, RngCore};

pub fn run() -> anyhow::Result<()> {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    println!("{}", hex::encode(bytes));
    Ok(())
}

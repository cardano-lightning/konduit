use rand::{TryRngCore, rngs::OsRng};

/// Generate a new Ed25519 private key.
#[derive(Debug, clap::Args)]
#[clap(disable_version_flag(true))]
pub(crate) struct Args {}

impl Args {
    pub(crate) fn execute(self) -> anyhow::Result<()> {
        let mut key = [0u8; 32];
        OsRng.try_fill_bytes(&mut key).unwrap();
        println!("{}", hex::encode(key));
        Ok(())
    }
}

use std::io::{self, Write};
use std::str::FromStr;

use anyhow::{anyhow, bail};
use cardano_tx_builder::cbor::ToCbor;
use cardano_tx_builder::{PlutusData, SigningKey};
use konduit_data::{Cheque, ChequeBody, Duration, Lock, MixedCheque, Secret, Tag, Unlocked};
use konduit_tx::Lovelace;

use crate::{env, metavar};

fn parse_duration(s: &str) -> anyhow::Result<Duration> {
    Duration::from_str(s).map_err(|e| anyhow!(e))
}

fn parse_lock(s: &str) -> anyhow::Result<Lock> {
    Lock::from_str(s).map_err(|e| anyhow!(e))
}

fn parse_secret(s: &str) -> anyhow::Result<Secret> {
    Secret::from_str(s).map_err(|e| anyhow!(e))
}

#[derive(Debug, clap::Args)]
#[clap(disable_version_flag(true))]
pub(crate) struct Args {
    /// Wallet's signing key; provide either this or --verification-key
    #[clap(
        long,
        value_name = metavar::ED25519_SIGNING_KEY,
        env = env::WALLET_SIGNING_KEY
    )]
    signing_key: SigningKey,

    #[clap(long, value_name = metavar::BYTES_32, env = env::CHANNEL_TAG)]
    channel_tag: Tag,

    #[clap(long, value_name = "INDEX")]
    index: u64,

    #[clap(long, value_name = metavar::LOVELACE)]
    amount: Lovelace,

    #[clap(
        long,
        value_name = "DURATION",
        help = "Timeout duration (e.g., '10s', '5min', '1h')",
        value_parser = parse_duration
    )]
    timeout: Duration,

    #[clap(
        long,
        value_name = "HEX32",
        help = "Lock hash (32-byte hex) for Cheque variant",
        value_parser = parse_lock,
        conflicts_with = "secret"
    )]
    lock: Option<Lock>,

    #[clap(
        long,
        value_name = "HEX32",
        help = "Secret (32-byte hex) for Unlocked variant (computes lock from secret)",
        value_parser = parse_secret,
        conflicts_with = "lock"
    )]
    secret: Option<Secret>,
}

impl Args {
    pub(crate) fn execute(self) -> anyhow::Result<()> {
        let mixed_cheque = match (self.lock, self.secret) {
            (Some(lock), None) => {
                let cheque_body = ChequeBody::new(self.index, self.amount, self.timeout, lock);
                let cheque = Cheque::make(&self.signing_key, &self.channel_tag, cheque_body);
                MixedCheque::Cheque(cheque)
            }
            (None, Some(secret)) => {
                let lock = Lock::from(secret.clone());
                let cheque_body = ChequeBody::new(self.index, self.amount, self.timeout, lock);
                let cheque = Cheque::make(&self.signing_key, &self.channel_tag, cheque_body);
                let unlocked = Unlocked::new(cheque, secret)?;
                MixedCheque::Unlocked(unlocked)
            }
            _ => bail!("Provide exactly one of --lock or --secret"),
        };

        let bytes = PlutusData::from(mixed_cheque).to_cbor();
        let mut stdout = io::stdout();
        stdout.write_all(&bytes)?;
        stdout.flush()?;

        Ok(())
    }
}

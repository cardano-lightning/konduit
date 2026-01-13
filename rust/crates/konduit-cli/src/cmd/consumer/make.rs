use std::time::{SystemTime, UNIX_EPOCH};

use cardano_tx_builder::{PlutusData, cbor::ToCbor};
use konduit_data::{ChequeBody, Duration, Indexes, Lock, Locked, Secret, Squash, SquashBody, Tag};

use crate::config::consumer::Config;

fn duration_from_relative(duration: Duration) -> Duration {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();
    Duration::from_secs(now + duration.as_secs())
}

/// Show
#[derive(clap::Subcommand)]
pub enum Cmd {
    /// Make a squash
    Squash {
        /// Channel tag
        #[arg(long)]
        tag: Tag,
        /// Total amount squashed (in lovelace)
        #[arg(long)]
        amount: u64,
        /// Greatest index included in squash
        #[arg(long)]
        index: u64,
        /// (Ordered) list of indexes excluded
        #[arg(long, default_value = "")]
        exclude: Indexes,
    },
    /// Make a locked cheque
    Locked {
        /// Channel tag
        #[arg(long)]
        tag: Tag,
        /// Cheque index
        #[arg(long)]
        index: u64,
        /// Amount (in lovelace)
        #[arg(long)]
        amount: u64,
        /// Timeout (absolute since POSIX)
        #[arg(long, required_unless_present = "duration")]
        timeout: Option<Duration>,
        /// Timeout (relative from now ie system time)
        #[arg(long, required_unless_present = "timeout")]
        duration: Option<Duration>,
        /// Lock
        #[arg(long, required_unless_present = "secret")]
        lock: Option<Lock>,
        /// Lock
        #[arg(long, required_unless_present = "lock")]
        secret: Option<Secret>,
    },
}

impl Cmd {
    pub(crate) fn run(self, config: &Config) -> anyhow::Result<()> {
        match self {
            Cmd::Squash {
                tag,
                amount,
                index,
                exclude,
            } => {
                let body = SquashBody::new(amount, index, exclude)?;
                let squash = Squash::make(
                    &cardano_tx_builder::SigningKey::from(config.wallet.clone()),
                    &tag,
                    body,
                );
                println!(
                    "{},{}",
                    hex::encode(PlutusData::from(squash.body).to_cbor()),
                    squash.signature
                );
                Ok(())
            }
            Cmd::Locked {
                tag,
                index,
                amount,
                timeout,
                duration,
                lock,
                secret,
            } => {
                let lock = lock
                    .or_else(|| secret.map(Lock::from))
                    .ok_or(anyhow::anyhow!("lock or secret required"))?;
                let timeout = timeout
                    .or_else(|| duration.map(duration_from_relative))
                    .ok_or(anyhow::anyhow!("timeout or duration required"))?;
                let body = ChequeBody::new(index, amount, timeout, lock);
                let locked = Locked::make(
                    &cardano_tx_builder::SigningKey::from(config.wallet.clone()),
                    &tag,
                    body,
                );
                println!(
                    "{},{}",
                    hex::encode(PlutusData::from(locked.body).to_cbor()),
                    locked.signature
                );
                Ok(())
            }
        }
    }
}

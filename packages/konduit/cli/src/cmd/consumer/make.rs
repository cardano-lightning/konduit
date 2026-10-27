use cardano_sdk::PosixMilliseconds;
use konduit_data::{ChequeBody, Duration, Indexes, Lock, Locked, Secret, Squash, SquashBody, Tag};
use konduit_tmp::to_signing_key;

use crate::config::consumer::Config;
use crate::output::{self, OutputJson};

fn duration_from_relative(duration: Duration) -> Duration {
    let now = PosixMilliseconds::now();
    Duration::from_millis(now.as_millis().saturating_add(duration.as_millis() as u64))
}

/// Show
#[derive(Debug, clap::Subcommand)]
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
        #[arg(long, default_value_t = false)]
        output_json: bool,
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
        #[arg(long, default_value_t = false)]
        output_json: bool,
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
                output_json,
            } => run_squash(
                config,
                &tag,
                amount,
                index,
                exclude,
                OutputJson(output_json),
            ),
            Cmd::Locked {
                tag,
                index,
                amount,
                timeout,
                duration,
                lock,
                secret,
                output_json,
            } => run_locked(
                config,
                &tag,
                index,
                amount,
                timeout,
                duration,
                lock,
                secret,
                OutputJson(output_json),
            ),
        }
    }
}

fn run_squash(
    config: &Config,
    tag: &Tag,
    amount: u64,
    index: u64,
    exclude: Indexes,
    output: OutputJson,
) -> anyhow::Result<()> {
    let body = SquashBody::new(amount, index, exclude)?;
    let squash = Squash::make(&to_signing_key(config.wallet.clone()), tag, body);
    if output.is_json() {
        let dto = output::SquashOutput::from_squash(&squash.into_unverified())?;
        output::render(output, &dto)?;
    } else {
        let squash = squash.into_unverified();
        println!(
            "{},{}",
            hex::encode(minicbor::to_vec(squash.body()).unwrap()),
            hex::encode(squash.signature())
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_locked(
    config: &Config,
    tag: &Tag,
    index: u64,
    amount: u64,
    timeout: Option<Duration>,
    duration: Option<Duration>,
    lock: Option<Lock>,
    secret: Option<Secret>,
    output: OutputJson,
) -> anyhow::Result<()> {
    let lock = lock
        .or_else(|| secret.map(Lock::from))
        .ok_or(anyhow::anyhow!("lock or secret required"))?;
    let timeout = match (timeout, duration) {
        (Some(timeout), _) => timeout,
        (None, Some(duration)) => duration_from_relative(duration),
        (None, None) => return Err(anyhow::anyhow!("timeout or duration required")),
    };
    let body = ChequeBody::new(index, amount, timeout, lock);
    let locked = Locked::make(&to_signing_key(config.wallet.clone()), tag, body);
    if output.is_json() {
        let dto = output::LockedOutput::from_locked(&locked.into_unverified())?;
        output::render(output, &dto)?;
    } else {
        let locked = locked.into_unverified();
        println!(
            "{},{}",
            hex::encode(minicbor::to_vec(locked.body()).unwrap()),
            hex::encode(locked.signature())
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::test_helpers;
    use konduit_data::Duration;
    use konduit_tmp::Keytag;

    fn test_tag() -> Tag {
        let config = test_helpers::consumer();
        let key = config.wallet.to_verification_key();
        let keytag = Keytag::new(&key, &Tag::from(vec![1; 8]));
        let (_key, tag) = keytag.split();
        tag
    }

    #[test]
    fn squash_json_roundtrip() {
        let config = test_helpers::consumer();
        let tag = test_tag();
        let exclude = Indexes::new(vec![2, 4]).unwrap();
        let body = SquashBody::new(1000, 5, exclude).unwrap();
        let signed = Squash::make(&to_signing_key(config.wallet.clone()), &tag, body);
        let dto = output::SquashOutput::from_squash(&signed.into_unverified()).unwrap();
        let json = serde_json::to_string(&dto).unwrap();
        let back: output::SquashOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(dto, back);
    }

    #[test]
    fn locked_json_roundtrip() {
        let config = test_helpers::consumer();
        let tag = test_tag();
        let lock = Lock([7; 32]);
        let body = ChequeBody::new(3, 5000, Duration::from_secs(123), lock);
        let locked = Locked::make(&to_signing_key(config.wallet.clone()), &tag, body);
        let dto = output::LockedOutput::from_locked(&locked.into_unverified()).unwrap();
        let json = serde_json::to_string(&dto).unwrap();
        let back: output::LockedOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(dto, back);
    }
}

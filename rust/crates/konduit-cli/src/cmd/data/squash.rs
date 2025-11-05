use cardano_tx_builder::SigningKey;
use konduit_tx::Lovelace;

use crate::{env, metavar};

// konduit_data which we want to build
// pub struct ChequeBody {
//     pub index: u64,
//     pub amount: u64,
//     pub timeout: Duration,
//     pub lock: Lock,
// }
//
// pub struct Cheque {
//     pub cheque_body: ChequeBody,
//     pub signature: Signature,
// }
//
// pub struct SquashBody {
//     pub amount: u64,
//     pub index: u64,
//     pub exclude: Vec<u64>,
// }
//
// Now the pieces needed to build a Squash
// pub struct SquashBody {
//     pub amount: u64,
//     pub index: u64,
//     pub exclude: Vec<u64>,
// }
// pub struct Squash {
//     pub squash_body: SquashBody,
//     pub signature: Signature,
// }
//
fn parse_exclude_list(s: &str) -> anyhow::Result<Vec<u64>> {
    s.split(',')
        .map(|part| part.trim().parse::<u64>().map_err(|e| anyhow::anyhow!(e)))
        .collect()
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
    channel_tag: konduit_data::Tag,

    #[clap(long, value_name = metavar::LOVELACE)]
    amount: Lovelace,

    #[clap(long, value_name = "INDEX")]
    index: u64,

    #[clap(
        long,
        value_name = "EXCLUDE",
        help = "Comma separated list of indexes to exclude",
        value_parser = parse_exclude_list
    )]
    exclude: Vec<u64>,
}

impl Args {
    pub(crate) fn execute(self) -> anyhow::Result<()> {
        // Squash has this helper:
        // pub fn make(signing_key: SigningKey, tag: Vec<u8>, squash_body: SquashBody) -> Self {
        // let exclude_list = parse_exclude_list(&self.exclude)?;
        let squash_body = konduit_data::SquashBody {
            amount: self.amount,
            index: self.index,
            exclude: self.exclude,
        };
        let squash =
            konduit_data::Squash::make(self.signing_key, self.channel_tag.0.to_vec(), squash_body);
        println!("{:?}", squash);
        Ok(())
    }
}

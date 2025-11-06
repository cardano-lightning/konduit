use std::io::{self, Write};

use cardano_tx_builder::cbor::ToCbor;
use cardano_tx_builder::{PlutusData, SigningKey};
use konduit_data::Indexes;
use konduit_tx::Lovelace;

use crate::{env, metavar};

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
    exclude: Option<Indexes>,
}

impl Args {
    pub(crate) fn execute(self) -> anyhow::Result<()> {
        // Squash has this helper:
        // pub fn make(signing_key: SigningKey, tag: Vec<u8>, squash_body: SquashBody) -> Self {
        // let exclude_list = parse_exclude_list(&self.exclude)?;
        let squash_body = konduit_data::SquashBody {
            amount: self.amount,
            index: self.index,
            exclude: match self.exclude {
                Some(exclude_list) => exclude_list,
                None => konduit_data::Indexes::new(vec![])?,
            },
        };
        let squash = konduit_data::Squash::make(&self.signing_key, &self.channel_tag, squash_body);

        let bytes = PlutusData::from(squash).to_cbor();
        let mut stdout = io::stdout();
        stdout.write_all(&bytes)?;
        stdout.flush()?;

        Ok(())
    }
}

use cardano_tx_builder::SigningKey;

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
    channel_tag: Option<konduit_data::Tag>,
}

impl Args {
    pub(crate) fn execute(self) -> anyhow::Result<()> {
        // TODO: finish this off
        Ok(())
    }
}

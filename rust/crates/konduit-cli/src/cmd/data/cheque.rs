use konduit_data::{cheque::Cheque, cheque_body::ChequeBody};

use crate::wallet::Wallet;

/// Create a (signed) cheque
#[derive(Debug, clap::Args)]
pub struct Cmd {
    /// Channel tag
    #[arg(long, value_parser=hex::decode,)] 
    tag: Vec<u8>,
    /// Cheque index
    #[arg(long)]
    index: u64,
    /// Cheque amount
    #[arg(long)]
    amount: u64,
    /// Cheque timeout. Posix time, milliseconds.
    #[arg(long)]
    timeout: u64,
    /// Cheque lock
    #[arg(long)]
    lock: String,
}

impl Cmd {
    fn run(self, w: Wallet) {
        let Cmd { tag, index, amount, timeout, lock } = self;
        let body = ChequeBody::new(index, amount, timeout, lock)
    }
}

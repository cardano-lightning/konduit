
use crate::{env::get_env, wallet::Wallet};

mod cheque;
mod squash;

#[derive(clap::Subcommand)]
/// Create data for konduit
/// Uses the wallet for signing
pub enum Cmd {
    /// Make a (signed) cheque
    #[command(subcommand)]
    Cheque(cheque::Cmd),
    /// Make a (signed) squash
    Squash(squash::Cmd),
}

impl Cmd {
    fn run(self) {
        let env = get_env();
        let w = Wallet::from_env(&env).unwrap();
        match self {
            Cmd::Cheque(cmd) => cmd.run(w),
            Cmd::Squash(cmd) => cmd.run(w), 
        }
    }
}

// fn handle_datum() {
//     let own_hash = Hash28([0; 28]);
//     let tag = Tag([1; 8].to_vec());
//     let add_vkey = VKey([2; 32]);
//     let sub_vkey = VKey([3; 32]);
//     let close_period = TimeDelta(86_400_000);
//     let constants = Constants {
//         tag,
//         add_vkey,
//         sub_vkey,
//         close_period,
//     };
//     let subbed = Amount(0x4444444444);
//     let stage = Stage::Opened(subbed);
//     let datum = Datum {
//         own_hash,
//         constants,
//         stage,
//     };
//     println!("{}", hex::encode(to_cbor(&datum).unwrap()))
// }

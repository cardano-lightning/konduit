use clap::Subcommand;
use tokio::runtime::Runtime;

use cardano_connect::CardanoConnect;
use cardano_tx_builder::{Address, NetworkId};
use crate::connector;

use crate::env::get_env;
use crate::wallet::{Wallet, generate_key};

#[derive(Subcommand)]
/// Wallet Api
pub enum Cmd {
    /// Gen new skey
    Gen,
    /// Show
    Show,
    /// Utxos at address. Requires `Cardano` connection.
    Utxos,
}

impl Cmd {
    pub fn run(self) {
        match self {
            Cmd::Gen => {
                println!("KONDUIT_WALLET_KEY={}", hex::encode(generate_key()));
            }
            Cmd::Show => {
                let env = get_env();
                let w = Wallet::from_env(&env).unwrap();
                let cred = w.credential();
                let addr_main = Address::new(NetworkId::MAINNET, cred.clone());
                let addr_test = Address::new(NetworkId::TESTNET, cred.clone());
                println!("VERIFICATION_KEY={}", hex::encode(w.verification_key()));
                // println!("PAYMENT_CRED={:?}", cred.into().to_bech32());
                println!("ADDRESS_MAINNET={:?}", addr_main);
                println!("ADDRESS_TESTNET={:?}", addr_test);
            }
            Cmd::Utxos => {
                let env = get_env();
                let w = Wallet::from_env(&env).unwrap();
                let cred = w.credential();
                let conn = connector::from_env(&env).unwrap();
                let rt = Runtime::new().expect("Failed to create Tokio runtime");
                rt.block_on(async {
                    println!("{:?}", conn.utxos_at(&cred, None).await);
                })
            }
        };
    }
}

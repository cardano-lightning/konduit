use cardano_connect::CardanoConnect;
use tokio::runtime::Runtime;

use crate::connector;
use crate::env::get_env;

/// Cardano api
#[derive(clap::Subcommand)]
pub enum Cmd {
    /// Health
    Health,
}

impl Cmd {
    pub(crate) fn run(self) {
        match self {
            Cmd::Health => {
                let env = get_env();
                let conn = connector::from_env(&env).unwrap();
                let rt = Runtime::new().expect("Failed to create Tokio runtime");
                rt.block_on(async {
                    println!("{:?}", conn.health().await);
                })
            }
        };
    }
}

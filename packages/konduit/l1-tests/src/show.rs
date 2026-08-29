use std::fmt::Display;

use cardano_connector::CardanoConnector;
use cardano_sdk::{Hash, Input};
use konduit_data::{Stage, VerifyingKey};
use konduit_tx::KONDUIT_VALIDATOR;
use serde::Serialize;

use crate::config::Config;

#[derive(Debug, Clone, Serialize)]
pub struct Script {
    hash: Hash<28>,
    reference: Option<Input>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Wallet {
    address: String,
    ada: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Adaptor {
    key: VerifyingKey,
}

#[derive(Debug, Clone, Serialize)]
pub struct Channel {
    key: VerifyingKey,
    stage: Stage,
    amount: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Show {
    script: Script,
    wallet: Wallet,
    // adaptor: Adaptor,
    // channels: Vec<Channel>
}

impl Display for Show {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&serde_json::to_string_pretty(self).unwrap())
    }
}

impl Show {
    pub async fn build(config: Config) -> anyhow::Result<Self> {
        let connector = config.cardano.build();
        let network_id = connector.network().into();
        let credential = config.wallet.credential();
        let utxos = connector
            .utxos_at(&config.wallet.credential(), None)
            .await?;
        let ada = utxos
            .values()
            .fold(0, |acc, cur| acc + cur.value().lovelace())
            / 1_000_000;

        let wallet = Wallet {
            address: config.wallet.address(network_id).to_string(),
            ada,
        };

        // // --- Adaptor ---
        // let adaptor = Adaptor {
        //     key: config.adaptor.key.verifying_key(),
        // };

        // --- Script ---
        // Presumably the adaptor's on-chain script hash, plus an optional
        // reference UTxO if one has been published. No source for either
        // in the given files, so these are placeholders.
        let script = Script {
            hash: KONDUIT_VALIDATOR.hash,
            reference: None, // TODO: look up reference script UTxO if applicable
        };

        // --- Channels ---
        // One Channel per configured account. Stage/amount presumably come
        // from querying on-chain state per account key — stubbed here.
        // let mut channels = Vec::with_capacity(config.accounts.len());
        // for acc in &config.accounts {
        //     let key = acc.key.verifying_key();
        //     let (stage, amount) = connector.channel_state(&key).await?; // TODO: real lookup
        //     channels.push(Channel { key, stage, amount });
        // }

        Ok(Show {
            script,
            wallet,
            // adaptor,
            // channels,
        })
    }
}

use crate::{cardano::ADA, config::hammer::Config};
use cardano_connect::CardanoConnect;
use cardano_tx_builder::{Credential, VerificationKey};
use konduit_data::{Duration, Tag};
use konduit_tx::{
    self, Bounds, ChannelOutput, KONDUIT_VALIDATOR, NetworkParameters,
    consumer::{Intent, OpenIntent},
};
use std::{collections::BTreeMap, str};
use tokio::runtime::Runtime;

use rand::{Rng, RngCore, distributions::Alphanumeric, thread_rng};
use serde::Serialize;
use std::ops::RangeInclusive;

/// Hammer txs. Random(ish) consumer actions.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// Probability X [0.0 - 1.0] for an account to open a NEW channel.
    #[arg(long, default_value_t = 0.3)]
    pub prob_open: f64,

    /// Length L of the alphanumeric Tag for new channels.
    #[arg(long, default_value_t = 8)]
    pub tag_length: usize,

    /// Initial open balance spread "lb..ub".
    #[arg(long, default_value = "10..20", value_parser = parse_range)]
    pub open_range: (u64, u64),

    /// Frequency weight for doing nothing on an existing channel.
    #[arg(long, default_value_t = 1)]
    pub do_nothing: u64,

    /// Frequency weight for adding to an existing channel.
    #[arg(long, default_value_t = 1)]
    pub do_add: u64,

    /// Frequency weight for closing an existing channel.
    #[arg(long, default_value_t = 1)]
    pub do_close: u64,

    /// Amount to add if 'add' is chosen for an existing channel.
    #[arg(long, default_value = "5..20", value_parser = parse_range)]
    pub add_range: (u64, u64),
}

#[derive(Serialize, Debug)]
pub struct MockEvent {
    pub account: String,
    pub tag: String,
    pub action: String,
    pub amount: u64,
}

// /// Simulation Engine
// pub async fn run_simulation(
//     accounts: Vec<String>,
//     existing_channels: Vec<ChannelState>,
//     args: Cmd
// ) -> anyhow::Result<()> {
//     let mut rng = thread_rng();
//     let mut results = Vec::new();
//
//     // 1. Calculate relative probabilities for maintenance using integer weights
//     let total_weight = args.do_nothing + args.do_add + args.do_close;
//
//     // 2. Process EXISTING channels
//     for channel in existing_channels {
//         if total_weight == 0 { break; } // Safety check
//
//         // Use a range up to the total sum of integer weights
//         let roll: u64 = rng.gen_range(0..total_weight);
//
//         if roll < args.do_nothing {
//             // "Do Nothing" selected
//             continue;
//         } else if roll < (args.do_nothing + args.do_add) {
//             // "Add" selected
//             results.push(MockEvent {
//                 account: channel.account.clone(),
//                 tag: channel.tag.clone(),
//                 action: "ADD".to_string(),
//                 amount: rng.gen_range(args.add_range.clone()),
//             });
//         } else {
//             // "Close" selected
//             results.push(MockEvent {
//                 account: channel.account.clone(),
//                 tag: channel.tag.clone(),
//                 action: "CLOSE".to_string(),
//                 amount: 0,
//             });
//         }
//     }
//
//     // 3. Process ACCOUNTS for new openings
//     for account in accounts {
//         if rng.gen_bool(args.prob_open) {
//             let tag: String = (&mut rng)
//                 .sample_iter(&Alphanumeric)
//                 .take(args.tag_length)
//                 .map(char::from)
//                 .collect();
//
//             results.push(MockEvent {
//                 account: account.clone(),
//                 tag,
//                 action: "OPEN".to_string(),
//                 amount: rng.gen_range(args.open_range.clone()),
//             });
//         }
//     }
//
//     // 4. Output as pretty-ish JSON
//     println!("{}", serde_json::to_string_pretty(&results)?);
//     Ok(())
// }

/// Example state fetched from your system
#[derive(Clone, Debug)]
pub struct ChannelState {
    pub account: String,
    pub tag: String,
}

fn parse_range(s: &str) -> Result<(u64, u64), String> {
    let parts: Vec<&str> = s.split("..").collect();
    if parts.len() != 2 {
        return Err("Format: lb..ub".into());
    }
    let lb = parts[0].parse().map_err(|e| format!("{}", e))?;
    let ub = parts[1].parse().map_err(|e| format!("{}", e))?;
    Ok((lb, ub))
}

impl Cmd {
    pub fn run(self, config: &Config) -> anyhow::Result<()> {
        let connector = config.connector.connector()?;
        let own_key = config.wallet().to_verification_key();
        let own_address = own_key.to_address(connector.network().into());
        let mut rng = rand::thread_rng();

        let create_tag = || {
            let l = rng.gen_range(1, self.tag_length);
            let mut vec = vec![0u8; l];
            rng.fill_bytes(&mut vec);
            Tag(vec)
        };

        let mut opens = Vec::new();
        for i in 0..config.accounts {
            if rng.gen_bool(self.prob_open) {
                let key = config.account(i);
                let sub_vkey = key.to_verification_key();
                let amount = rng.gen_range(self.open_range.0, self.open_range.1);
                // FIXME :: Don't hardcode close period.
                let item = OpenIntent {
                    tag: create_tag(),
                    sub_vkey,
                    close_period: Duration::from_secs(60 * 60 * 24),
                    amount,
                };
                opens.push(item);
            }
        }

        let bounds = Bounds::twenty_mins();
        let accounts = (0..config.accounts).map(|i| config.account(i).to_verification_key());
        Runtime::new()?.block_on(async {
            let protocol_parameters = connector.protocol_parameters().await?;
            let network_id = connector.network().into();
            let network_parameters = NetworkParameters {
                network_id,
                protocol_parameters,
            };

            let konduit_utxos = connector
                .utxos_at(&Credential::from_script(KONDUIT_VALIDATOR.hash), None)
                .await?;

            let opened = Vec::new();
            let closed = Vec::new();
            let mut intents = BTreeMap::new();

            for (input, output) in konduit_utxos.into_iter() {
                let Some(co) = ChannelOutput::from_output(&output) else {
                    continue;
                };
                let sub_vkey = co.constants.sub_vkey;
                let Some(index) = accounts.position(|x| x == sub_vkey) else {
                    continue;
                };
                match co.stage {
                    konduit_data::Stage::Opened(_, _) => {
                        intents.insert((input.clone(), Intent::Add(50000000)));
                        opened.push((input, closed));
                    }
                    _ => {
                        closed.push((input, output));
                    }
                }
            }

            let intents = self
                .add
                .iter()
                .map(|a| <(Tag, Intent)>::from(a.clone()))
                .chain(self.close.iter().map(|c| (c.clone(), Intent::Close)))
                .collect::<BTreeMap<_, _>>();

            let utxos = connector
                .utxos_at(&own_address.payment(), None)
                .await?
                .into_iter()
                .chain(
                    connector
                        .utxos_at(
                            &config.host_address.payment(),
                            config.host_address.delegation().as_ref(),
                        )
                        .await?,
                )
                .chain(channels.into_iter())
                .collect();
            let mut tx = konduit_tx::consumer::tx(
                &network_parameters,
                &own_key,
                opens,
                intents,
                &utxos,
                bounds,
            )?;
            println!("Tx id :: {}", tx.id());
            for vkey in tx.specified_signitories() {
                let index = vkeys.position(vkey).ok_or(anyhow::anyhow!("Missing vkey"));
                tx.sign(config.account(i));
            }
            tx.sign(&config.wallet());
            connector.submit(&tx).await
        })
    }
}

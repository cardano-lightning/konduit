use crate::{cardano::ADA, config::hammer::Config};
use cardano_connector_client::CardanoConnector;
use cardano_sdk::{Credential, Hash, SigningKey};
use konduit_data::{Constants, Duration, Tag};
use konduit_tx::{
    self, Bounds, ChannelOutput, KONDUIT_VALIDATOR, NetworkParameters,
    hammer::{self, Intent, OpenIntent},
};
use std::{collections::BTreeMap, str};
use tokio::runtime::Runtime;

use rand::{Rng, RngCore};

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
    #[arg(long, default_value = "3..20", value_parser = parse_range)]
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
    #[arg(long, default_value = "1..20", value_parser = parse_range)]
    pub add_range: (u64, u64),
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

        let create_tag = || {
            let mut rng = rand::thread_rng();
            let l = rng.gen_range(1, self.tag_length);
            let mut vec = vec![0u8; l];
            rng.fill_bytes(&mut vec);
            Tag::from(vec)
        };

        let mut rng = rand::thread_rng();
        let mut opens = Vec::new();
        for i in 0..config.accounts {
            if rng.gen_bool(self.prob_open) {
                let key = config.account(i);
                let sub_vkey = key.to_verification_key();
                let amount = rng.gen_range(self.open_range.0, self.open_range.1) * ADA;
                // FIXME :: Don't hardcode close period.
                let constants = Constants {
                    tag: create_tag(),
                    add_vkey: key.to_verification_key(),
                    sub_vkey,
                    close_period: Duration::from_secs(60 * 60 * 24),
                };
                let item = OpenIntent { constants, amount };
                opens.push(item);
            }
        }

        let do_add = self.do_add;
        let do_close = self.do_add + self.do_close;
        let total_weight = self.do_add + self.do_close + self.do_nothing;

        let bounds = Bounds::twenty_mins();
        let accounts = (0..config.accounts)
            .map(|i| config.account(i).to_verification_key())
            .collect::<Vec<_>>();
        let vkh_lookup = (0..config.accounts)
            .map(|i| config.account(i))
            .map(|k| (Hash::<28>::new(k.to_verification_key()), k))
            .collect::<BTreeMap<Hash<28>, SigningKey>>();
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

            let mut opened = Vec::new();
            let mut closed = Vec::new();
            let mut intents = BTreeMap::new();

            if total_weight > 0 {
                for (input, output) in konduit_utxos.into_iter() {
                    let Some(co) = ChannelOutput::from_output(&output) else {
                        continue;
                    };
                    let add_vkey = co.constants.add_vkey;
                    if !accounts.contains(&add_vkey) {
                        continue;
                    };
                    match co.stage {
                        konduit_data::Stage::Opened(_, _) => {
                            let roll: u64 = rng.gen_range(0, total_weight);
                            if roll < do_add {
                                // "Add" selected
                                let amount = rng.gen_range(self.add_range.0, self.add_range.1);
                                intents.insert(co.keytag(), Intent::Add(amount * ADA));
                                opened.push((input, output));
                            } else if roll < do_close {
                                // "Close" selected
                                intents.insert(co.keytag(), Intent::Close);
                                opened.push((input, output));
                            } else {
                                // "Do Nothing" selected
                                continue;
                            }
                        }
                        _ => {
                            closed.push((input, output));
                        }
                    }
                }
            }
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
                .chain(opened.into_iter())
                .chain(closed.into_iter())
                .collect();
            let mut tx = hammer::tx(
                &network_parameters,
                &own_key,
                &opens,
                &intents,
                &utxos,
                bounds,
            )?;
            let signers = tx.required_signatories(&utxos).expect("FIXME");
            println!("Tx id :: {}", tx.id());
            for vkh in signers {
                let key = vkh_lookup.get(&vkh).ok_or(anyhow::anyhow!("Missing vkh"))?;
                println!("{:?}", vkh);
                tx.sign(key);
            }
            for open in opens.iter() {
                println!("{:?}", open);
            }
            for intent in intents.iter() {
                println!("{:?}", intent);
            }
            println!("{:#}", tx);
            connector.submit(&tx).await
        })
    }
}

#[cfg(test)]
mod test {
    use cardano_sdk::{Input, hash};

    #[test]
    fn input_order() {
        let i0 = Input::new(
            hash!("702206530b2e1566e90b3aec753bd0abbf397842bd5421e0c3d23ed10167b3ce"),
            42,
        );
        let i1 = Input::new(
            hash!("702206530b2e1566e90b3aec753bd0abbf397842bd5421e0c3d23ed10167b3cf"),
            42,
        );
        let i2 = Input::new(
            hash!("702206530b2e1566e90b3aec753bd0abbf397842bd5421e0c3d23ed10167b3ce"),
            43,
        );
        assert!(i0 < i1, "i0 must be less than i1");
        assert!(i2 < i1, "i2 must be less than i1");
        assert!(i0 < i2, "i0 must be less than i2");
    }
}

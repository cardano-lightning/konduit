use std::{collections::BTreeMap, str};

use konduit_data::{Duration, Tag};
use tokio::runtime::Runtime;

use cardano_connect::CardanoConnect;
use cardano_tx_builder::VerificationKey;

use konduit_tx::{
    self, Bounds, NetworkParameters,
    consumer::{Intent, OpenIntent},
};

use crate::{cardano::ADA, config::consumer::Config};

/// Consumer tx. Can open, add, close, expire, elapse, and end.
/// Only open add and close need to be declared, the other steps are inferred from the context.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// Open konduit channel. CSV format.
    /// Note that a MIN_ADA_BUFFER will be added to the declared amount.
    #[arg(long, value_names = ["TAG,ADAPTOR_KEY,CLOSE_PERIOD,ADA"])]
    open: Vec<OpenArgs>,

    /// Add ada to channel
    #[arg(long, value_names = ["TAG,ADA"])]
    add: Vec<TagAmount>,

    /// Close channel
    #[arg(long, value_names = ["TAG"])]
    close: Vec<Tag>,
}

#[derive(Debug, Clone)]
pub struct OpenArgs {
    tag: Tag,
    sub_vkey: VerificationKey,
    close_period: Duration,
    amount: u64,
}

impl str::FromStr for OpenArgs {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let [a, b, c, d] = <[&str; 4]>::try_from(s.split(",").collect::<Vec<&str>>())
            .map_err(|_err| anyhow::anyhow!("Expected 4 args"))?;
        Ok(Self {
            tag: a.parse()?,
            sub_vkey: b.parse()?,
            close_period: c.parse()?,
            amount: d.parse::<u64>()? * ADA,
        })
    }
}

#[derive(Debug, Clone)]
struct TagAmount {
    pub tag: Tag,
    pub amount: u64,
}

impl str::FromStr for TagAmount {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let [a, b] = <[&str; 2]>::try_from(s.split(",").collect::<Vec<&str>>())
            .map_err(|_err| anyhow::anyhow!("Expected 2 args"))?;
        Ok(Self {
            tag: a.parse()?,
            amount: b.parse::<u64>()? * ADA,
        })
    }
}

impl From<TagAmount> for (Tag, Intent) {
    fn from(value: TagAmount) -> Self {
        (value.tag, Intent::Add(value.amount))
    }
}

impl From<OpenArgs> for OpenIntent {
    fn from(args: OpenArgs) -> Self {
        Self {
            tag: args.tag,
            sub_vkey: args.sub_vkey,
            close_period: args.close_period,
            amount: args.amount,
        }
    }
}

impl Cmd {
    pub fn run(self, config: &Config) -> anyhow::Result<()> {
        let connector = config.connector.connector()?;
        let own_key = config.wallet.to_verification_key();
        let own_address = config.wallet.to_address(&connector.network().into());
        let opens = self
            .open
            .into_iter()
            .map(|x| OpenIntent::from(x))
            .collect::<Vec<_>>();
        let intents = self
            .add
            .iter()
            .map(|a| <(Tag, Intent)>::from(a.clone()))
            .chain(self.close.iter().map(|c| (c.clone(), Intent::Close)))
            .collect::<BTreeMap<_, _>>();
        let bounds = Bounds::twenty_mins();

        Runtime::new()?.block_on(async {
            let protocol_parameters = connector.protocol_parameters().await?;
            let network_id = connector.network().into();
            let network_parameters = NetworkParameters {
                network_id,
                protocol_parameters,
            };
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
                        .await?
                        .into_iter(),
                )
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
            tx.sign(config.wallet.clone().into());
            connector.submit(&tx).await
        })
    }
}

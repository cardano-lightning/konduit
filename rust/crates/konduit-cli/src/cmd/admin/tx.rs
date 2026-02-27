use crate::{cardano::ADA, config::admin::Config};
use cardano_connector::CardanoConnector;
use cardano_sdk::{Address, Value, address::kind};
use konduit_tx::{self, KONDUIT_VALIDATOR};
use std::str;
use tokio::runtime::Runtime;

/// Create and submit Konduit transactions
#[derive(Debug, Clone, clap::Subcommand)]
pub enum Cmd {
    /// Send ada to addresses. Note that this will spend any reference script
    Send(SendArgs),

    /// "Deploy" aka upload tx
    Deploy(DeployArgs),
}

#[derive(Debug, Clone, clap::Args)]
pub struct SendArgs {
    /// If set, will spend utxos with scripts. Unset by default.
    #[arg(long, default_value_t = false)]
    spend_all: bool,

    /// The amounts are in ada (not lovelace!)
    #[arg(long, value_names = ["ADDRESS,ADA"])]
    to: Vec<AddressAmount>,

    /// Where the rest of the value goes. Defaults to own address
    #[arg(long)]
    rest: Option<Address<kind::Shelley>>,
}

#[derive(Debug, Clone)]
struct AddressAmount(pub Address<kind::Shelley>, pub u64);

impl str::FromStr for AddressAmount {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let [a, b] = <[&str; 2]>::try_from(s.split(",").collect::<Vec<&str>>())
            .map_err(|_err| anyhow::anyhow!("Expected 2 args"))?;
        Ok(Self(a.parse()?, b.parse()?))
    }
}

impl From<AddressAmount> for (Address<kind::Any>, Value<u64>) {
    fn from(value: AddressAmount) -> Self {
        (value.0.into(), Value::new(value.1 * ADA))
    }
}

#[derive(Debug, Clone, clap::Args)]
pub struct DeployArgs {
    /// If true, will spend utxos with scripts. Defaults to false
    #[arg(long, default_value_t = false)]
    spend_all: bool,
}

impl Cmd {
    pub fn run(self, config: &Config) -> anyhow::Result<()> {
        let connector = config.connector.connector()?;
        let own_address = config
            .wallet
            .to_verification_key()
            .to_address(connector.network().into());
        match self {
            Cmd::Send(args) => {
                let tos = args.to.iter().map(|a| a.clone().into()).collect::<Vec<_>>();
                let change_address = args.rest.unwrap_or(own_address.clone()).into();
                Runtime::new()?.block_on(async {
                    let utxos = connector
                        .utxos_at(&own_address.payment(), None)
                        .await?
                        .into_iter()
                        .filter(|(_, o)| o.script().is_none() || args.spend_all)
                        .collect();
                    let protocol_parameters = &connector.protocol_parameters().await?;
                    let mut tx =
                        konduit_tx::admin::send(protocol_parameters, &utxos, tos, change_address)?;
                    println!("Tx id :: {}", tx.id());
                    tx.sign(&config.wallet);
                    connector.submit(&tx).await
                })
            }

            Cmd::Deploy(args) => {
                let host_address = Address::<kind::Any>::from(config.host_address.clone());
                let own_address = config
                    .wallet
                    .to_verification_key()
                    .to_address(connector.network().into());
                let change_address = Address::<kind::Any>::from(own_address.clone());
                Runtime::new()?.block_on(async {
                    let utxos = connector
                        .utxos_at(&own_address.payment(), None)
                        .await?
                        .into_iter()
                        .filter(|(_, o)| o.script().is_some() || !args.spend_all)
                        .collect();
                    let protocol_parameters = connector.protocol_parameters().await?;
                    let mut tx = konduit_tx::admin::deploy(
                        &protocol_parameters,
                        &utxos,
                        KONDUIT_VALIDATOR.script.clone(),
                        host_address,
                        change_address,
                    )?;
                    println!("Tx id :: {}", tx.id());
                    tx.sign(&config.wallet);
                    connector.submit(&tx).await
                })
            }
        }
    }
}

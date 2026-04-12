use crate::{cardano::ADA, config::admin::Config};
use cardano_connector::CardanoConnector;
use cardano_sdk::{Address, SigningKey, Value, address::kind};
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
        match self {
            Cmd::Send(args) => Runtime::new()?.block_on(run_send(&connector, &config.wallet, args)),

            Cmd::Deploy(args) => Runtime::new()?.block_on(run_deploy(
                &connector,
                &config.wallet,
                &config.host_address,
                args,
            )),
        }
    }
}

async fn run_send(
    connector: &impl CardanoConnector,
    wallet: &SigningKey,
    args: SendArgs,
) -> anyhow::Result<()> {
    let own_address = wallet
        .to_verification_key()
        .to_address(connector.network().into());
    let tos = args.to.iter().map(|a| a.clone().into()).collect::<Vec<_>>();
    let change_address = args.rest.unwrap_or(own_address.clone()).into();
    let utxos = connector
        .utxos_at(&own_address.payment(), None)
        .await?
        .into_iter()
        .filter(|(_, o)| o.script().is_none() || args.spend_all)
        .collect();
    let protocol_parameters = connector.protocol_parameters().await?;
    let mut tx = konduit_tx::admin::send(&protocol_parameters, &utxos, tos, change_address)?;
    println!("Tx id :: {}", tx.id());
    tx.sign(wallet);
    connector.submit(&tx).await
}

async fn run_deploy(
    connector: &impl CardanoConnector,
    wallet: &SigningKey,
    host_address: &Address<kind::Shelley>,
    args: DeployArgs,
) -> anyhow::Result<()> {
    let own_address = wallet
        .to_verification_key()
        .to_address(connector.network().into());
    let change_address = Address::<kind::Any>::from(own_address.clone());
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
        Address::<kind::Any>::from(host_address.clone()),
        change_address,
    )?;
    println!("Tx id :: {}", tx.id());
    tx.sign(wallet);
    connector.submit(&tx).await
}

#[cfg(test)]
mod tests {
    use super::{ADA, SendArgs, run_send};
    use cardano_connector::CardanoConnector;
    use cardano_sdk::{
        Address, Credential, Hash, Input, Network, Output, ProtocolParameters, SigningKey,
        Transaction, Value, address::kind, transaction::state,
    };
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct ConnectorState {
        protocol_parameter_calls: usize,
        submit_calls: usize,
    }

    struct FakeConnector {
        network: Network,
        expected_payment: Credential,
        utxos: BTreeMap<Input, Output>,
        state: Arc<Mutex<ConnectorState>>,
    }

    impl FakeConnector {
        fn new(expected_payment: Credential, utxos: BTreeMap<Input, Output>) -> Self {
            Self {
                network: Network::Preview,
                expected_payment,
                utxos,
                state: Arc::new(Mutex::new(ConnectorState::default())),
            }
        }
    }

    impl CardanoConnector for FakeConnector {
        fn network(&self) -> Network {
            self.network
        }

        async fn health(&self) -> anyhow::Result<String> {
            Ok("ok".to_string())
        }

        async fn protocol_parameters(&self) -> anyhow::Result<ProtocolParameters> {
            self.state
                .lock()
                .expect("state lock")
                .protocol_parameter_calls += 1;
            Ok(ProtocolParameters::preview())
        }

        async fn utxos_at(
            &self,
            payment: &Credential,
            delegation: Option<&Credential>,
        ) -> anyhow::Result<BTreeMap<Input, Output>> {
            assert_eq!(payment, &self.expected_payment);
            assert!(delegation.is_none());
            Ok(self.utxos.clone())
        }

        async fn submit(
            &self,
            _transaction: &Transaction<state::ReadyForSigning>,
        ) -> anyhow::Result<()> {
            self.state.lock().expect("state lock").submit_calls += 1;
            Ok(())
        }
    }

    fn payment_address(wallet: &SigningKey) -> Address<kind::Shelley> {
        wallet
            .to_verification_key()
            .to_address(Network::Preview.into())
    }

    fn wallet_utxos(address: &Address<kind::Shelley>) -> BTreeMap<Input, Output> {
        BTreeMap::from([(
            Input::new(Hash::<32>::from([9; 32]), 0),
            Output::new(address.clone().into(), Value::new(10 * ADA)),
        )])
    }

    #[tokio::test]
    async fn send_smoke_path_builds_and_submits_transaction() {
        let wallet = SigningKey::from([5; 32]);
        let own_address = payment_address(&wallet);
        let connector = FakeConnector::new(own_address.payment(), wallet_utxos(&own_address));
        let destination = wallet
            .to_verification_key()
            .to_address(Network::Preview.into());
        let args = SendArgs {
            spend_all: false,
            to: vec![super::AddressAmount(destination, 2)],
            rest: None,
        };

        run_send(&connector, &wallet, args)
            .await
            .expect("send smoke path should succeed");

        let state = connector.state.lock().expect("state lock");
        assert_eq!(state.protocol_parameter_calls, 1);
        assert_eq!(state.submit_calls, 1);
    }
}

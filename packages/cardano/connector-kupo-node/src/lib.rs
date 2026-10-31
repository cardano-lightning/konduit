use anyhow::{Context, anyhow};
use cardano_connector::CardanoConnector;
use cardano_connector_node::{Config as NodeConfig, Node};
use cardano_sdk::{
    Address, Credential, Hash, Input, Network, Output, ProtocolParameters, Transaction, Value,
    cbor, transaction::state,
};
use kupo_client::{
    Client as KupoClient, DatumType, Match, MatchFilters, MatchStatus, Pattern, ScriptLanguage,
};
use std::collections::BTreeMap;
use std::future::Future;

/// Reads UTxOs from Kupo, falling back to a local node query, and delegates
/// protocol parameters and transaction submission to the node.
pub struct KupoNode {
    kupo: KupoClient,
    node: Node,
}

impl KupoNode {
    pub async fn connect(kupo_url: &str, node_config: NodeConfig) -> anyhow::Result<Self> {
        let kupo = KupoClient::new(kupo_url)
            .map_err(|error| anyhow!(error))
            .with_context(|| format!("invalid Kupo URL {kupo_url}"))?;
        let node = Node::connect(node_config).await?;
        Ok(Self { kupo, node })
    }

    async fn query_kupo(
        &self,
        payment: &Credential,
        delegation: Option<&Credential>,
    ) -> anyhow::Result<BTreeMap<Input, Output>> {
        let pattern = match delegation {
            Some(delegation) => Pattern::address(
                Address::new(self.node.network().into(), payment.clone())
                    .with_delegation(delegation.clone())
                    .to_string(),
            ),
            None => Pattern::address(
                payment.select(|hash| format!("{hash}/*"), |hash| format!("{hash}/*")),
            ),
        };
        let matches = self
            .kupo
            .matches(
                &pattern,
                &MatchFilters::new()
                    .with_status(MatchStatus::Unspent)
                    .with_resolve_hashes(true),
            )
            .await
            .map_err(|error| anyhow!(error))
            .context("Kupo UTxO query failed")?;

        let mut utxos = BTreeMap::new();
        for matched in matches.body {
            let (input, output) = map_match(matched)?;
            utxos.insert(input, output);
        }
        Ok(utxos)
    }
}

impl CardanoConnector for KupoNode {
    fn network(&self) -> Network {
        self.node.network()
    }

    async fn health(&self) -> anyhow::Result<String> {
        let response = self.kupo.health().await.map_err(|error| anyhow!(error))?;
        if response.body.connection_status != kupo_client::ConnectionStatus::Connected {
            anyhow::bail!("Kupo is disconnected from the Cardano node");
        }
        Ok(format!(
            "Kupo connected; node tip={:?}",
            response.body.most_recent_node_tip
        ))
    }

    async fn protocol_parameters(&self) -> anyhow::Result<ProtocolParameters> {
        self.node.protocol_parameters().await
    }

    async fn utxos_at(
        &self,
        payment: &Credential,
        delegation: Option<&Credential>,
    ) -> anyhow::Result<BTreeMap<Input, Output>> {
        with_node_fallback(self.query_kupo(payment, delegation).await, || {
            self.node.utxos_at(payment, delegation)
        })
        .await
    }

    async fn submit(
        &self,
        transaction: &Transaction<state::ReadyForSigning>,
    ) -> anyhow::Result<()> {
        self.node.submit(transaction).await
    }
}

async fn with_node_fallback<T, F, Fut>(query: anyhow::Result<T>, fallback: F) -> anyhow::Result<T>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = anyhow::Result<T>>,
{
    match query {
        Ok(value) => Ok(value),
        Err(error) => {
            eprintln!("Kupo UTxO query failed; falling back to local node: {error:#}");
            fallback().await
        }
    }
}

fn map_match(matched: Match) -> anyhow::Result<(Input, Output)> {
    let tx_hash: [u8; 32] = matched.transaction_id.0;
    let input = Input::new(Hash::from(tx_hash), u64::from(matched.output_index));
    let address = Address::try_from(matched.address.as_str())
        .map_err(|error| anyhow!("invalid address in Kupo match: {error}"))?;
    let assets = matched
        .value
        .assets
        .into_iter()
        .map(|(id, quantity)| {
            let policy: [u8; 28] = id.policy_id.0;
            (
                Hash::from(policy),
                [(
                    id.asset_name
                        .map_or_else(Vec::new, |name| name.into_bytes()),
                    quantity,
                )],
            )
        })
        .collect::<Vec<_>>();
    let mut output = Output::new(address, Value::new(matched.value.coins).with_assets(assets));
    match (matched.datum_type, matched.datum, matched.datum_hash) {
        (Some(DatumType::Inline), Some(datum), _) => {
            let datum = cbor::decode(&datum).context("failed to decode Kupo inline datum")?;
            output = output.with_datum(datum);
        }
        (_, _, Some(hash)) => output = output.with_datum_hash(Hash::from(hash.0)),
        _ => {}
    }
    if let Some(script) = matched.script {
        let version = match script.language {
            ScriptLanguage::PlutusV1 => cardano_sdk::PlutusVersion::V1,
            ScriptLanguage::PlutusV2 => cardano_sdk::PlutusVersion::V2,
            ScriptLanguage::PlutusV3 => cardano_sdk::PlutusVersion::V3,
            ScriptLanguage::Native => anyhow::bail!("native reference scripts are unsupported"),
        };
        output = output.with_plutus_script(cardano_sdk::PlutusScript::new(version, script.script));
    }
    Ok((input, output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_unspent_kupo_match_to_sdk_utxo() {
        let payment = Credential::from_key(Hash::from([2; 28]));
        let delegation = Credential::from_key(Hash::from([3; 28]));
        let address =
            Address::new(cardano_sdk::NetworkId::TESTNET, payment).with_delegation(delegation);
        let matched = Match {
            address: address.to_string(),
            created_at: kupo_client::Checkpoint {
                block_no: 1,
                slot_no: 2,
                header_hash: [3; 32].into(),
            },
            datum: None,
            datum_hash: None,
            datum_type: None,
            output_index: 4,
            script_hash: None,
            script: None,
            spent_at: None,
            transaction_index: 0,
            transaction_id: [5; 32].into(),
            value: kupo_client::Value {
                coins: 7,
                assets: BTreeMap::new(),
            },
        };

        let (input, output) = map_match(matched).expect("match maps");
        assert_eq!(input.output_index(), 4);
        assert_eq!(output.value().lovelace(), 7);
    }

    #[tokio::test]
    async fn successful_empty_kupo_result_does_not_fall_back() {
        let result = with_node_fallback(Ok(Vec::<u8>::new()), || async {
            panic!("fallback must not be called")
        })
        .await
        .expect("empty Kupo result succeeds");
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn failed_kupo_query_uses_node_fallback() {
        let result = with_node_fallback::<_, _, _>(Err(anyhow!("Kupo unavailable")), || async {
            Ok(vec![42])
        })
        .await
        .expect("node fallback succeeds");
        assert_eq!(result, vec![42]);
    }
}

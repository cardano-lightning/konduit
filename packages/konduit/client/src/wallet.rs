use crate::{
    Signer,
    core::{Credential, Input, NetworkId, Output, Value},
    utxo_batch,
};
use cardano_connector::CardanoConnector;
use cardano_sdk::{
    Address, Hash, Signature, Transaction, VerificationKey, address::kind, transaction::state,
};
use std::{collections::BTreeMap, future::Future, sync::Arc};

/// Based on CIP-30. At least permits a CIP-30 wallet to be wrapped and impl this trait.
pub trait Wallet {
    type Error: std::error::Error + Send + Sync + 'static;

    fn network_id(&self) -> impl Future<Output = Result<NetworkId, Self::Error>>;

    /// `api.getChangeAddress()`. The wallet's preferred address for receiving change
    fn change_address(&self) -> impl Future<Output = Result<Address<kind::Any>, Self::Error>>;

    /// `api.getUtxos(value, paginate)`. Pagination is not honored.
    /// `value`, when given, IS honored and returns `None` if the wallet cannot satisfy value
    fn utxos(
        &self,
        value: Option<Value<u64>>,
    ) -> impl Future<Output = Result<Option<BTreeMap<Input, Output>>, Self::Error>>;

    /// `api.signTx(tx, partialSign: true)`. Always requests a partial
    /// signature
    fn sign_tx(
        &self,
        tx: &Transaction<state::ReadyForSigning>,
    ) -> impl Future<Output = Result<(VerificationKey, Signature), Self::Error>>;

    /// Analogue to `api.submitTx(tx: cbor<transaction>): Promise<hash32>`.
    fn submit(
        &self,
        tx: &Transaction<state::ReadyForSigning>,
    ) -> impl Future<Output = Result<Hash<32>, Self::Error>>;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("signing error: {0}")]
    Signing(String),
    #[error("connector error: {0}")]
    Connector(String),
}

impl From<utxo_batch::Error> for Error {
    fn from(e: utxo_batch::Error) -> Self {
        Error::Connector(e.to_string())
    }
}

/// `Wallet` backed by an embedded `Signer`, plus a real `Connector` and
/// (optional) stake credential.
pub struct Embedded<Connector: CardanoConnector, S: Signer> {
    connector: Arc<Connector>,
    signer: Arc<S>,
    delegation: Option<Credential>,
}

impl<Connector: CardanoConnector, S: Signer> Embedded<Connector, S> {
    pub fn new(connector: Arc<Connector>, signer: Arc<S>, delegation: Option<Credential>) -> Self {
        Self {
            connector,
            signer,
            delegation,
        }
    }
}

impl<Connector: CardanoConnector, S: Signer> Wallet for Embedded<Connector, S> {
    type Error = Error;

    async fn network_id(&self) -> Result<NetworkId, Self::Error> {
        Ok(NetworkId::from(self.connector.network()))
    }

    async fn change_address(&self) -> Result<Address<kind::Any>, Self::Error> {
        let network_id = self.network_id().await?;
        let payment_credential = Credential::from(&self.signer.verification_key());
        let address = Address::new(network_id, payment_credential);
        match &self.delegation {
            Some(delegation) => Ok(address.with_delegation(delegation.clone()).into()),
            None => Ok(address.into()),
        }
    }

    async fn utxos(
        &self,
        value: Option<Value<u64>>,
    ) -> Result<Option<BTreeMap<Input, Output>>, Self::Error> {
        let vk = self.signer.verification_key();
        let payment_credential = Credential::from(&vk);

        let mut pairs = vec![(payment_credential.clone(), None)];
        if let Some(stake) = &self.delegation {
            pairs.push((payment_credential, Some(stake.clone())));
        }

        let all_utxos = utxo_batch::utxo_batch(&*self.connector, &pairs).await?;

        let Some(value) = value else {
            return Ok(if all_utxos.is_empty() {
                None
            } else {
                Some(all_utxos)
            });
        };

        let utxos: Vec<(Input, Output)> = all_utxos.into_iter().collect();

        let Some(selection) = Value::cover(&value, &utxos, |(_, output)| output.value()) else {
            return Ok(None);
        };

        let selected = selection
            .inputs
            .into_iter()
            .cloned()
            .collect::<BTreeMap<Input, Output>>();
        Ok(Some(selected))
    }

    async fn sign_tx(
        &self,
        tx: &Transaction<state::ReadyForSigning>,
    ) -> Result<(VerificationKey, Signature), Self::Error> {
        let tbs = tx.id();
        let signature = self
            .signer
            .sign(tbs.as_ref())
            .await
            .map_err(|e| Error::Signing(e.to_string()))?;
        Ok((self.signer.verification_key(), signature))
    }

    async fn submit(
        &self,
        tx: &Transaction<state::ReadyForSigning>,
    ) -> Result<Hash<32>, Self::Error> {
        self.connector
            .submit(tx)
            .await
            .map_err(|e| Error::Connector(e.to_string()))?;
        Ok(tx.id())
    }
}

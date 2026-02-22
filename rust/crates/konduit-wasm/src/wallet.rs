use crate::{CardanoConnector, Channel, Marshall, TransactionSummary, marshall::Unmarshall};
use cardano_connect::NetworkName;
use cardano_connect_wasm as wasm;
use cardano_tx_builder::{
    Credential, NetworkId, Signature, SigningKey, VerificationKey, address::ShelleyAddress,
};
use std::ops::Deref;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Clone)]
pub struct Wallet {
    signing_key: SigningKey,
    stake_credential: Option<Credential>,
    exit_address: Option<ShelleyAddress>,
    network: NetworkName,
}

impl Wallet {
    pub(crate) fn new(signing_key: SigningKey, network: NetworkName) -> wasm::Result<Self> {
        Ok(Self {
            signing_key,
            stake_credential: None,
            exit_address: None,
            network,
        })
    }

    pub(crate) fn sign(&self, msg: impl AsRef<[u8]>) -> (VerificationKey, Signature) {
        (
            self.signing_key.to_verification_key(),
            self.signing_key.sign(msg),
        )
    }
}

#[wasm_bindgen]
impl Wallet {
    // ------------------------------------------------------------------------ Initialize

    #[wasm_bindgen(js_name = "create")]
    pub fn create(network: NetworkName) -> wasm::Result<Self> {
        Self::restore(SigningKey::new(), network)
    }

    #[wasm_bindgen(js_name = "restore")]
    pub fn restore(signing_key: SigningKey, network: NetworkName) -> wasm::Result<Self> {
        Self::new(signing_key, network)
    }

    // ------------------------------------------------------------------------ Inspecting

    #[wasm_bindgen(getter, js_name = "verificationKey")]
    pub fn verification_key(&self) -> VerificationKey {
        self.signing_key.to_verification_key()
    }

    #[wasm_bindgen(getter, js_name = "paymentCredential")]
    pub fn payment_credential(&self) -> Credential {
        self.verification_key().to_credential()
    }

    #[wasm_bindgen(getter)]
    pub fn address(&self) -> ShelleyAddress {
        let mut address = self
            .signing_key
            .to_verification_key()
            .to_address(NetworkId::from(*self.network.deref()));

        if let Some(stake_credential) = self.stake_credential.as_ref() {
            address = address.with_delegation(stake_credential.clone());
        }

        address.into()
    }

    #[wasm_bindgen(getter, js_name = "stakeCredential")]
    pub fn stake_credential(&self) -> Option<Credential> {
        self.stake_credential.clone()
    }

    #[wasm_bindgen(setter, js_name = "stakeCredential")]
    pub fn set_stake_credential(&mut self, stake_credential: Option<Credential>) {
        self.stake_credential = stake_credential;
    }

    #[wasm_bindgen(getter, js_name = "exitAddress")]
    pub fn exit_address(&self) -> Option<ShelleyAddress> {
        self.exit_address.clone()
    }

    #[wasm_bindgen(setter, js_name = "exitAddress")]
    pub fn set_exit_address(&mut self, exit_address: Option<ShelleyAddress>) {
        self.exit_address = exit_address;
    }

    #[wasm_bindgen(getter, js_name = "networkName")]
    pub fn network_name(&self) -> NetworkName {
        self.network
    }

    #[wasm_bindgen(getter, js_name = "networkId")]
    pub fn network_id(&self) -> NetworkId {
        self.network._wasm_to_network_id()
    }

    // ------------------------------------------------------------------------ Querying

    #[wasm_bindgen(js_name = "balance")]
    pub async fn balance(
        &self,
        connector: &CardanoConnector,
        konduit_validator: &Credential,
    ) -> wasm::Result<u64> {
        let l1_balance = connector
            .balance(self.signing_key.to_verification_key().as_ref())
            .await?;

        let l2_balance = Channel::opened(connector, self, konduit_validator)
            .await?
            .iter()
            .fold(0, |total, channel| total + channel.amount());

        Ok(l1_balance + l2_balance)
    }

    #[wasm_bindgen(js_name = "transactions")]
    pub async fn transactions(
        &self,
        connector: &CardanoConnector,
    ) -> wasm::Result<Vec<TransactionSummary>> {
        connector.transactions(&self.payment_credential()).await
    }

    // ------------------------------------------------------------------------ Marshalling

    #[wasm_bindgen(js_name = "serialize")]
    pub fn serialize(&self) -> String {
        // We leak a clone to preserve the original key while using existing core type APIs.
        let signing_key = unsafe { SigningKey::leak(self.signing_key.clone()) };

        (
            self.network,
            signing_key,
            &self.stake_credential,
            &self.exit_address,
        )
            .marshall()
    }

    #[wasm_bindgen(js_name = "deserialize")]
    pub fn deserialize(serialized: &str) -> wasm::Result<Wallet> {
        let (network, signing_key, stake_credential, exit_address) =
            Unmarshall::unmarshall(serialized)?;

        let signing_key = <[u8; 32]>::into(signing_key);

        Ok(Self {
            network,
            signing_key,
            stake_credential,
            exit_address,
        })
    }
}

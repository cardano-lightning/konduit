use crate::{
    Channel, Connector, Marshall,
    core::{Credential, NetworkId, SigningKey, wasm},
    marshall::Unmarshall,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Clone)]
pub struct Wallet {
    signing_key: SigningKey,
    stake_credential: Option<Credential>,
    exit_address: Option<wasm::ShelleyAddress>,
    network_id: NetworkId,
}

impl Wallet {
    pub(crate) fn new(signing_key: SigningKey, network_id: NetworkId) -> crate::Result<Self> {
        Ok(Self {
            signing_key,
            stake_credential: None,
            exit_address: None,
            network_id,
        })
    }
}

#[wasm_bindgen]
impl Wallet {
    // ------------------------------------------------------------------------ Initialize

    #[wasm_bindgen(js_name = "create")]
    pub fn create(network_id: wasm::NetworkId) -> crate::Result<Self> {
        Self::new(SigningKey::new(), network_id.into())
    }

    #[wasm_bindgen(js_name = "restore")]
    pub fn restore(
        signing_key: wasm::SigningKey,
        network_id: wasm::NetworkId,
    ) -> crate::Result<Self> {
        Self::new(signing_key.into(), network_id.into())
    }

    // ------------------------------------------------------------------------ Inspecting

    #[wasm_bindgen(getter, js_name = "signingKey")]
    pub fn signing_key(&self) -> wasm::SigningKey {
        self.signing_key.clone().into()
    }

    #[wasm_bindgen(getter, js_name = "verificationKey")]
    pub fn verification_key(&self) -> wasm::VerificationKey {
        self.signing_key().to_verification_key().into()
    }

    #[wasm_bindgen(getter, js_name = "paymentCredential")]
    pub fn payment_credential(&self) -> wasm::Credential {
        self.verification_key().to_credential().into()
    }

    #[wasm_bindgen(getter)]
    pub fn address(&self) -> wasm::ShelleyAddress {
        let mut address = self.verification_key().to_address(self.network_id);

        if let Some(stake_credential) = self.stake_credential.as_ref() {
            address = address.with_delegation(stake_credential.clone());
        }

        address.into()
    }

    #[wasm_bindgen(getter, js_name = "stakeCredential")]
    pub fn stake_credential(&self) -> Option<wasm::Credential> {
        self.stake_credential.clone().map(Into::into)
    }

    #[wasm_bindgen(setter, js_name = "stakeCredential")]
    pub fn set_stake_credential(&mut self, stake_credential: Option<wasm::Credential>) {
        self.stake_credential = stake_credential.map(Into::into);
    }

    #[wasm_bindgen(getter, js_name = "exitAddress")]
    pub fn exit_address(&self) -> Option<wasm::ShelleyAddress> {
        self.exit_address.clone()
    }

    #[wasm_bindgen(setter, js_name = "exitAddress")]
    pub fn set_exit_address(&mut self, exit_address: Option<wasm::ShelleyAddress>) {
        self.exit_address = exit_address;
    }

    #[wasm_bindgen(getter, js_name = "networkId")]
    pub fn network_id(&self) -> wasm::NetworkId {
        self.network_id.into()
    }

    // ------------------------------------------------------------------------ Querying

    #[wasm_bindgen(js_name = "balance")]
    pub async fn balance(&self, connector: &Connector) -> crate::Result<u64> {
        let l1_balance = connector._wasm_balance(&self.verification_key()).await?;

        let l2_balance = Channel::opened(connector, self)
            .await?
            .iter()
            .fold(0, |total, channel| total + channel.amount());

        Ok(l1_balance + l2_balance)
    }

    #[wasm_bindgen(js_name = "transactions")]
    pub async fn transactions(
        &self,
        connector: &Connector,
    ) -> crate::Result<Vec<wasm::TransactionSummary>> {
        connector
            ._wasm_transactions(&self.payment_credential())
            .await
    }

    // ------------------------------------------------------------------------ Marshalling

    #[wasm_bindgen(js_name = "serialize")]
    pub fn serialize(&self) -> String {
        // We leak a clone to preserve the original key while using existing core type APIs.
        let signing_key = unsafe { SigningKey::leak(self.signing_key.clone()) };

        (
            self.network_id,
            signing_key,
            &self.stake_credential,
            &self.exit_address,
        )
            .marshall()
    }

    #[wasm_bindgen(js_name = "deserialize")]
    pub fn deserialize(serialized: &str) -> crate::Result<Wallet> {
        let decoded: (
            NetworkId,
            [u8; 32],
            Option<Credential>,
            Option<wasm::ShelleyAddress>,
        ) = Unmarshall::unmarshall(serialized)?;
        let (network_id, signing_key_bytes, stake_credential, exit_address) = decoded;

        let signing_key = SigningKey::from(signing_key_bytes);

        Ok(Self {
            network_id,
            signing_key,
            stake_credential,
            exit_address,
        })
    }
}

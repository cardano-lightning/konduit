use crate::wasm::{self, Credential, NetworkId, ShelleyAddress, SigningKey, VerificationKey};
use anyhow::anyhow;
use wasm_bindgen::prelude::*;

/// A rudimentary wallet interface
#[wasm_bindgen]
pub struct Wallet {
    network_id: NetworkId,
    signing_key: SigningKey,
    stake_credential: Option<Credential>,
    exit_address: wasm::Result<ShelleyAddress>,
}

impl Wallet {
    pub fn new(network_id: NetworkId, signing_key: SigningKey) -> Self {
        Self {
            network_id,
            signing_key,
            stake_credential: None,
            exit_address: Err(anyhow!("no exit address").into()),
        }
    }

    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }
}

// Wallet-related methods for the Konduit interface.
#[wasm_bindgen]
impl Wallet {
    #[wasm_bindgen(getter, js_name = "verificationKey")]
    pub fn verification_key(&self) -> VerificationKey {
        self.signing_key.to_verification_key().into()
    }

    #[wasm_bindgen(getter, js_name = "paymentCredential")]
    pub fn payment_credential(&self) -> Credential {
        self.verification_key().to_credential().into()
    }

    #[wasm_bindgen(getter, js_name = "stakeCredential")]
    pub fn stake_credential(&self) -> Option<Credential> {
        self.stake_credential.clone()
    }

    #[wasm_bindgen(setter, js_name = "stakeCredential")]
    pub fn set_stake_credential(&mut self, stake_credential: &Credential) {
        self.stake_credential = Some(stake_credential.clone())
    }

    #[wasm_bindgen(js_name = "resetStakeCredential")]
    pub fn reset_stake_credential(&mut self) {
        self.stake_credential = None;
    }

    #[wasm_bindgen(getter, js_name = "address")]
    pub fn address(&self) -> ShelleyAddress {
        let mut address = self.verification_key().to_address(self.network_id.into());

        if let Some(stake_credential) = self.stake_credential.as_ref() {
            address = address.with_delegation(stake_credential.clone().into());
        }

        address.into()
    }

    #[wasm_bindgen(getter, js_name = "exitAddress")]
    pub fn exit_address(&self) -> Option<ShelleyAddress> {
        match self.exit_address {
            Ok(ref exit_address) => Some(exit_address.clone()),
            Err(_) => None,
        }
    }

    #[wasm_bindgen(setter, js_name = "exitAddress")]
    pub fn set_exit_address(&mut self, exit_address: &ShelleyAddress) {
        self.exit_address = Ok(exit_address.clone());
    }

    #[wasm_bindgen(js_name = "resetExitAddress")]
    pub fn reset_exit_address(&mut self) {
        self.exit_address = Err(anyhow!("no exit address").into());
    }
}

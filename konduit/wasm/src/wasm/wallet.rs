use crate::{
    core, wallet,
    wasm::{self, Connector, Credential, ShelleyAddress, TransactionSummary, VerificationKey},
    wasm_proxy,
};
use anyhow::anyhow;
use std::{ops::Deref, rc::Rc};
use wasm_bindgen::prelude::*;

wasm_proxy! {
    #[derive(Debug, Clone)]
    #[doc = "A rudimentary wallet interface"]
    Wallet => Rc<wallet::Wallet>
}

impl Wallet {
    pub fn new(network_id: core::NetworkId, signing_key: core::SigningKey) -> Self {
        Self(Rc::new(wallet::Wallet::new(network_id, signing_key)))
    }

    pub fn signing_key(&self) -> &core::SigningKey {
        &self.signing_key
    }

    pub fn verification_key(&self) -> core::VerificationKey {
        self.signing_key().to_verification_key()
    }

    pub fn stake_credential(&self) -> Option<core::Credential> {
        self.stake_credential.borrow().clone()
    }
}

#[wasm_bindgen]
impl Wallet {
    #[wasm_bindgen(getter, js_name = "verificationKey")]
    pub fn _wasm_verification_key(&self) -> VerificationKey {
        self.verification_key().into()
    }

    #[wasm_bindgen(getter, js_name = "paymentCredential")]
    pub fn _wasm_payment_credential(&self) -> Credential {
        self._wasm_verification_key().to_credential().into()
    }

    #[wasm_bindgen(getter, js_name = "stakeCredential")]
    pub fn _wasm_stake_credential(&self) -> Option<Credential> {
        self.stake_credential().clone().map(Into::into)
    }

    #[wasm_bindgen(setter, js_name = "stakeCredential")]
    pub fn _wasm_set_stake_credential(&self, stake_credential: &Credential) {
        *self.stake_credential.borrow_mut() = Some(stake_credential.clone().into())
    }

    #[wasm_bindgen(js_name = "resetStakeCredential")]
    pub fn _wasm_reset_stake_credential(&self) {
        *self.stake_credential.borrow_mut() = None;
    }

    #[wasm_bindgen(getter, js_name = "address")]
    pub fn _wasm_address(&self) -> ShelleyAddress {
        let mut address = self.verification_key().to_address(self.network_id);

        if let Some(stake_credential) = self.stake_credential.borrow().deref() {
            address = address.with_delegation(core::Credential::clone(stake_credential));
        }

        address.into()
    }

    #[wasm_bindgen(getter, js_name = "exitAddress")]
    pub fn _wasm_exit_address(&self) -> Option<ShelleyAddress> {
        match self.exit_address.borrow().deref() {
            Ok(exit_address) => Some(core::Address::clone(exit_address).into()),
            Err(_) => None,
        }
    }

    #[wasm_bindgen(setter, js_name = "exitAddress")]
    pub fn _wasm_set_exit_address(&self, exit_address: &ShelleyAddress) {
        *self.exit_address.borrow_mut() = Ok(exit_address.clone().into());
    }

    #[wasm_bindgen(js_name = "resetExitAddress")]
    pub fn _wasm_reset_exit_address(&self) {
        *self.exit_address.borrow_mut() = Err(anyhow!("no exit address"));
    }

    /// Retrieve the balance of the underlying L1 wallet.
    #[wasm_bindgen(js_name = "balance")]
    pub async fn _wasm_balance(&self, connector: &Connector) -> wasm::Result<u64> {
        Ok(connector.balance(self.verification_key()).await?)
    }

    /// Retrieve the transaction activity around the underlying L1 wallet. This includes channels
    /// opening and closing, but not intermediate operation on channels that do not involve the
    /// wallet.
    #[wasm_bindgen(js_name = "transactions")]
    pub async fn _wasm_transactions(
        &self,
        connector: &Connector,
    ) -> wasm::Result<Vec<TransactionSummary>> {
        Ok(connector
            .transactions(&self._wasm_payment_credential())
            .await?
            .into_iter()
            .map(From::from)
            .collect())
    }
}

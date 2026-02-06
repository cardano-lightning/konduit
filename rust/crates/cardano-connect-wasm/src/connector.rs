use anyhow::anyhow;
use cardano_connect::{CardanoConnect, Network};
use cardano_tx_builder::{
    Address, Credential, Input, Output, ProtocolParameters, SigningKey, Transaction,
    VerificationKey,
    cbor::ToCbor,
    transaction::{TransactionReadyForSigning, state},
};
use gloo_net::http::{Request, Response};
use gloo_timers::callback::Timeout;
use std::{collections::BTreeMap, ops::Deref};
use wasm_bindgen::prelude::*;
use web_sys::{AbortController, AbortSignal};

mod balance;
mod health;
mod network;
mod utxos_at;

#[wasm_bindgen]
pub struct CardanoConnector {
    base_url: String,
    http_timeout_ms: u32,
    network: Network,
}

// -------------------------------------------------------------------- WASM API

#[wasm_bindgen]
impl CardanoConnector {
    #[wasm_bindgen]
    pub async fn new(base_url: &str, http_timeout_ms: Option<u32>) -> crate::Result<Self> {
        let http_timeout_ms = http_timeout_ms.unwrap_or(10_000);
        let base_url = base_url.strip_suffix("/").unwrap_or(base_url).to_string();
        let network = Network::Other(0);

        let mut connector = Self {
            base_url,
            http_timeout_ms,
            network,
        };

        let network = connector
            .get::<network::Response>("/network", http_timeout_ms)
            .await?
            .network;

        match network.as_str() {
            "mainnet" => {
                connector.network = Network::Mainnet;
            }
            "preprod" => {
                connector.network = Network::Preprod;
            }
            "preview" => {
                connector.network = Network::Preview;
            }
            _ => Err(anyhow!("unsupported network: {}", network))?,
        };

        Ok(connector)
    }

    #[wasm_bindgen(js_name = "signAndSubmit")]
    pub async fn sign_and_submit(
        &self,
        transaction: &mut TransactionReadyForSigning,
        signing_key: &[u8],
    ) -> crate::Result<Vec<u8>> {
        let signing_key: SigningKey = <[u8; 32]>::try_from(signing_key)
            .map_err(|_| anyhow!("invalid signing key length"))?
            .into();

        transaction.sign(signing_key);

        let tx_hash = transaction.id();

        self.submit(transaction.deref()).await?;

        Ok(tx_hash.as_ref().into())
    }

    #[wasm_bindgen]
    pub async fn balance(&self, verification_key: &[u8]) -> crate::Result<u64> {
        let verification_key: VerificationKey = <[u8; 32]>::try_from(verification_key)
            .map_err(|_| anyhow!("invalid verification key length"))?
            .into();

        let addr = Address::new(self.network().into(), Credential::from(verification_key));

        let balance = self
            .get::<balance::Response>(&format!("/balance/{addr}"), self.http_timeout_ms)
            .await?;

        Ok(balance.lovelace.parse::<u64>().map_err(|e| anyhow!(e))?)
    }

    #[wasm_bindgen(getter)]
    pub fn network_magic_number(&self) -> u64 {
        self.network.into()
    }
}

// -------------------------------------------------------------------- Internals

#[wasm_bindgen]

impl CardanoConnector {
    async fn get<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        timeout_ms: u32,
    ) -> anyhow::Result<T> {
        let (abort_on_timeout, timeout_handle) = Self::mk_abort_on_timeout(timeout_ms)?;
        let request = Request::get(&format!("{}{path}", self.base_url))
            .abort_signal(Some(&abort_on_timeout))
            .build()?;
        let result = self.send::<T>(request).await;
        timeout_handle.cancel();
        result
    }

    fn mk_abort_on_timeout(timeout_ms: u32) -> anyhow::Result<(AbortSignal, Timeout)> {
        let controller =
            AbortController::new().map_err(|_| anyhow!("Failed to create AbortController"))?;
        let signal: AbortSignal = controller.signal();
        let timeout_controller = controller.clone(); // Clone for move into closure
        let timeout_handle = Timeout::new(timeout_ms, move || {
            timeout_controller.abort();
            log::warn!("Aborted request due to timeout after {}ms", timeout_ms);
        });
        anyhow::Ok((signal, timeout_handle))
    }

    async fn post<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: impl Into<JsValue>,
        timeout_ms: u32,
    ) -> anyhow::Result<T> {
        let body = js_sys::JSON::stringify(&body.into())
            .map_err(|e| anyhow!("failed to serialize request body: {:?}", e))?;
        let (abort_on_timeout, timeout_handle) = Self::mk_abort_on_timeout(timeout_ms)?;
        let request = Request::post(&format!("{}{path}", self.base_url))
            .abort_signal(Some(&abort_on_timeout))
            .body(body)?;

        let result = self.send::<T>(request).await;
        timeout_handle.cancel();
        result
    }

    async fn send<T: serde::de::DeserializeOwned>(&self, request: Request) -> anyhow::Result<T> {
        let method = request.method();
        let url = request.url();
        let title = format!("{method} {url}");
        let title_str = title.as_str();

        let response = request.send().await.map_err(|e| {
            log::error!("{title_str} failed: {e:?}");
            anyhow!(e)
        })?;

        Self::handle_non_success(title_str, &response).await?;

        response
            .json()
            .await
            .map_err(|e| anyhow!("invalid JSON response from backend: {e}"))
    }

    async fn handle_non_success(title: &str, response: &Response) -> anyhow::Result<()> {
        if !response.ok() {
            return Err(anyhow!(
                "{title} failed (status={}): {:?}",
                response.status(),
                response
                    .text()
                    .await
                    .unwrap_or_else(|_| "unable to decode Response body".to_string()),
            ));
        }

        Ok(())
    }
}

// -------------------------------------------------------- CardanoConnect Trait

impl CardanoConnect for CardanoConnector {
    fn network(&self) -> Network {
        self.network
    }

    async fn health(&self) -> anyhow::Result<String> {
        let health = self
            .get::<health::Response>("/health", self.http_timeout_ms)
            .await?;
        Ok(health.status)
    }

    async fn protocol_parameters(&self) -> anyhow::Result<ProtocolParameters> {
        match self.network {
            Network::Mainnet => Ok(ProtocolParameters::mainnet()),
            Network::Preprod => Ok(ProtocolParameters::preprod()),
            Network::Preview => Ok(ProtocolParameters::preview()),
            Network::Other(..) => Err(anyhow!(
                "protocol parameters for 'other' networks are not supported"
            )),
        }
    }

    /// If delegation is None then it _should_ be ignored:
    /// Any address with matching payment credential should be returned.
    async fn utxos_at(
        &self,
        payment: &Credential,
        delegation: Option<&Credential>,
    ) -> anyhow::Result<BTreeMap<Input, Output>> {
        let mut addr = Address::new(self.network().into(), payment.clone());
        if let Some(delegation) = delegation {
            addr = addr.with_delegation(delegation.clone());
        }

        let utxos = self
            .get::<Vec<utxos_at::Response>>(&format!("/utxos_at/{addr}"), self.http_timeout_ms)
            .await?;

        Ok(utxos
            .into_iter()
            .map(|obj| <(Input, Output)>::try_from(obj).expect("failed to convert UtxoObject"))
            .collect())
    }

    async fn submit(
        &self,
        transaction: &Transaction<state::ReadyForSigning>,
    ) -> anyhow::Result<()> {
        let body = singleton("transaction", hex::encode(transaction.to_cbor()))?;

        self.post::<serde_json::Value>("/submit", body, self.http_timeout_ms)
            .await?;

        Ok(())
    }
}

// --------------------------------------------------------------------- Helpers

fn singleton(key: &str, value: impl Into<JsValue>) -> anyhow::Result<JsValue> {
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &(key.into()), &(value.into())).map_err(|e| {
        anyhow!(
            "failed to construct singleton object with key '{}': {:?}",
            key,
            e
        )
    })?;
    Ok(obj.into())
}

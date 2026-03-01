use crate::{
    Adaptor, Connector, HttpClient, core, l1, l2,
    wasm::{
        self, AdaptorInfo, ChannelOutput, Hash32, NetworkId, Quote, ShelleyAddress, SigningKey,
        Tag, Wallet,
    },
};
use anyhow::anyhow;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

/// A 'black-box' API for Konduit L1 & L2 operations.
#[wasm_bindgen]
pub struct Konduit {
    network_id: NetworkId,
    script_deployment_address: ShelleyAddress,
    wallet: Wallet,
    connector: wasm::Result<Rc<Connector<HttpClient>>>,
    adaptor: wasm::Result<Adaptor<HttpClient>>,
}

impl Konduit {
    fn l1_client(&self) -> wasm::Result<l1::Client<'_, Connector<HttpClient>>> {
        Ok(l1::Client::new(
            self.connector.as_ref()?,
            self.wallet.signing_key(),
        ))
    }

    fn l2_client(&self) -> wasm::Result<l2::Client<'_, HttpClient>> {
        Ok(l2::Client::new(
            self.adaptor.as_ref()?,
            self.wallet.signing_key(),
        ))
    }
}

/// A 'black-box' API for Konduit L1 & L2 operations.
#[wasm_bindgen]
impl Konduit {
    /// Create a brand new instance using an internally generated signing key.
    ///
    /// By default, the instance comes without adaptor, connector or channel tag set. These can be
    /// recovered using specific setters on the object.
    #[wasm_bindgen(constructor)]
    pub fn new(network_id: &NetworkId, script_deployment_address: &ShelleyAddress) -> Self {
        Self::from_signing_key(
            network_id,
            script_deployment_address,
            SigningKey::_wasm_new(),
        )
    }

    /// Restore an instance from a signing key. Everything else (connector, adaptor, ...) is
    /// initially NOT configured.
    #[wasm_bindgen(js_name = "fromSigningKey")]
    pub fn from_signing_key(
        network_id: &NetworkId,
        script_deployment_address: &ShelleyAddress,
        signing_key: SigningKey,
    ) -> Self {
        Konduit {
            network_id: *network_id,
            script_deployment_address: script_deployment_address.clone(),
            wallet: Wallet::new(*network_id, signing_key),
            connector: Err(anyhow!("no available connector").into()),
            adaptor: Err(anyhow!("no available adaptor").into()),
        }
    }

    /// A handle on the underlying wallet.
    #[wasm_bindgen(getter, js_name = "wallet")]
    pub fn wallet(&self) -> Wallet {
        self.wallet.clone()
    }

    /// Current network id for which the app is configured.
    #[wasm_bindgen(getter, js_name = "networkId")]
    pub fn network_id(&self) -> NetworkId {
        self.network_id
    }
}

// Connector-related interface
#[wasm_bindgen]
impl Konduit {
    /// Get a reference to the connector.
    #[wasm_bindgen(getter, js_name = "connector")]
    pub fn _wasm_connector(&self) -> wasm::Result<wasm::Connector> {
        self.connector.clone().map(Into::into)
    }

    /// Configure or reconfigure the associated connector for the instance.
    #[wasm_bindgen(js_name = "setConnector")]
    pub async fn set_connector(&mut self, url: &str) -> wasm::Result<()> {
        if self.connector.as_ref().is_ok_and(|c| c.base_url() == url) {
            log::debug!("set_connector: connector already set to {url}.");
            return Ok(());
        }

        let connector = Rc::new(Connector::new(HttpClient::new(url)).await?);

        self.connector = Ok(connector);

        log::debug!("set_connector: connector set to {url}.");

        Ok(())
    }
}

// Adaptor-related interface
#[wasm_bindgen]
impl Konduit {
    /// Configure an (unauthenticated) adaptor, without a defined tag yet. Suitable to get the
    /// adaptor info and other non-authenticated operations.
    #[wasm_bindgen(js_name = "setAdaptor")]
    pub async fn set_adaptor(&mut self, url: &str) -> wasm::Result<AdaptorInfo> {
        if let Ok(adaptor) = self.adaptor.as_ref()
            && adaptor.base_url() == url
        {
            log::debug!("set_adaptor: adaptor already set to {url}.");
            return Ok(adaptor.info().clone().into());
        }

        let adaptor = Adaptor::new(HttpClient::new(url), None).await?;
        let adaptor_info: AdaptorInfo = adaptor.info().clone().into();

        self.adaptor = Ok(adaptor);

        log::debug!("set_adaptor: adaptor already set to {url}.");

        Ok(adaptor_info)
    }

    /// Reset a previously known tag, if any.
    #[wasm_bindgen(js_name = "setChannelTag")]
    pub fn set_channel_tag(&mut self, tag: &Tag) -> wasm::Result<()> {
        if self.adaptor.as_ref()?.tag() != Some(tag) {
            let tag: core::Tag = tag.clone().into();
            let keytag = core::Keytag::new(self.wallet.verification_key().into(), tag.clone());
            self.adaptor.as_mut()?.set_keytag(Some(&keytag));
        }

        Ok(())
    }

    /// Re-sync the open channel, if any, with the adaptor, squashing anything that needs squashing
    /// behind the scene. And return the total provably owed amount (i.e. total squashed, plus
    /// locked and unlocked).
    #[wasm_bindgen(js_name = "syncChannel")]
    pub async fn sync_channel(&self) -> wasm::Result<u64> {
        if let Some(tag) = self.adaptor.as_ref()?.tag() {
            log::debug!("sync_channel: for tag = {}", tag);
            let client = self.l2_client()?;

            // 1. Inspect the receipt to collect the amount we owe the adaptor, but only trust squash
            //    and cheques that verify.
            let owed = if let Some(receipt) = client.receipt().await? {
                receipt.provably_owed(&self.wallet.verification_key(), tag)
            } else {
                0
            };

            // 2. Regardless, always send a null squash to check if there's any unresolved state to
            //    settle with the server.
            let squash_response = client.squash(core::SquashBody::default()).await?;
            client.sync(squash_response, true).await?;

            Ok(owed)
        } else {
            log::debug!("sync_channel: no tag set, assuming no channel to sync.");
            Ok(0)
        }
    }

    /// Get a quote for a given Bolt11 invoice from the adapator.
    #[wasm_bindgen(js_name = "getQuoteFor")]
    pub async fn get_quote_for(&self, invoice: &str) -> wasm::Result<Quote> {
        Ok(self.l2_client()?.quote(invoice).await?.into())
    }

    /// Pay an invoice using a previously established quote.
    #[wasm_bindgen(js_name = "pay")]
    pub async fn pay(&self, invoice: &str, quote: &Quote) -> wasm::Result<()> {
        let client = self.l2_client()?;
        let squash_status = client.pay(invoice, quote).await?;
        client.sync(squash_status, true).await?;
        Ok(())
    }
}

// Channel(s) related interfaces
#[wasm_bindgen]
impl Konduit {
    /// Open a channel with the given tag and initial deposit.
    #[wasm_bindgen(js_name = "openChannel")]
    pub async fn open_channel(&mut self, tag: &Tag, amount: u64) -> wasm::Result<Hash32> {
        let tag: core::Tag = tag.clone().into();

        log::debug!("open_channel: for tag = {}", tag);

        let adaptor = self.adaptor.as_ref()?;
        let adaptor_key = adaptor.info().channel_parameters.adaptor_key;
        let close_period = adaptor.info().channel_parameters.close_period;

        let opens = vec![core::consumer::OpenIntent {
            tag: tag.clone(),
            sub_vkey: adaptor_key,
            close_period,
            amount,
        }];

        let open_tx: Hash32 = self
            .l1_client()?
            .execute(
                self.wallet.signing_key(),
                self.wallet.stake_credential().as_deref(),
                opens,
                Default::default(),
                &self.script_deployment_address,
            )
            .await?
            .into();

        self.set_channel_tag(&tag.into())?;

        Ok(open_tx)
    }

    /// Find currently opened channels that belongs to "us"
    #[wasm_bindgen(js_name = "openedChannels")]
    pub async fn opened_channels(&self) -> wasm::Result<Vec<ChannelOutput>> {
        Ok(self
            .l1_client()?
            .opened_channels(self.wallet.stake_credential().as_deref())
            .await?
            .map(Into::into)
            .collect())
    }

    /// Close the currently active channel, if any.
    #[wasm_bindgen(js_name = "closeChannel")]
    pub async fn close_channel(&mut self) -> wasm::Result<Hash32> {
        let tag: core::Tag = self
            .adaptor
            .as_ref()?
            .tag()
            .ok_or::<wasm::Error>(
                anyhow!("close_channel: no tag set: attempting to close non-existing channel?")
                    .into(),
            )?
            .clone();

        let close_tx: Hash32 = self
            .l1_client()?
            .execute(
                self.wallet.signing_key(),
                self.wallet.stake_credential().as_deref(),
                vec![],
                From::from([(tag, core::consumer::Intent::Close)]),
                &self.script_deployment_address.clone(),
            )
            .await?
            .into();

        self.adaptor.as_mut()?.set_keytag(None);

        Ok(close_tx)
    }
}

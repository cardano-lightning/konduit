use crate::{
    Adaptor, Connector, HttpClient, core, l1, l2,
    wasm::{
        self, AdaptorInfo, ChannelOutput, Hash32, Lock, Lockeds, NetworkId, Quote, ShelleyAddress,
        SigningKey, Tag, Wallet,
    },
};
use anyhow::anyhow;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_time::{Duration, SystemTime, UNIX_EPOCH};

/// A period during which we tolerate the adaptor to hold onto an extra locked cheque. This
/// effectively extends the timeout, simply to ensure that clocks skew between the consumer and
/// adaptor aren't causing any weird concurrence issue.
///
/// In principle, this can be relatively small but there's no particular risk in having it high. It
/// simply increases the delay before which we consider a payment "expired".
const LOCKED_CHEQUE_GRACE_PERIOD: Duration = Duration::from_secs(60);

/// A 'black-box' API for Konduit L1 & L2 operations.
#[wasm_bindgen]
pub struct Konduit {
    _network_id: NetworkId,

    // Address at which is deployed the Konduit validator. We could also cache the UTxO
    // corresponding to the address to avoid re-fetching it on every single request.
    _script_deployment_address: ShelleyAddress,

    // Rudimentary wallet holding the consumer's credentials.
    _wallet: Wallet,

    // Connection to a connector, as a result since it could initially be missing.
    _connector: wasm::Result<Rc<Connector<HttpClient>>>,

    // Connection to an adaptor, as a result since it could initially be missing.
    _adaptor: wasm::Result<Adaptor<HttpClient>>,
}

impl Konduit {
    fn l1_client(&self) -> wasm::Result<l1::Client<'_, Connector<HttpClient>>> {
        Ok(l1::Client::new(
            self._connector.as_ref()?,
            self._wallet.signing_key(),
        ))
    }

    fn l2_client(&self) -> wasm::Result<l2::Client<'_, HttpClient>> {
        Ok(l2::Client::new(
            self._adaptor.as_ref()?,
            self._wallet.signing_key(),
        ))
    }

    async fn squash(
        &self,
        client: l2::Client<'_, HttpClient>,
        tag: &core::Tag,
        squash_status: core::SquashStatus,
        lockeds: &mut Lockeds,
    ) -> wasm::Result<SyncStatus> {
        let mut sync_status = SyncStatus {
            owed: 0,
            squashed: client
                .sync(squash_status, true, lockeds.as_filter())
                .await?
                .into_iter()
                .map(From::from)
                .collect(),
        };

        sync_status.owed = if let Some(mut receipt) = client.receipt().await? {
            // Prune expired *locked* cheques, including a grace period.
            let now = get_current_time() - LOCKED_CHEQUE_GRACE_PERIOD;
            receipt.timeout(core::Duration::from_secs(now.as_secs()));

            // Update our internal state with the remaining locked cheques.
            lockeds.reset(
                receipt
                    .lockeds()
                    .into_iter()
                    .map(|locked| *locked.lock())
                    .collect(),
            );

            // We have *just squashed* everything with the adaptor, hence we do not expect any
            // unlocked cheques to be present in the receipt. If it's the case, then the
            // adaptor is doing something odd and we should abort.
            if !receipt.unlockeds().is_empty() {
                return Err(anyhow!(
                    "found unlocked cheques even after squashing; adaptor is onto something..."
                )
                .into());
            }

            receipt.provably_owed(&self._wallet.verification_key(), tag)
        } else {
            0
        };

        log::debug!("sync_channel: {:#?}", sync_status,);

        Ok(sync_status)
    }
}

/// A 'black-box' API for Konduit L1 & L2 operations.
#[wasm_bindgen]
impl Konduit {
    /// Restore an instance from a signing key. Everything else (connector, adaptor, ...) is
    /// initially NOT configured.
    ///
    /// Note that this take ownership of the signing key /!\, to prevent it from leaking elsewhere
    /// afterwards.
    #[wasm_bindgen(constructor)]
    pub fn new(
        network_id: &NetworkId,
        script_deployment_address: &ShelleyAddress,
        signing_key: SigningKey,
    ) -> Self {
        Konduit {
            _network_id: *network_id,
            _script_deployment_address: script_deployment_address.clone(),
            _wallet: Wallet::new(*network_id, signing_key),
            _connector: Err(anyhow!("no available connector").into()),
            _adaptor: Err(anyhow!("no available adaptor").into()),
        }
    }

    /// A handle on the underlying wallet.
    #[wasm_bindgen(getter, js_name = "wallet")]
    pub fn _wasm_wallet(&self) -> Wallet {
        self._wallet.clone()
    }

    /// Current network id for which the app is configured.
    #[wasm_bindgen(getter, js_name = "networkId")]
    pub fn network_id(&self) -> NetworkId {
        self._network_id
    }
}

// Connector-related interface
#[wasm_bindgen]
impl Konduit {
    /// Get a reference to the connector.
    #[wasm_bindgen(getter, js_name = "connector")]
    pub fn _wasm_connector(&self) -> wasm::Result<wasm::Connector> {
        self._connector.clone().map(Into::into)
    }

    /// Configure or reconfigure the associated connector for the instance.
    #[wasm_bindgen(js_name = "setConnector")]
    pub async fn set_connector(&mut self, url: &str) -> wasm::Result<()> {
        if self._connector.as_ref().is_ok_and(|c| c.base_url() == url) {
            log::debug!("set_connector: connector already set to {url}.");
            return Ok(());
        }

        let connector = Rc::new(Connector::new(HttpClient::new(url)).await?);

        self._connector = Ok(connector);

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
        if let Ok(adaptor) = self._adaptor.as_ref()
            && adaptor.base_url() == url
        {
            log::debug!("set_adaptor: adaptor already set to {url}.");
            return Ok(adaptor.info().clone().into());
        }

        let adaptor = Adaptor::new(HttpClient::new(url), None).await?;
        let adaptor_info: AdaptorInfo = adaptor.info().clone().into();

        self._adaptor = Ok(adaptor);

        log::debug!("set_adaptor: adaptor already set to {url}.");

        Ok(adaptor_info)
    }

    /// Recover a previously known tag, if any.
    #[wasm_bindgen(js_name = "setChannelTag")]
    pub fn set_channel_tag(&mut self, tag: &Tag) -> wasm::Result<()> {
        if self._adaptor.as_ref()?.tag() != Some(tag) {
            let tag: core::Tag = tag.clone().into();
            let keytag = core::Keytag::new(self._wallet.verification_key().into(), tag.clone());
            self._adaptor.as_mut()?.set_keytag(Some(&keytag));
        }

        Ok(())
    }

    /// Get a quote for a given Bolt11 invoice from the adapator.
    #[wasm_bindgen(js_name = "getQuoteFor")]
    pub async fn get_quote_for(&self, invoice: &str) -> wasm::Result<Quote> {
        Ok(self.l2_client()?.quote(invoice).await?.into())
    }

    /// Synchronize the channel with the adaptor.
    #[wasm_bindgen(js_name = "syncChannel")]
    pub async fn sync_channel(&mut self, lockeds: &mut Lockeds) -> wasm::Result<SyncStatus> {
        if let Some(tag) = self._adaptor.as_ref()?.tag() {
            log::debug!("sync_channel: for tag={}", tag);
            let client = self.l2_client()?;
            let squash_status = client.squash(core::SquashBody::default()).await?;
            self.squash(client, tag, squash_status, lockeds).await
        } else {
            log::debug!("sync_channel: no tag set, assuming no channel to sync.");
            Ok(SyncStatus::default())
        }
    }

    /// Pay an invoice using a previously established quote.
    #[wasm_bindgen(js_name = "pay")]
    pub async fn pay(
        &mut self,
        invoice: &str,
        quote: &Quote,
        lockeds: &mut Lockeds,
    ) -> wasm::Result<SyncStatus> {
        if let Some(tag) = self._adaptor.as_ref()?.tag() {
            log::debug!("pay: for tag={}, quote={:?}", tag, quote);
            let client = self.l2_client()?;
            lockeds.add(quote.lock);
            let squash_status = client.pay(invoice, quote).await?;
            self.squash(client, tag, squash_status, lockeds).await
        } else {
            Err(anyhow!("pay: no tag set; is the channel open?").into())
        }
    }
}

// Channel(s) related interfaces
#[wasm_bindgen]
impl Konduit {
    /// Find channels that belongs to "us"
    #[wasm_bindgen(js_name = "channels")]
    pub async fn channels(&self) -> wasm::Result<Vec<ChannelOutput>> {
        Ok(self
            .l1_client()?
            .channels(self._wallet.stake_credential().as_deref())
            .await?
            .map(Into::into)
            .collect())
    }

    /// Open a channel with the given tag and initial deposit.
    #[wasm_bindgen(js_name = "openChannel")]
    pub async fn open_channel(&mut self, tag: &Tag, amount: u64) -> wasm::Result<Hash32> {
        let tag: core::Tag = tag.clone().into();

        log::debug!("open_channel: for tag = {}", tag);

        let adaptor = self._adaptor.as_ref()?;
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
                self._wallet.signing_key(),
                self._wallet.stake_credential().as_deref(),
                opens,
                Default::default(),
                &self._script_deployment_address,
            )
            .await?
            .into();

        self.set_channel_tag(&tag.into())?;

        Ok(open_tx)
    }

    /// Add funds to an existing channel.
    #[wasm_bindgen(js_name = "addToChannel")]
    pub async fn add_to_channel(&mut self, amount: u64) -> wasm::Result<Hash32> {
        let tag: core::Tag = self
            ._adaptor
            .as_ref()?
            .tag()
            .ok_or::<wasm::Error>(
                anyhow!("add_to_channel: no tag set: attempting to add to non-existing channel?")
                    .into(),
            )?
            .clone();

        let add_tx: Hash32 = self
            .l1_client()?
            .execute(
                self._wallet.signing_key(),
                self._wallet.stake_credential().as_deref(),
                vec![],
                From::from([(tag, core::consumer::Intent::Add(amount))]),
                &self._script_deployment_address.clone(),
            )
            .await?
            .into();

        Ok(add_tx)
    }

    /// Close the currently active channel, if any.
    #[wasm_bindgen(js_name = "closeChannel")]
    pub async fn close_channel(&mut self) -> wasm::Result<Hash32> {
        let tag: core::Tag = self
            ._adaptor
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
                self._wallet.signing_key(),
                self._wallet.stake_credential().as_deref(),
                vec![],
                From::from([(tag, core::consumer::Intent::Close)]),
                &self._script_deployment_address.clone(),
            )
            .await?
            .into();

        self._adaptor.as_mut()?.set_keytag(None);

        Ok(close_tx)
    }
}

fn get_current_time() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|e| unreachable!("couldn't compute duration since UNIX epoch: {e} !?"))
}

#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
#[doc = "A summary of a syncChannel operation."]
pub struct SyncStatus {
    pub owed: u64,
    // Unlocked cheques that have just been squashed.
    squashed: Vec<Lock>,
}

#[wasm_bindgen]
impl SyncStatus {
    #[wasm_bindgen(getter, js_name = "squashed")]
    pub fn _wasm_squashed(&self) -> Vec<Lock> {
        self.squashed.clone()
    }
}

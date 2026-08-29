use std::{collections::BTreeMap, sync::Arc};

use cardano_connector::CardanoConnector;
use cardano_connector_direct::Blockfrost;
use cardano_sdk::{
    Address, Hash, Input, NetworkId, Output, PlutusScript, ProtocolParameters, Signature,
    Transaction, Value, VerificationKey,
    address::kind::{self, Shelley},
    transaction::state::ReadyForSigning,
};
use cardano_wallet::{Embedded, Wallet};
use serde::{Deserialize, Serialize};

use crate::{Addressbook, Config, NetworkParameters, Tip, Waiter, addressbook, waiter};

/// The addressbook label the wallet's own address is always registered
/// under - see `Session::new`/`load_addressbook`.
const WALLET_LABEL: &str = "wallet";

/// Which path `sign_and_submit` uses to broadcast.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum SubmitVia {
    #[default]
    Wallet,
    Connector,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // TODO: CardanoConnector doesn't expose a nameable error type yet.
    #[error("chain connector error: {0}")]
    Connector(String),
    #[error("wallet error: {0}")]
    Wallet(String),
    #[error("failed to build tx: {0}")]
    BuildTx(String),
    #[error("waiter: {0}")]
    Timeout(#[from] waiter::Error),
    #[error("addressbook: {0}")]
    Addressbook(#[from] addressbook::Error),
    #[error("no ref script tracked for hash {0:?}")]
    RefScriptNotFound(Hash<28>),
    #[error("refusing to untrack the wallet's own address")]
    ProtectedAddress,
}

pub struct Session<C, W> {
    wallet: W,
    cardano: Arc<C>,
    waiter: Waiter,
    tip: Tip,
    // Not auto-synced with `tip` in general - kept in step by
    // `track`/`untrack`/`forget`. `load_tip`/`load_addressbook` each
    // independently re-establish only the wallet-address invariant; see
    // their docs.
    addressbook: Addressbook,
    // Cached at construction - addresses don't change mid-session.
    wallet_address: Address<Shelley>,
    submit_via: SubmitVia,
    network_parameters: NetworkParameters,
}

// TODO:: Should this be done by some other means? deref to wallet?
impl<C: CardanoConnector, W: Wallet> Wallet for Session<C, W> {
    type Error = Error;

    async fn network_id(&self) -> Result<NetworkId, Self::Error> {
        Ok(self.network_parameters.network_id)
    }

    async fn change_address(&self) -> Result<Address<kind::Any>, Self::Error> {
        Ok(self.wallet_address.clone().into())
    }

    /// Reads `tip`'s cache at the wallet's own address instead of
    /// calling the underlying wallet
    async fn utxos(
        &self,
        _value: Option<Value<u64>>,
    ) -> Result<Option<BTreeMap<Input, Output>>, Self::Error> {
        Ok(self.utxos_at(&self.wallet_address).cloned())
    }

    async fn sign_tx(
        &self,
        tx: &Transaction<ReadyForSigning>,
    ) -> Result<(VerificationKey, Signature), Self::Error> {
        self.wallet
            .sign_tx(tx)
            .await
            .map_err(|e| Error::Wallet(e.to_string()))
    }

    /// Always submits via the wallet directly - unlike
    /// `sign_and_submit`, this doesn't consult `submit_via`.
    async fn submit(&self, tx: &Transaction<ReadyForSigning>) -> Result<Hash<32>, Self::Error> {
        self.wallet
            .submit(tx)
            .await
            .map_err(|e| Error::Wallet(e.to_string()))
    }
}

impl Session<Blockfrost, Embedded<Blockfrost>> {
    pub async fn init(config: Config) -> Result<Self, Error> {
        let cardano = Arc::new(config.cardano.build());
        let wallet = Embedded::new(cardano.clone(), config.wallet, None);
        Session::new(cardano, wallet, Waiter::new(config.wait), config.submit_via).await
    }

    #[cfg(feature = "direct-embedded")]
    pub async fn direct_embedded(blockfrost_project_id: impl Into<String>) -> Result<Self, Error> {
        let cardano = Arc::new(Blockfrost::new(blockfrost_project_id.into()));
        let wallet = Embedded::new(cardano.clone(), cardano_wallet::Config::default(), None);
        Session::new(
            cardano,
            wallet,
            Waiter::new(waiter::Config::default()),
            SubmitVia::default(),
        )
        .await
    }
}

impl<C: CardanoConnector, W: Wallet> Session<C, W> {
    // ---- construction ----------------------------------------------

    pub async fn new(
        cardano: Arc<C>,
        wallet: W,
        waiter: Waiter,
        submit_via: SubmitVia,
    ) -> Result<Self, Error> {
        let network_id: NetworkId = wallet
            .network_id()
            .await
            .map_err(|e| Error::Wallet(e.to_string()))?;
        let wallet_address = wallet
            .change_address()
            .await
            .map_err(|e| Error::Wallet(e.to_string()))?
            .as_shelley()
            .ok_or(Error::Wallet("only shelley support".to_string()))?;
        let protocol_parameters = cardano
            .protocol_parameters()
            .await
            .map_err(|e| Error::Connector(e.to_string()))?;
        // The wallet's own address is always tracked and always labelled
        // - `Addressbook::default()` is empty, so this can't collide.
        let mut addressbook = Addressbook::default();
        addressbook.insert(WALLET_LABEL.to_string(), wallet_address.clone())?;
        let mut tip = Tip::empty();
        tip.track(wallet_address.clone());
        Ok(Self {
            wallet,
            wallet_address,
            submit_via,
            waiter,
            cardano,
            network_parameters: NetworkParameters {
                network_id,
                protocol_parameters,
            },
            tip,
            addressbook,
        })
    }

    // ---- accessors ---------------------------------------------------

    pub fn change_address(&self) -> Address<Shelley> {
        self.wallet_address.clone()
    }

    pub fn network_id(&self) -> NetworkId {
        self.network_parameters.network_id
    }

    pub fn protocol_parameters(&self) -> &ProtocolParameters {
        &self.network_parameters.protocol_parameters
    }

    pub fn network_parameters(&self) -> &NetworkParameters {
        &self.network_parameters
    }

    pub fn tip(&self) -> &Tip {
        &self.tip
    }

    pub fn addressbook(&self) -> &Addressbook {
        &self.addressbook
    }

    /// Every address currently tracked in `tip`.
    pub fn tracked(&self) -> impl Iterator<Item = &Address<Shelley>> {
        self.tip.addresses()
    }

    /// Resolves a label or literal address via the addressbook.
    pub fn resolve(&self, input: &str) -> Result<Address<Shelley>, Error> {
        Ok(self.addressbook.resolve(input)?)
    }

    /// `None` until `refresh_at`/`refresh_many`/`refresh_all` has run for
    /// `address`.
    pub fn utxos_at(&self, address: &Address<Shelley>) -> Option<&BTreeMap<Input, Output>> {
        self.tip.utxos_at(address)
    }

    /// Whether `input` (a label or literal address) is currently
    /// tracked in `tip`.
    pub fn is_tracked(&self, input: &str) -> Result<bool, Error> {
        let address = self.addressbook.resolve(input)?;
        Ok(self.tip.is_tracked(&address))
    }

    /// Cached utxos for `input` (a label or literal address) - the
    /// `resolve` + `utxos_at` combination in one call. `Ok(None)` means
    /// `input` resolved fine but hasn't been refreshed yet.
    pub fn utxos_of(&self, input: &str) -> Result<Option<&BTreeMap<Input, Output>>, Error> {
        let address = self.addressbook.resolve(input)?;
        Ok(self.utxos_at(&address))
    }

    pub fn fuel(&self) -> BTreeMap<Input, Output> {
        self.utxos_at(&self.wallet_address)
            .into_iter()
            .flatten()
            .filter(|(_, output)| output.script().is_none())
            .map(|(input, output)| (input.clone(), output.clone()))
            .collect()
    }

    pub fn wallet_script(&self, hash: &Hash<28>) -> Option<(Input, Output)> {
        self.utxos_at(&self.wallet_address)?
            .iter()
            .find_map(|(input, output)| {
                output
                    .script()
                    .filter(|script| &Hash::<28>::from(*script) == hash)
                    .map(|_| (input.clone(), output.clone()))
            })
    }

    pub fn wallet_scripts(&self) -> impl Iterator<Item = (Hash<28>, Input, Output)> + '_ {
        self.utxos_at(&self.wallet_address)
            .into_iter()
            .flatten()
            .filter_map(|(input, output)| {
                output
                    .script()
                    .map(|script| (Hash::<28>::from(script), input.clone(), output.clone()))
            })
    }

    /// Unlike `wallet_script`, searches every tracked address, not just
    /// the wallet's.
    pub fn ref_script(&self, hash: &Hash<28>) -> Option<(Input, Output)> {
        self.tip.addresses().find_map(|address| {
            self.tip
                .utxos_at(address)?
                .iter()
                .find_map(|(input, output)| {
                    output
                        .script()
                        .filter(|script| &Hash::<28>::from(*script) == hash)
                        .map(|_| (input.clone(), output.clone()))
                })
        })
    }

    // ---- hydrating from an external cache/file ------------------------

    /// Replaces `tip` wholesale - anything not in the replacement stops
    /// being tracked, except the wallet's own address: it's re-marked as
    /// tracked (with no utxos if the replacement didn't have them, same
    /// as a fresh `Session`) since it can never be untracked.
    pub fn load_tip(&mut self, mut tip: Tip) {
        tip.track(self.wallet_address.clone());
        self.tip = tip;
    }

    /// Replaces the addressbook wholesale, then re-establishes the one
    /// invariant `Session` relies on: the wallet's own address is always
    /// labelled. Errors only if the replacement already uses
    /// `WALLET_LABEL` for a *different* address.
    pub fn load_addressbook(&mut self, mut addressbook: Addressbook) -> Result<(), Error> {
        if addressbook.get_label(&self.wallet_address).is_none() {
            addressbook.insert(WALLET_LABEL.to_string(), self.wallet_address.clone())?;
        }
        self.addressbook = addressbook;
        Ok(())
    }

    // ---- syncing tip from the chain -----------------------------------

    /// Fetches the current utxo set at `address`, without touching
    /// `tip`. The wallet's own address is special-cased to come from
    /// the wallet itself, never the connector - it's the source of
    /// truth for outputs it controls, and this can't be bypassed by
    /// going through `refresh_at`/`refresh`/`refresh_many` on it.
    async fn fetch_at(&self, address: &Address<Shelley>) -> Result<BTreeMap<Input, Output>, Error> {
        if *address == self.wallet_address {
            return Ok(self
                .wallet
                .utxos(None)
                .await
                .map_err(|e| Error::Wallet(e.to_string()))?
                .unwrap_or_default());
        }
        self.cardano
            .utxos_at(&address.payment(), address.delegation().as_ref())
            .await
            .map_err(|e| Error::Connector(e.to_string()))
    }

    pub async fn refresh_wallet(&mut self) -> Result<(), Error> {
        self.refresh_at(self.wallet_address.clone()).await
    }

    pub async fn refresh_at(&mut self, address: Address<Shelley>) -> Result<(), Error> {
        let utxos = self.fetch_at(&address).await?;
        self.tip.refresh(address, utxos);
        Ok(())
    }

    pub async fn refresh(&mut self, input: &str) -> Result<(), Error> {
        let address = self.addressbook.resolve(input)?;
        self.refresh_at(address).await
    }

    /// Refreshes every address in `addresses`. Fetches are sequential
    /// (see below) and `tip` isn't touched until all of them succeed -
    /// a failure partway through leaves `tip` exactly as it was, rather
    /// than half-updated with whichever addresses happened to fetch
    /// first.
    ///
    /// Sequential for now - switch to `futures::future::try_join_all` if
    /// this becomes a bottleneck.
    pub async fn refresh_many(
        &mut self,
        addresses: impl IntoIterator<Item = Address<Shelley>>,
    ) -> Result<(), Error> {
        let mut snapshots = Vec::new();
        for address in addresses {
            let utxos = self.fetch_at(&address).await?;
            snapshots.push((address, utxos));
        }
        self.tip.refresh_many(snapshots);
        Ok(())
    }

    pub async fn refresh_all(&mut self) -> Result<(), Error> {
        self.refresh_wallet().await?;
        // Already just refreshed above; `fetch_at` would route it
        // through the wallet again correctly if included here, but
        // there's no reason to fetch it twice.
        let addresses: Vec<_> = self
            .tip
            .addresses()
            .filter(|a| **a != self.wallet_address)
            .cloned()
            .collect();
        self.refresh_many(addresses).await
    }

    // ---- tracking (tip + addressbook together) -------------------------

    /// Labels `address` in the addressbook, and marks it as watched in
    /// `tip`. The label is mandatory: unlike the old `Option<String>`
    /// form, there's no implicit fallback to `address.to_string()` as
    /// its own label, and no ambiguity about whether the first argument
    /// is a label to reuse or an address to add - callers always hand
    /// over both explicitly. Symmetric with `untrack`: this is the only
    /// pair that touches both stores.
    pub fn track(&mut self, label: String, address: Address<Shelley>) -> Result<(), Error> {
        if let Err(err) = self.addressbook.insert(label, address.clone()) {
            if matches!(err, addressbook::Error::AlreadyExists) {
                tracing::warn!("address already exists");
            } else {
                Err(err)?;
            }
        };
        self.tip.track(address);
        Ok(())
    }

    /// Stops watching `address` and drops any label attached to it -
    /// the typed counterpart to `refresh_at`/`refresh_many`, for
    /// addresses that were only ever fetched directly and never given a
    /// label via `track`. Refuses the wallet's own address: it's always
    /// tracked and always labelled, by construction (`Session::new`,
    /// `load_tip`, `load_addressbook`), since `fuel`/`wallet_script`s
    /// depend on it never going away.
    pub fn forget(&mut self, address: &Address<Shelley>) -> Result<(), Error> {
        if *address == self.wallet_address {
            return Err(Error::ProtectedAddress);
        }
        self.addressbook.remove_by_address(address);
        self.tip.untrack(address);
        Ok(())
    }

    /// Drops the address labeled `label` from both `tip` and the
    /// addressbook. Resolution is always label-first: `label` is looked
    /// up in the addressbook, and only if no such label is known is it
    /// parsed as a literal address, as a last resort (see
    /// `Addressbook::resolve`). Once resolved, handled identically to
    /// `forget`.
    pub fn untrack(&mut self, label: &str) -> Result<(), Error> {
        let address = self.addressbook.resolve(label)?;
        self.forget(&address)
    }

    // ---- building & submitting transactions -----------------------------

    pub fn wallet_utxos(&self) -> Option<&BTreeMap<Input, Output>> {
        self.tip.utxos_at(&self.wallet_address)
    }

    pub async fn upload(&mut self, script: PlutusScript) -> Result<Hash<32>, Error> {
        self.wallet_utxos()
            .ok_or(Error::Wallet("Unsynced or no utxos".to_string()))?;
        let tx = cardano_wallet::txs::upload(
            &self.network_parameters.protocol_parameters,
            &self.fuel(),
            script,
            self.wallet_address.clone().into(),
        )
        .map_err(|e| Error::BuildTx(e.to_string()))?;
        self.sign_and_submit(tx).await
    }

    pub async fn teardown(&mut self, hash: &Hash<28>) -> Result<Hash<32>, Error> {
        // Typed pre-check - otherwise `txs::teardown` surfaces the same
        // "not found" as an opaque `BuildTx` string.
        self.wallet_script(hash)
            .ok_or(Error::RefScriptNotFound(*hash))?;
        let utxos = self
            .wallet_utxos()
            .ok_or(Error::Wallet("Unsynced or no utxos".to_string()))?;
        let tx = cardano_wallet::txs::teardown(
            &self.network_parameters.protocol_parameters,
            utxos,
            *hash,
            self.wallet_address.clone().into(),
        )
        .map_err(|e| Error::BuildTx(e.to_string()))?;
        self.sign_and_submit(tx).await
    }

    pub async fn sign_and_submit(
        &mut self,
        mut tx: Transaction<ReadyForSigning>,
    ) -> Result<Hash<32>, Error> {
        let (vkey, sig) = self
            .wallet
            .sign_tx(&tx)
            .await
            .map_err(|e| Error::Wallet(e.to_string()))?;
        tx.add_witness(vkey, sig);
        match self.submit_via {
            SubmitVia::Wallet => self
                .wallet
                .submit(&tx)
                .await
                .map_err(|e| Error::Wallet(e.to_string())),
            SubmitVia::Connector => {
                self.cardano
                    .submit(&tx)
                    .await
                    .map_err(|e| Error::Connector(e.to_string()))?;
                Ok(tx.id())
            }
        }
    }

    // ---- waiting for confirmation ----------------------------------------

    /// Caches the confirming fetch into `tip` before returning, so
    /// callers don't need a separate refresh after.
    pub async fn wait_for(
        &mut self,
        address: &Address<Shelley>,
        id: &Hash<32>,
    ) -> Result<(), Error> {
        for attempt in 0..self.waiter.max_attempts() {
            tracing::warn!("awaiting {} at {:?}, attempt {}", id, address, attempt);
            let utxos = self.fetch_at(address).await?;
            if utxos.keys().any(|i| i.transaction_id() == *id) {
                self.tip.refresh(address.clone(), utxos);
                return Ok(());
            }
            if attempt + 1 == self.waiter.max_attempts() {
                break;
            }
            self.waiter.wait().await;
        }
        Err(self.waiter.timed_out(id).into())
    }

    pub async fn wait_wallet(&mut self, id: &Hash<32>) -> Result<(), Error> {
        let wallet_address = self.wallet_address.clone();
        self.wait_for(&wallet_address, id).await
    }
}

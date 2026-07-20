use cardano_connector::CardanoConnector;
use cardano_sdk::{
    Address, Hash, NetworkId, Output, Transaction, address::kind,
    transaction::state::ReadyForSigning,
};
use konduit_data::{Stage, Tag, VerifyingKey};
use konduit_tx::{
    Channel, ChannelUtxo, NetworkParameters, Open, SteppedUtxos, find_reference_script,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::{
    Signer, Wallet,
    core::{Credential, Input, KONDUIT_VALIDATOR},
    time, utxo_batch,
};

mod policies;
pub use policies::{BoundsPolicy, Policies, SubmitPolicy};

mod config;
pub use config::Config;

mod directives;
pub use directives::{Directives, Intent, OpenIntent};

mod cache;
pub use cache::Cache;

mod error;
pub use error::Error;

/// `L1` has a single owner (there's no `Arc<L1>` fan-out — the orchestrator
/// holds one, and swaps it wholesale on config change), so `Config`,
/// `Cache`, and `Directives` are plain, lock-free fields:
///
/// - [`Config`] is caller-authored and immutable through `L1` — there's
///   no `&mut` path to it at all. To change anything in it, pull a copy
///   out with [`L1::config`], mutate the copy via `Config`'s own setters,
///   and build a fresh `L1` via [`L1::with_cache`].
/// - [`Cache`] is chain-pulled and fully disposable; a re-pull replaces
///   anything lost.
/// - [`Directives`] is pending build intent with no chain-side source to
///   recover it from.
///
/// `Cache` and `Directives` are mutated through ordinary `&mut self`
/// methods — no locking, no poisoning to account for. They're still
/// reconciled together in [`L1::set_tip`], which is the one place that
/// needs both.
pub struct L1<Connector: CardanoConnector, S: Signer, W: Wallet> {
    connector: Arc<Connector>,
    signer: Arc<S>,
    wallet: Arc<W>,
    config: Config,
    cache: Cache,
    directives: Directives,
}

impl<Connector, S, W> L1<Connector, S, W>
where
    Connector: CardanoConnector,
    S: Signer,
    W: Wallet,
{
    /// Fresh `L1`: no chain-pulled cache, no pending directives yet.
    pub fn new(connector: Arc<Connector>, signer: Arc<S>, wallet: Arc<W>, config: Config) -> Self {
        L1 {
            connector,
            signer,
            wallet,
            config,
            cache: Cache::default(),
            directives: Directives::default(),
        }
    }

    /// Reinstantiate with a (possibly just-updated) `Config`, carrying
    /// over previously chain-pulled `Cache` and previously-set
    /// `Directives` — e.g. after changing a policy via `config()` +
    /// `Config::set_*`, so the caller doesn't lose already-pulled
    /// channels or pending intent in the process.
    pub fn with_cache(
        connector: Arc<Connector>,
        signer: Arc<S>,
        wallet: Arc<W>,
        config: Config,
        cache: Cache,
        directives: Directives,
    ) -> Self {
        L1 {
            connector,
            signer,
            wallet,
            config,
            cache,
            directives,
        }
    }

    /// The current, caller-owned config. There is no `config_mut` —
    /// mutate a `.clone()` via `Config`'s own setters and pass it to
    /// [`L1::with_cache`] to get an `L1` reflecting the change.
    pub fn config(&self) -> &Config {
        &self.config
    }
    pub fn cache(&self) -> &Cache {
        &self.cache
    }
    pub fn cache_mut(&mut self) -> &mut Cache {
        &mut self.cache
    }
    pub fn directives(&self) -> &Directives {
        &self.directives
    }
    pub fn directives_mut(&mut self) -> &mut Directives {
        &mut self.directives
    }

    // add_vkey
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signer.verification_key().into_bytes().into()
    }

    /// Resolve `tag` against currently cached channels and set `intent`
    /// for the matching input(s). Reusing a tag across multiple channels
    /// is strongly discouraged but not an error: if more than one channel
    /// matches, `intent` is applied to all of them and a warning is logged.
    pub fn add_intent_by_tag(&mut self, tag: &Tag, intent: Intent) {
        let inputs: Vec<Input> = self
            .cache
            .channels()
            .iter()
            .filter(|(_, (_, channel))| channel.constants().tag == *tag)
            .map(|(input, _)| input.clone())
            .collect();
        let matched = inputs.len();
        if matched > 1 {
            log::warn!(
                "tag {tag:?} matches {matched} channels; applying intent to all of them — \
                 reusing a tag across channels is strongly discouraged",
            );
        }
        for input in inputs {
            self.directives.add_intent(input, intent.clone());
        }
    }

    /// Query the wallet's preferred change address (CIP-30
    /// `getChangeAddress`). Purely a read against the wallet — doesn't
    /// touch `Config`. To actually update the change address, fold the
    /// result into a copy of `config()` and reinstantiate via
    /// `with_cache`.
    pub async fn wallet_change_address(&self) -> Result<Address<kind::Any>, Error> {
        self.wallet
            .change_address()
            .await
            .map_err(|e| Error::Wallet(e.to_string()))
    }

    // -- Chain state, pulled explicitly, each on its own cadence --

    /// Pull the (singular) konduit reference script utxo. Changes rarely.
    pub async fn pull_reference_script(
        &mut self,
        reference_script_address: &Address<kind::Shelley>,
    ) -> Result<(), Error> {
        let utxos_at_script = self
            .connector
            .utxos_at(
                &reference_script_address.payment(),
                reference_script_address.delegation().as_ref(),
            )
            .await
            .map_err(|e| Error::Connector(e.to_string()))?;
        let reference_script =
            find_reference_script(&utxos_at_script).ok_or(Error::NoReferenceScript)?;
        self.cache.set_reference_script(reference_script);
        Ok(())
    }

    /// Pull network parameters. Changes only on hard forks / param
    /// updates — call on its own, much coarser cadence than channel pulls.
    pub async fn pull_network_parameters(&mut self) -> Result<(), Error> {
        let network_parameters = NetworkParameters {
            network_id: NetworkId::from(self.connector.network()),
            protocol_parameters: self
                .connector
                .protocol_parameters()
                .await
                .map_err(|e| Error::Connector(e.to_string()))?,
        };
        self.cache.set_network_parameters(network_parameters);
        Ok(())
    }

    /// Replace the cached tip, and drop any directive referring to an
    /// input that no longer has a live channel — the one place `Cache`
    /// and `Directives` need to be reconciled together.
    fn set_tip(
        &mut self,
        wallet_utxos: BTreeMap<Input, Output>,
        channels: BTreeMap<Input, (Output, Channel)>,
    ) {
        let live_inputs = self.cache.set_tip(wallet_utxos, channels);
        self.directives.retain_live(&live_inputs);
    }

    /// Pull konduit channel utxos across the base credential and every
    /// currently-registered delegation, parsing each into a `ChannelUtxo`
    /// and keeping only those matching this consumer's `add_vkey`. Also
    /// pulls wallet fuel utxos, since both feed the same tx-building step
    /// and share this cadence. Any pending intent whose channel no
    /// longer exists is dropped by `set_tip`; the rest are kept.
    pub async fn pull_channels(&mut self) -> Result<(), Error> {
        let delegations = self.config.delegations();
        let add_vkey = self.verifying_key();

        let payment_credential = KONDUIT_VALIDATOR.to_credential();
        let mut pairs = vec![(payment_credential.clone(), None)];
        pairs.extend(
            delegations
                .into_iter()
                .map(|d| (payment_credential.clone(), Some(d))),
        );

        let utxos_konduit = utxo_batch::utxo_batch(&*self.connector, &pairs)
            .await
            .map_err(|e| Error::Connector(e.to_string()))?;

        // NOTE: assumes `ChannelUtxo::utxo()` returns a `(Input, Output)`
        // pair by value/clone and `ChannelUtxo::data()` returns `&Channel`
        // — verify against `konduit_tx`.
        let channels: BTreeMap<Input, (Output, Channel)> = utxos_konduit
            .into_iter()
            .filter_map(|u| ChannelUtxo::try_from(u).ok())
            .filter(|u| u.data().constants().add_vkey == add_vkey)
            .map(|u| {
                let (input, output) = u.utxo().clone();
                let channel = u.data().clone();
                (input, (output, channel))
            })
            .collect();

        // NOTE: assumes `Utxos: Into<BTreeMap<Input, Output>>` — verify
        // against `konduit_tx`.
        let wallet_utxos: BTreeMap<Input, Output> = self
            .wallet
            .utxos(None)
            .await
            .map_err(|e| Error::Wallet(e.to_string()))?
            .unwrap_or_default()
            .into();

        self.set_tip(wallet_utxos, channels);
        Ok(())
    }

    /// Pull everything on its own cadence: reference script, network
    /// parameters, and channels (+ fuel). Does **not** touch the change
    /// address — that lives in `Config` now, and changing it means
    /// building a new `Config` (see `wallet_change_address`) and
    /// reinstantiating via `with_cache`.
    pub async fn pull_all(
        &mut self,
        reference_script_address: &Address<kind::Shelley>,
    ) -> Result<(), Error> {
        self.pull_reference_script(reference_script_address).await?;
        self.pull_network_parameters().await?;
        self.pull_channels().await?;
        Ok(())
    }

    /// Purely functional assembly from cached state — no I/O, no signing.
    fn build(&self) -> Result<Transaction<ReadyForSigning>, Error> {
        let bounds = self.config.bounds_policy().to_bounds(&time::now()?);
        let add_vkey = self.verifying_key();

        let network_parameters = self
            .cache
            .network_parameters()
            .ok_or(Error::NoNetworkParameters)?;
        let reference_script = self
            .cache
            .reference_script()
            .ok_or(Error::NoReferenceScript)?;
        let change_address = self.config.change_address().ok_or(Error::NoChangeAddress)?;
        let wallet_utxos = self.cache.wallet_utxos();
        let channels = self.cache.channels();
        let opens = self.directives.opens();
        let intents = self.directives.intents();

        if opens.is_empty() && channels.is_empty() {
            return Err(Error::NothingToDo);
        }

        // NOTE: assumes `ChannelUtxo: From<(Utxo, Channel)>` (or an
        // equivalent constructor) to rebuild a steppable `ChannelUtxo`
        // from its cached parts — verify against `konduit_tx`, the exact
        // constructor name may differ.
        let steppeds = channels
            .into_iter()
            .filter_map(|(input, (output, channel))| {
                // FIXME :: this use of ChannelUtxo is misguided.
                let u = ChannelUtxo::try_from((input.clone(), output)).unwrap();
                match u.data().stage() {
                    Stage::Opened(..) => match intents.get(&input)? {
                        Intent::Add(amount) => u.add(*amount).ok(),
                        Intent::Close => u
                            .close(&bounds.upper.expect("Must have upper bound for close"))
                            .ok(),
                    },
                    Stage::Closed(..) => bounds.lower.and_then(|lower| u.elapse(&lower).ok()),
                    Stage::Responded(_, pendings) => {
                        if pendings.is_empty() {
                            u.end(bounds.lower.as_ref()).ok()
                        } else {
                            bounds.lower.and_then(|lower| u.expire(&lower).ok())
                        }
                    }
                }
            })
            .collect::<Vec<_>>();
        let steppeds = SteppedUtxos::from(steppeds);

        let opens = opens
            .into_values()
            .map(|o| Open::new(o.amount, o.constants(add_vkey), None))
            .collect::<Vec<_>>();

        // NOTE: assumes `BTreeMap<Input, Output>: Into<Utxos>` for the
        // reverse conversion expected by `konduit_tx::tx::tx` — verify.
        let wallet_utxos: konduit_tx::Utxos = wallet_utxos.into();

        konduit_tx::tx::tx(
            &network_parameters,
            Some(&reference_script),
            change_address,
            steppeds,
            opens,
            &wallet_utxos,
        )
        .map_err(|e| Error::Tx(e.to_string()))
    }

    /// Sign a freshly `build`-ed tx: wallet always signs, plus the
    /// consumer key when distinct from the wallet and channels are being
    /// spent. Caches the result.
    pub async fn sign(&mut self) -> Result<Transaction<ReadyForSigning>, Error> {
        let channels_spent = !self.cache.channels().is_empty();
        let mut tx = self.build()?;

        // The wallet always signs: it's the party actually spending its
        // own utxos, and this crate never takes custody of that key.
        let (wallet_vk, wallet_signature) = self
            .wallet
            .sign_tx(&tx)
            .await
            .map_err(|e| Error::Wallet(e.to_string()))?;
        tx.add_witness(wallet_vk.clone(), wallet_signature);

        // consumer_key aka add_vkey signs only when required and is distinct from wallet.
        if wallet_vk == self.signer.verification_key() && channels_spent {
            let tbs = tx.id();
            let consumer_signature = self
                .signer
                .sign(tbs.as_ref())
                .await
                .map_err(|e| Error::Signing(e.to_string()))?;
            tx.add_witness(self.signer.verification_key(), consumer_signature);
        }

        self.cache.set_built_tx(tx.clone());
        Ok(tx)
    }

    /// Submit the cached tx (from the most recent `sign`) per
    /// `config().submit_policy()`.
    pub async fn submit(&self) -> Result<Hash<32>, Error> {
        let tx = self.cache.built_tx().ok_or(Error::NoStatedTx)?;

        match self.config.submit_policy() {
            SubmitPolicy::ViaConnector => {
                self.connector
                    .submit(&tx)
                    .await
                    .map_err(|e| Error::Connector(e.to_string()))?;
            }
            SubmitPolicy::ViaWallet => {
                self.wallet
                    .submit(&tx)
                    .await
                    .map_err(|e| Error::Wallet(e.to_string()))?;
            }
        }

        Ok(tx.id())
    }

    pub async fn execute(&mut self) -> Result<Hash<32>, Error> {
        self.build()?;
        self.sign().await?;
        self.submit().await
    }
}

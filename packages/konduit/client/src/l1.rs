use crate::{
    Signer, Wallet,
    core::{Credential, Input, KONDUIT_VALIDATOR},
};
use cardano_connector::CardanoConnector;
use cardano_sdk::{
    Address, Hash, NetworkId, Transaction, address::kind, transaction::state::ReadyForSigning,
};
use konduit_data::{Constants, Duration, Stage, Tag, VerifyingKey};
use konduit_tx::{
    Bounds, ChannelUtxo, NetworkParameters, Open, SteppedUtxos, Utxo, Utxos, find_reference_script,
};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, RwLock},
};

/// FIXME :: Use #[from].
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("nothing to do: no channels to open and no konduit utxos found")]
    NothingToDo,
    #[error("no reference script utxo cached: call pull_reference_script first")]
    NoReferenceScript,
    #[error("no network parameters cached: call pull_network_parameters first")]
    NoNetworkParameters,
    #[error("no change address cached: call pull_change_address first")]
    NoChangeAddress,
    #[error("no tx cached: call build first")]
    NoCachedTx,
    #[error("connector error: {0}")]
    Connector(String),
    #[error("wallet error: {0}")]
    Wallet(String),
    #[error("failed to build transaction: {0}")]
    Tx(String),
    #[error("signing error: {0}")]
    Signing(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Encode, Decode)]
pub enum SubmitPolicy {
    #[n(0)]
    ViaConnector,
    #[n(1)]
    ViaWallet,
}

impl Default for SubmitPolicy {
    fn default() -> Self {
        SubmitPolicy::ViaConnector
    }
}

/// How far into the future the transaction's upper validity bound is set,
/// anchored to the moment `build` runs. Defaults to 20 minutes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Encode, Decode)]
pub struct BoundsPolicy {
    #[n(0)]
    window: Duration,
}

impl BoundsPolicy {
    pub fn new(window: Duration) -> Self {
        Self { window }
    }
    pub fn window(&self) -> Duration {
        self.window
    }
    pub fn set_window(&mut self, window: Duration) {
        self.window = window;
    }

    // ASSUMPTION: `Bounds` exposes a general "upper bound `window` from
    // now, no lower bound" constructor under some name — this mirrors
    // whatever `Bounds::twenty_mins()` did internally. Swap for the real
    // API name.
    fn to_bounds(self) -> Bounds {
        Bounds::upper_in(self.window)
    }
}

impl Default for BoundsPolicy {
    fn default() -> Self {
        Self {
            window: Duration::from_secs(20 * 60),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Encode, Decode)]
struct Policies {
    #[n(0)]
    submit: SubmitPolicy,
    #[n(1)]
    bounds: BoundsPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct OpenIntent {
    #[n(0)]
    pub tag: Tag,
    #[n(1)]
    pub sub_vkey: VerifyingKey,
    #[n(2)]
    pub close_period: Duration,
    #[n(3)]
    pub amount: u64,
}

impl OpenIntent {
    fn constants(self, add_vkey: VerifyingKey) -> Constants {
        Constants {
            tag: self.tag,
            add_vkey,
            sub_vkey: self.sub_vkey,
            close_period: self.close_period,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub enum Intent {
    #[n(0)]
    Add(#[n(0)] u64),
    #[n(1)]
    Close,
}

/// Data pulled from chain/wallet, plus pending intent. Read together as
/// one snapshot in `build`. Staleness is the caller's responsibility:
/// call the `pull_*` methods (or `pull`) as often as your staleness
/// tolerance requires.
///
/// NOTE: most fields are runtime cache, re-populated by `pull_*` — confirm
/// every field type actually supports encode/decode before relying on
/// (de)serializing this whole struct (`Transaction<ReadyForSigning>` in
/// particular may not implement these derives; if not, store its raw CBOR
/// bytes instead and reconstruct on read).
#[derive(Debug, Clone, Default, Serialize, Deserialize, Encode, Decode)]
struct Cache {
    #[n(0)]
    network_parameters: Option<NetworkParameters>,
    #[n(1)]
    utxo_script_ref: Option<Utxo>,
    #[n(2)]
    delegations: Vec<Credential>,
    #[n(3)]
    channels: Vec<ChannelUtxo>,
    #[n(4)]
    utxos_wallet: Utxos,
    #[n(5)]
    opens: Vec<OpenIntent>,
    #[n(6)]
    intents: BTreeMap<Input, Intent>,
    #[n(7)]
    change_address: Option<Address<kind::Any>>,
    #[n(8)]
    tx: Option<Transaction<ReadyForSigning>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Encode, Decode)]
pub struct L1State {
    #[n(0)]
    cache: Cache,
    #[n(1)]
    policies: Policies,
}

pub struct L1<Connector: CardanoConnector, S: Signer, W: Wallet> {
    connector: Arc<Connector>,
    signer: Arc<S>,
    wallet: Arc<W>,
    state: RwLock<L1State>,
}

impl<Connector, S, W> L1<Connector, S, W>
where
    Connector: CardanoConnector,
    S: Signer,
    W: Wallet,
{
    pub fn new(connector: Arc<Connector>, signer: Arc<S>, wallet: Arc<W>) -> Self {
        L1 {
            connector,
            signer,
            wallet,
            state: RwLock::new(L1State::default()),
        }
    }

    pub fn from_state(
        connector: Arc<Connector>,
        signer: Arc<S>,
        wallet: Arc<W>,
        state: L1State,
    ) -> Self {
        L1 {
            connector,
            signer,
            wallet,
            state: RwLock::new(state),
        }
    }

    fn read_state<T>(&self, f: impl FnOnce(&L1State) -> T) -> T {
        f(&self.state.read().unwrap_or_else(|p| p.into_inner()))
    }

    fn write_state(&self, f: impl FnOnce(&mut L1State)) {
        f(&mut self.state.write().unwrap_or_else(|p| p.into_inner()));
    }

    pub fn state(&self) -> L1State {
        self.read_state(Clone::clone)
    }

    // add_vkey
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signer.verification_key().into_bytes().into()
    }

    // -- Policies --

    pub fn submit_policy(&self) -> SubmitPolicy {
        self.read_state(|s| s.policies.submit)
    }
    pub fn set_submit_policy(&self, policy: SubmitPolicy) {
        self.write_state(|s| s.policies.submit = policy);
    }
    pub fn bounds_policy(&self) -> BoundsPolicy {
        self.read_state(|s| s.policies.bounds)
    }
    pub fn set_bounds_policy(&self, policy: BoundsPolicy) {
        self.write_state(|s| s.policies.bounds = policy);
    }

    // -- Delegations, mutated piecewise --

    pub fn add_delegation(&self, credential: Credential) {
        self.write_state(|s| s.cache.delegations.push(credential));
    }
    pub fn remove_delegation(&self, credential: &Credential) {
        self.write_state(|s| s.cache.delegations.retain(|c| c != credential));
    }
    pub fn delegations(&self) -> Vec<Credential> {
        self.read_state(|s| s.cache.delegations.clone())
    }

    // -- Intent, mutated piecewise --

    pub fn add_intent(&self, input: Input, intent: Intent) {
        self.write_state(|s| {
            s.cache.intents.insert(input, intent);
        });
    }

    /// Resolve `tag` against currently cached channels and set `intent`
    /// for the matching input(s). Reusing a tag across multiple channels
    /// is strongly discouraged but not an error: if more than one channel
    /// matches, `intent` is applied to all of them and a warning is logged.
    pub fn add_intent_by_tag(&self, tag: &Tag, intent: Intent) {
        let inputs: Vec<Input> = self.read_state(|s| {
            s.cache
                .channels
                .iter()
                .filter(|u| u.data().constants().tag == *tag)
                .map(|u| u.utxo().0.clone())
                .collect()
        });

        if inputs.len() > 1 {
            log::warn!(
                "tag {tag:?} matches {} channels; applying intent to all of them — \
                 reusing a tag across channels is strongly discouraged",
                inputs.len()
            );
        }

        self.write_state(|s| {
            for input in &inputs {
                s.cache.intents.insert(input.clone(), intent.clone());
            }
        });
    }

    pub fn remove_intent(&self, input: &Input) {
        self.write_state(|s| {
            s.cache.intents.remove(input);
        });
    }
    pub fn clear_intents(&self) {
        self.write_state(|s| s.cache.intents.clear());
    }
    pub fn add_open(&self, open: OpenIntent) {
        self.write_state(|s| s.cache.opens.push(open));
    }
    pub fn clear_opens(&self) {
        self.write_state(|s| s.cache.opens.clear());
    }

    // -- Change address: cached, settable directly or pulled from the wallet --

    pub fn change_address(&self) -> Option<Address<kind::Any>> {
        self.read_state(|s| s.cache.change_address.clone())
    }

    /// Set the change address directly, bypassing the wallet. Overwritten
    /// the next time `pull_change_address` or `pull` runs.
    pub fn set_change_address(&self, address: Address<kind::Any>) {
        self.write_state(|s| s.cache.change_address = Some(address));
    }

    /// Pull the wallet's preferred change address (CIP-30
    /// `getChangeAddress`) and cache it, overwriting whatever was there.
    pub async fn pull_change_address(&self) -> Result<(), Error> {
        let change_address = self
            .wallet
            .change_address()
            .await
            .map_err(|e| Error::Wallet(e.to_string()))?;
        self.write_state(|s| s.cache.change_address = Some(change_address));
        Ok(())
    }

    // -- Chain state, pulled explicitly, each on its own cadence --

    /// Pull the (singular) konduit reference script utxo. Changes only
    /// when the script deployment moves.
    pub async fn pull_reference_script(
        &self,
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
        let utxo_script_ref = find_reference_script(&utxos_at_script).cloned();
        self.write_state(|s| s.cache.utxo_script_ref = utxo_script_ref);
        Ok(())
    }

    /// Pull network parameters. Changes only on hard forks / param
    /// updates — call on its own, much coarser cadence than channel pulls.
    pub async fn pull_network_parameters(&self) -> Result<(), Error> {
        let network_parameters = NetworkParameters {
            network_id: NetworkId::from(self.connector.network()),
            protocol_parameters: self
                .connector
                .protocol_parameters()
                .await
                .map_err(|e| Error::Connector(e.to_string()))?,
        };
        self.write_state(|s| s.cache.network_parameters = Some(network_parameters));
        Ok(())
    }

    fn network_parameters(&self) -> Result<NetworkParameters, Error> {
        self.read_state(|s| s.cache.network_parameters.clone())
            .ok_or(Error::NoNetworkParameters)
    }

    /// Pull konduit channel utxos across the base credential and every
    /// currently-registered delegation, parsing each into a `ChannelUtxo`
    /// and keeping only those matching this consumer's `add_vkey`. Also
    /// pulls wallet fuel utxos, since both feed the same tx-building step
    /// and share this cadence. Any pending intent whose channel no
    /// longer exists is dropped; the rest are kept.
    pub async fn pull_channels(&self) -> Result<(), Error> {
        let delegations = self.read_state(|s| s.cache.delegations.clone());
        let add_vkey = self.verifying_key();

        let payment_credential = KONDUIT_VALIDATOR.to_credential();
        let mut pairs = vec![(payment_credential.clone(), None)];
        pairs.extend(
            delegations
                .into_iter()
                .map(|d| (payment_credential.clone(), Some(d))),
        );

        let utxos_konduit = utxo_batch(&*self.connector, &pairs)
            .await
            .map_err(|e| Error::Connector(e.to_string()))?;

        let channels: Vec<ChannelUtxo> = utxos_konduit
            .into_iter()
            .filter_map(|u| ChannelUtxo::try_from(u).ok())
            .filter(|u| u.data().constants().add_vkey == add_vkey)
            .collect();

        let live_inputs: BTreeSet<Input> = channels.iter().map(|u| u.utxo().0.clone()).collect();

        let utxos_wallet = self
            .wallet
            .utxos(None)
            .await
            .map_err(|e| Error::Wallet(e.to_string()))?
            .unwrap_or_default();

        self.write_state(|s| {
            s.cache.channels = channels;
            s.cache.utxos_wallet = utxos_wallet;
            s.cache
                .intents
                .retain(|input, _| live_inputs.contains(input));
        });
        Ok(())
    }

    /// Pull everything: reference script, network parameters, channels
    /// (+ fuel), and the change address. Note: this **overwrites** any
    /// change address set via `set_change_address` with whatever the
    /// wallet currently reports.
    pub async fn pull_all(
        &self,
        reference_script_address: &Address<kind::Shelley>,
    ) -> Result<(), Error> {
        self.pull_reference_script(reference_script_address).await?;
        self.pull_network_parameters().await?;
        self.pull_channels().await?;
        self.pull_change_address().await?;
        Ok(())
    }

    /// Currently cached channels belonging to this consumer.
    pub fn channels(&self) -> Vec<ChannelUtxo> {
        self.read_state(|s| s.cache.channels.clone())
    }

    /// Currently cached, most recently built tx, if any.
    pub fn cached_tx(&self) -> Option<Transaction<ReadyForSigning>> {
        self.read_state(|s| s.cache.tx.clone())
    }

    /// Purely functional assembly from cached state — no I/O, no signing.
    fn build(&self) -> Result<Transaction<ReadyForSigning>, Error> {
        let network_parameters = self.network_parameters()?;
        let add_vkey = self.verifying_key();
        let bounds = self.bounds_policy().to_bounds();

        let (utxo_script_ref, change_address, channels, utxos_wallet, opens, intents) = self
            .read_state(|s| {
                (
                    s.cache.utxo_script_ref.clone(),
                    s.cache.change_address.clone(),
                    s.cache.channels.clone(),
                    s.cache.utxos_wallet.clone(),
                    s.cache.opens.clone(),
                    s.cache.intents.clone(),
                )
            });
        let utxo_script_ref = utxo_script_ref.ok_or(Error::NoReferenceScript)?;
        let change_address = change_address.ok_or(Error::NoChangeAddress)?;

        if opens.is_empty() && channels.is_empty() {
            return Err(Error::NothingToDo);
        }

        let steppeds = channels
            .into_iter()
            .filter_map(|u| {
                let input = u.utxo().0.clone();
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
            .into_iter()
            .map(|o| Open::new(o.amount, o.constants(add_vkey), None))
            .collect::<Vec<_>>();

        konduit_tx::tx::tx(
            &network_parameters,
            Some(&utxo_script_ref),
            change_address,
            steppeds,
            opens,
            &utxos_wallet,
        )
        .map_err(|e| Error::Tx(e.to_string()))
    }

    /// Sign a freshly `build`-ed tx: wallet always signs, plus the
    /// consumer key when distinct from the wallet and channels are being
    /// spent. Caches the result.
    pub async fn sign(&self) -> Result<Transaction<ReadyForSigning>, Error> {
        let channels_spent = !self.read_state(|s| s.cache.channels.is_empty());
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

        self.write_state(|s| s.cache.tx = Some(tx.clone()));
        Ok(tx)
    }

    /// Submit the cached tx (from the most recent `sign`) per `submit_policy`.
    pub async fn submit(&self) -> Result<Hash<32>, Error> {
        let tx = self.cached_tx().ok_or(Error::NoCachedTx)?;

        match self.submit_policy() {
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

    pub async fn execute(&self) -> Result<Hash<32>, Error> {
        self.build()?;
        self.sign().await?;
        self.submit().await
    }
}

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
use std::sync::{Arc, RwLock};

use crate::{
    Signer, Wallet,
    core::{Credential, Input, KONDUIT_VALIDATOR},
    time, utxo_batch,
};

mod config;
pub use config::{BoundsPolicy, Config, SubmitPolicy};

mod directives;
pub use directives::{Directives, Intent, OpenIntent};

mod cache;
pub use cache::Cache;

mod state;
pub use state::State;

/// FIXME :: Use #[from].
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("time: {0}")]
    Time(#[from] time::Error),
    #[error("nothing to do: no channels to open and no konduit utxos found")]
    NothingToDo,
    #[error("no reference script utxo cached: call pull_reference_script first")]
    NoReferenceScript,
    #[error("no network parameters cached: call pull_network_parameters first")]
    NoNetworkParameters,
    #[error("no change address cached: call pull_change_address first")]
    NoChangeAddress,
    #[error("no tx cached: call build first")]
    NoStatedTx,
    #[error("connector error: {0}")]
    Connector(String),
    #[error("wallet error: {0}")]
    Wallet(String),
    #[error("failed to build transaction: {0}")]
    Tx(String),
    #[error("signing error: {0}")]
    Signing(String),
}

pub struct L1<Connector: CardanoConnector, S: Signer, W: Wallet> {
    connector: Arc<Connector>,
    signer: Arc<S>,
    wallet: Arc<W>,
    state: RwLock<State>,
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
            state: RwLock::new(State::default()),
        }
    }

    pub fn from_state(
        connector: Arc<Connector>,
        signer: Arc<S>,
        wallet: Arc<W>,
        state: State,
    ) -> Self {
        L1 {
            connector,
            signer,
            wallet,
            state: RwLock::new(state),
        }
    }

    fn read_state<T>(&self, f: impl FnOnce(&State) -> T) -> T {
        f(&self.state.read().unwrap_or_else(|p| p.into_inner()))
    }

    fn write_state<T>(&self, f: impl FnOnce(&mut State) -> T) -> T {
        f(&mut self.state.write().unwrap_or_else(|p| p.into_inner()))
    }

    pub fn state(&self) -> State {
        self.read_state(Clone::clone)
    }

    // add_vkey
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signer.verification_key().into_bytes().into()
    }

    // -- Policies --

    pub fn submit_policy(&self) -> SubmitPolicy {
        self.read_state(State::submit_policy)
    }
    pub fn set_submit_policy(&self, policy: SubmitPolicy) {
        self.write_state(|c| c.set_submit_policy(policy));
    }
    pub fn bounds_policy(&self) -> BoundsPolicy {
        self.read_state(State::bounds_policy)
    }
    pub fn set_bounds_policy(&self, policy: BoundsPolicy) {
        self.write_state(|c| c.set_bounds_policy(policy));
    }
    pub fn autocomplete(&self) -> bool {
        self.read_state(State::autocomplete)
    }
    pub fn set_autocomplete(&self, autocomplete: bool) {
        self.write_state(|c| c.set_autocomplete(autocomplete));
    }

    // -- Delegations, mutated piecewise --

    pub fn add_delegation(&self, credential: Credential) {
        self.write_state(|c| c.add_delegation(credential));
    }
    pub fn remove_delegation(&self, credential: &Credential) {
        self.write_state(|c| c.remove_delegation(credential));
    }
    pub fn delegations(&self) -> Vec<Credential> {
        self.read_state(State::delegations)
    }

    // -- Intent, mutated piecewise --

    pub fn add_intent(&self, input: Input, intent: Intent) {
        self.write_state(|c| c.add_intent(input, intent));
    }

    /// Resolve `tag` against currently cached channels and set `intent`
    /// for the matching input(s). Reusing a tag across multiple channels
    /// is strongly discouraged but not an error: if more than one channel
    /// matches, `intent` is applied to all of them and a warning is logged.
    pub fn add_intent_by_tag(&self, tag: &Tag, intent: Intent) {
        let matched = self.write_state(|c| c.add_intent_by_tag(tag, intent));
        if matched > 1 {
            log::warn!(
                "tag {tag:?} matches {matched} channels; applying intent to all of them — \
                 reusing a tag across channels is strongly discouraged",
            );
        }
    }

    pub fn remove_intent(&self, input: &Input) {
        self.write_state(|c| c.remove_intent(input));
    }
    pub fn clear_intents(&self) {
        self.write_state(State::clear_intents);
    }

    // -- Force: manual expire/elapse/end overrides. Only consulted when
    // `autocomplete` is `false` — ignored entirely when it's `true`.
    // FIXME: not yet consulted in `build` at all — see `State::autocomplete`.

    pub fn force(&self) -> BTreeSet<Input> {
        self.read_state(State::force)
    }
    pub fn add_force(&self, input: Input) {
        self.write_state(|c| c.add_force(input));
    }
    pub fn remove_force(&self, input: &Input) {
        self.write_state(|c| c.remove_force(input));
    }
    pub fn clear_force(&self) {
        self.write_state(State::clear_force);
    }

    // -- Opens, keyed by tag, mutated piecewise --

    pub fn add_open(&self, open: OpenIntent) {
        self.write_state(|c| c.add_open(open));
    }
    pub fn remove_open(&self, tag: &Tag) {
        self.write_state(|c| c.remove_open(tag));
    }
    pub fn clear_opens(&self) {
        self.write_state(State::clear_opens);
    }
    pub fn opens(&self) -> BTreeMap<Tag, OpenIntent> {
        self.read_state(State::opens)
    }

    // -- Change address: cached, settable directly or pulled from the wallet --

    pub fn change_address(&self) -> Option<Address<kind::Any>> {
        self.read_state(State::change_address)
    }

    /// Set the change address directly, bypassing the wallet. Overwritten
    /// the next time `pull_change_address` or `pull_all` runs.
    pub fn set_change_address(&self, address: Address<kind::Any>) {
        self.write_state(|c| c.set_change_address(address));
    }

    /// Pull the wallet's preferred change address (CIP-30
    /// `getChangeAddress`) and cache it, overwriting whatever was there.
    pub async fn pull_change_address(&self) -> Result<(), Error> {
        let change_address = self
            .wallet
            .change_address()
            .await
            .map_err(|e| Error::Wallet(e.to_string()))?;
        self.write_state(|c| c.set_change_address(change_address));
        Ok(())
    }

    // -- Chain state, pulled explicitly, each on its own cadence --

    /// Pull the (singular) konduit reference script utxo. Changes rarely.
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
        let reference_script =
            find_reference_script(&utxos_at_script).ok_or(Error::NoReferenceScript)?;
        self.write_state(|c| c.set_reference_script(reference_script));
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
        self.write_state(|c| c.set_network_parameters(network_parameters));
        Ok(())
    }

    /// Pull konduit channel utxos across the base credential and every
    /// currently-registered delegation, parsing each into a `ChannelUtxo`
    /// and keeping only those matching this consumer's `add_vkey`. Also
    /// pulls wallet fuel utxos, since both feed the same tx-building step
    /// and share this cadence. Any pending intent whose channel no
    /// longer exists is dropped by `State::set_tip`; the rest are kept.
    pub async fn pull_channels(&self) -> Result<(), Error> {
        let delegations = self.read_state(State::delegations);
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

        self.write_state(|c| c.set_tip(wallet_utxos, channels));
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
    pub fn channels(&self) -> BTreeMap<Input, (Output, Channel)> {
        self.read_state(State::channels)
    }

    /// Currently cached, most recently built tx, if any.
    pub fn cached_tx(&self) -> Option<Transaction<ReadyForSigning>> {
        self.read_state(State::built_tx)
    }

    /// Purely functional assembly from cached state — no I/O, no signing.
    fn build(&self) -> Result<Transaction<ReadyForSigning>, Error> {
        let bounds = self.bounds_policy().to_bounds(&time::now()?);
        let add_vkey = self.verifying_key();

        let state::BuildInputs {
            network_parameters,
            reference_script,
            change_address,
            wallet_utxos,
            channels,
            opens,
            intents,
        } = self.read_state(State::build_inputs);

        let network_parameters = network_parameters.ok_or(Error::NoNetworkParameters)?;
        let reference_script = reference_script.ok_or(Error::NoReferenceScript)?;
        let change_address = change_address.ok_or(Error::NoChangeAddress)?;

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
    /// spent. States the result.
    pub async fn sign(&self) -> Result<Transaction<ReadyForSigning>, Error> {
        let channels_spent = !self.read_state(State::channels).is_empty();
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

        self.write_state(|c| c.set_built_tx(tx.clone()));
        Ok(tx)
    }

    /// Submit the cached tx (from the most recent `sign`) per `submit_policy`.
    pub async fn submit(&self) -> Result<Hash<32>, Error> {
        let tx = self.cached_tx().ok_or(Error::NoStatedTx)?;

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

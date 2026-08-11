use std::collections::BTreeMap;
use std::iter;

use cardano_connector_direct::Blockfrost;
use cardano_sdk::VerificationKey;
use cardano_sdk::{
    Address, Hash, Input, NetworkId, Output, ProtocolParameters, Transaction, address::kind,
    transaction::state::ReadyForSigning,
};

use cardano_connector::CardanoConnector;
use konduit_data::{Duration, Secret, Stage, Tag, VerifyingKey};
use konduit_tmp::Receipt;
use konduit_tx::{
    Bounds, ChannelUtxo, KONDUIT_VALIDATOR, NetworkParameters, Open, SteppedUtxo, SteppedUtxos,
    find_reference_script,
};

use crate::config::{AccountConfig, AdaptorConfig, Config, WalletConfig};
use crate::strategy::Strategy;

#[derive(Debug)]
pub struct WaitConfig {
    poll: Duration,
    max_attempts: u8,
}

impl Default for WaitConfig {
    fn default() -> Self {
        Self {
            poll: Duration::from_secs(10),
            max_attempts: 10,
        }
    }
}

struct Runner<C> {
    config: Config,
    keys: BTreeMap<VerificationKey, cardano_sdk::SigningKey>,
    cardano: C,
    network_parameters: NetworkParameters,
    ref_script: Option<(Input, Output)>,
    fuel: BTreeMap<Input, Output>,
    channels: Vec<ChannelUtxo>,
}

impl Runner<Blockfrost> {
    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn wallet(&self) -> &WalletConfig {
        &self.config().wallet
    }

    pub fn adaptor(&self) -> &AdaptorConfig {
        &self.config().adaptor
    }

    pub fn accounts(&self) -> &Vec<AccountConfig> {
        &self.config().accounts
    }

    pub fn host_address(&self) -> Address<kind::Any> {
        self.wallet()
            .address(self.network_parameters.network_id)
            .into()
    }

    pub fn change_address(&self) -> Address<kind::Any> {
        self.wallet()
            .address(self.network_parameters.network_id)
            .into()
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

    fn signing_key(&self, vk: &VerificationKey) -> anyhow::Result<&cardano_sdk::SigningKey> {
        self.keys
            .get(vk)
            .ok_or(anyhow::anyhow!("Key not recognized!"))
    }

    pub async fn build(config: Config) -> anyhow::Result<Self> {
        let mut x = Self::init(config).await?;
        x.reload_wallet().await?;
        x.reload_channels().await?;
        Ok(x)
    }

    pub async fn init(config: Config) -> anyhow::Result<Self> {
        let cardano = config.cardano.build();
        let network_id: NetworkId = cardano.network().into();
        let protocol_parameters = cardano.protocol_parameters().await?;
        let network_parameters = NetworkParameters {
            network_id,
            protocol_parameters,
        };
        let keys = iter::once(config.adaptor.cardano_signing_key())
            .chain(
                config
                    .accounts
                    .iter()
                    .map(AccountConfig::cardano_signing_key),
            )
            .map(|sk| (sk.to_verification_key(), sk))
            .collect();
        Ok(Self {
            config,
            cardano,
            keys,
            network_parameters,
            ref_script: Default::default(),
            channels: Default::default(),
            fuel: Default::default(),
        })
    }

    pub async fn reload_wallet(&mut self) -> anyhow::Result<()> {
        let wallet_utxos = self
            .cardano
            .utxos_at(&self.wallet().credential(), None)
            .await?;
        let ref_script = find_reference_script(&wallet_utxos);
        self.ref_script = ref_script;
        let fuel = wallet_utxos
            .into_iter()
            .filter(|utxo| utxo.1.script().is_none())
            .collect();
        self.fuel = fuel;
        Ok(())
    }

    pub async fn reload_channels(&mut self) -> anyhow::Result<()> {
        let utxos = self
            .cardano
            .utxos_at(&KONDUIT_VALIDATOR.to_credential(), None)
            .await?;
        // sub_vkey is baked into every channel's constants as *this* adaptor's
        // own key at open time (see `Staging::apply_action`), so it's
        // invariant across every channel we should ever touch. Filter it
        // here, once, rather than re-deriving "is this ours" per intent
        // later - any UTXO at the validator address with a different
        // sub_vkey isn't a channel we run.
        let adaptor_vkey = self.adaptor().constants().0;
        let channels = utxos
            .into_iter()
            .filter_map(|io| ChannelUtxo::try_from(io).ok())
            .filter(|c| c.data().constants().sub_vkey == adaptor_vkey)
            .collect();
        self.channels = channels;
        Ok(())
    }

    pub async fn sign_and_submit(
        &mut self,
        mut tx: Transaction<ReadyForSigning>,
    ) -> anyhow::Result<Hash<32>> {
        tx.sign(&self.wallet().cardano_signing_key());
        let id = tx.id();
        self.cardano.submit(&tx).await?;
        Ok(id)
    }

    pub fn deploy(&mut self) -> anyhow::Result<Transaction<ReadyForSigning>> {
        konduit_tx::admin::deploy(
            &self.protocol_parameters(),
            &self.fuel,
            KONDUIT_VALIDATOR.script.clone(),
            self.host_address(),
            self.change_address(),
        )
    }

    fn tx(
        &mut self,
        opens: Vec<Open>,
        steppeds: Vec<SteppedUtxo>,
    ) -> anyhow::Result<Transaction<ReadyForSigning>> {
        let steppeds = SteppedUtxos::from(steppeds);
        let signers = steppeds.signers();
        let mut tx = konduit_tx::tx::tx(
            &self.network_parameters,
            self.ref_script.as_ref(),
            self.change_address(),
            steppeds,
            opens,
            &self.fuel,
        )?;
        for vk in signers.iter() {
            tx.sign(self.signing_key(vk)?);
        }
        Ok(tx)
    }

    pub async fn wait_til(&mut self, id: &Hash<32>) -> anyhow::Result<()> {
        self.wait_til_with(id, WaitConfig::default()).await
    }

    /// Waits for `id` to land (via `self.fuel`, the *wallet's* UTXOs), then
    /// refreshes `self.channels` before returning.
    ///
    /// That refresh matters more than it looks: Blockfrost indexes
    /// different addresses with slightly different latency, so the wallet
    /// address can show a confirmation a beat before the validator address
    /// does. Doing the channel reload here, right as confirmation is
    /// detected, is strictly better than a separate call afterward (the
    /// gap between the two used to be exactly wide enough for a caller to
    /// build a tx against channel UTXOs that had already been spent) - but
    /// it's still a single snapshot, not a verified-fresh one, so this
    /// narrows the race rather than closing it outright.
    pub async fn wait_til_with(&mut self, id: &Hash<32>, cfg: WaitConfig) -> anyhow::Result<()> {
        for attempt in 0..cfg.max_attempts {
            tracing::warn!("awaiting {} attempt {}", id, attempt);
            if self.fuel.keys().any(|i| i.transaction_id() == *id) {
                self.reload_channels().await?;
                return Ok(());
            }

            if attempt + 1 == cfg.max_attempts {
                break;
            }

            tokio::time::sleep(*cfg.poll).await;
            self.reload_wallet().await?;
        }

        Err(anyhow::anyhow!(
            "timed out waiting for transaction {id:?} after {} attempts",
            cfg.max_attempts
        ))
    }

    pub fn stage_tx(&mut self, bounds: Bounds) -> Staging<'_> {
        let channels = self.channels.clone();
        Staging {
            runner: self,
            bounds,
            channels,
            steppeds: Vec::new(),
            opens: Vec::new(),
        }
    }
}

pub struct Staging<'r> {
    runner: &'r mut Runner<Blockfrost>,
    bounds: Bounds,
    channels: Vec<ChannelUtxo>,
    steppeds: Vec<SteppedUtxo>,
    opens: Vec<Open>,
}

impl<'r> Staging<'r> {
    pub fn accounts(&self) -> &[AccountConfig] {
        &self.runner.config.accounts
    }

    pub fn has_channel(&self, account: &VerifyingKey) -> bool {
        self.channels
            .iter()
            .any(|c| c.data().constants().add_vkey == *account)
    }

    pub fn stage(&self, account: &VerifyingKey) -> Option<Stage> {
        self.channels
            .iter()
            .find(|c| c.data().constants().add_vkey == *account)
            .map(|c| c.data().stage().clone())
    }

    pub fn apply_action(&mut self, account: &AccountConfig, action: Action) -> anyhow::Result<()> {
        if let Action::Open { amount } = action {
            let (adaptor_vkey, adaptor_duration) = self.runner.adaptor().constants();
            let constants = account.constants(adaptor_vkey, adaptor_duration);
            self.opens.push(Open::new(amount, constants, None));
            return Ok(());
        }

        self.step_channel(&account.verifying_key(), &account.tag, action)
    }

    /// Like `apply_action`, but treats "this account's channel isn't
    /// currently eligible for `action`" (no channel at all, wrong stage, or
    /// its lower/upper bound isn't met yet) as an ordinary non-event
    /// (`Ok(false)`) instead of an error. Meant for "unambiguous" actions
    /// (`Elapse`/`Expire`/`End`) a strategy wants to opportunistically probe
    /// across every channel each round, without duplicating the eligibility
    /// rules `channel.rs` already encodes internally.
    ///
    /// NOTE: this can't currently distinguish "not eligible yet" from "no
    /// channel for this account" from a genuine unexpected failure - all
    /// three collapse to `Ok(false)`. Fine for an exploratory strategy;
    /// worth tightening (matching on the underlying `StepError`) if silent
    /// failures ever need to be surfaced instead.
    pub fn try_apply_action(
        &mut self,
        account: &AccountConfig,
        action: Action,
    ) -> anyhow::Result<bool> {
        match self.apply_action(account, action) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// True if nothing has been applied to this tx-in-progress yet - the
    /// caller (see `tx::run`) should read this as "nothing to commit".
    pub fn is_empty(&self) -> bool {
        self.opens.is_empty() && self.steppeds.is_empty()
    }

    pub fn opens_len(&self) -> usize {
        self.opens.len()
    }

    pub fn steppeds_len(&self) -> usize {
        self.steppeds.len()
    }

    fn step_channel(
        &mut self,
        account: &VerifyingKey,
        tag: &Tag,
        action: Action,
    ) -> anyhow::Result<()> {
        // Resolve whichever bound `action` needs *before* touching
        // `self.channels`, so a missing bound errors out cleanly without
        // needing to undo a channel removal.
        match &action {
            Action::Claim { .. } | Action::Close | Action::Unlock { .. }
                if self.bounds.upper.is_none() =>
            {
                return Err(anyhow::anyhow!(
                    "this action needs an upper bound, but none was set for this tx"
                ));
            }
            Action::Elapse | Action::Expire if self.bounds.lower.is_none() => {
                return Err(anyhow::anyhow!(
                    "this action needs a lower bound, but none was set for this tx"
                ));
            }
            _ => {}
        }

        let idx = self
            .channels
            .iter()
            .position(|c| {
                c.data().constants().add_vkey == *account && c.data().constants().tag == *tag
            })
            .ok_or_else(|| anyhow::anyhow!("no open channel for account {account:?}"))?;
        let channel = self.channels.remove(idx);

        let result = match action {
            Action::Add { amount } => channel.add(amount),
            Action::Claim { receipt } => channel.any_claim(&receipt, &self.bounds.upper.unwrap()),
            Action::Close => channel.close(&self.bounds.upper.unwrap()),
            Action::Elapse => channel.elapse(&self.bounds.lower.unwrap()),
            Action::Expire => channel.expire(&self.bounds.lower.unwrap()),
            Action::End => channel.end(self.bounds.lower.as_ref()),
            Action::Unlock { secrets } => {
                channel.unlock_with_secrets(secrets, &self.bounds.upper.unwrap())
            }
            Action::Open { .. } => {
                unreachable!("apply_action resolves Open before calling step_channel")
            }
        };

        match result {
            Ok(utxo_and_data) => {
                self.steppeds.push(utxo_and_data.into());
                Ok(())
            }
            Err((boxed_utxo_and_data, err)) => {
                self.channels
                    .insert(idx, ChannelUtxo::from(*boxed_utxo_and_data));
                Err(anyhow::anyhow!("{err}"))
            }
        }
    }

    pub fn commit(self) -> anyhow::Result<Transaction<ReadyForSigning>> {
        self.runner.tx(self.opens, self.steppeds)
    }
}

pub async fn run(config: Config, steps: u32, strategy: &mut impl Strategy) -> anyhow::Result<()> {
    let mut txer = Runner::build(config).await?;

    for i in 0..steps {
        // Blockfrost indexes different addresses with different latency, so
        // even a freshly-`reload_channels`'d `Runner` can still be a beat
        // stale (see `wait_til_with`'s docs) - a submission built against
        // that staleness fails with a Cardano-side validation error (bad/
        // already-spent inputs), not a Rust error, so it's only visible
        // here as `sign_and_submit` returning `Err`. On that failure:
        // pause, re-read channels, and rebuild the round from scratch
        // (including a fresh `Bounds::five_mins()` - no reason to reuse an
        // increasingly-stale window across retries) rather than
        // resubmitting the same (now provably stale) tx.
        //
        // NOTE: this retries *any* submission failure, not just staleness -
        // there's no attempt to distinguish "the chain caught up, try
        // again" from "this tx is genuinely invalid and always will be".
        // Fine for now; worth inspecting the actual Cardano error body if
        // repeated non-transient failures ever burn through retries for no
        // reason.
        //
        // NOTE: `strategy.choose` re-runs on every retry, which re-evolves
        // any RNG-backed state a strategy owns (e.g. `StepStrategy`'s
        // receipt sessions) an extra step per retry, not just once per
        // round. Harmless here, but worth knowing if that state's exact
        // evolution count ever needs to be relied on.
        const MAX_ATTEMPTS: u32 = 3;
        let mut attempt = 0;

        loop {
            // TODO: assuming `Bounds::five_mins()` already gives a proper
            // [now, now + 5min] window (both bounds set), not just an upper
            // - `Elapse`/`Expire`/`End` now genuinely need `lower` too, not
            // just `Claim`/`Close`'s `upper`. If it only sets `upper`, this
            // needs a real way to get "now" as a `Duration` to fill in
            // `lower` as well.
            let bounds = Bounds::five_mins();

            let mut staging = txer.stage_tx(bounds);
            strategy.choose(&mut staging)?;

            if staging.is_empty() {
                tracing::info!("round {i}: nothing to do, skipping");
                break;
            }

            tracing::info!(
                "round {i}: committing ({} opens, {} steps)",
                staging.opens_len(),
                staging.steppeds_len()
            );
            let tx = staging.commit()?;

            match txer.sign_and_submit(tx).await {
                Ok(id) => {
                    txer.wait_til(&id).await?;
                    break;
                }
                Err(err) => {
                    attempt += 1;
                    if attempt >= MAX_ATTEMPTS {
                        return Err(err.context(format!(
                            "round {i}: submission failed after {attempt} attempts"
                        )));
                    }
                    tracing::warn!(
                        "round {i}: submission failed (attempt {attempt}/{MAX_ATTEMPTS}), \
                         pausing and rebuilding against fresh channel state: {err}"
                    );
                    tokio::time::sleep(*Duration::from_secs(10)).await;
                    txer.reload_channels().await?;
                }
            }
        }
    }

    Ok(())
}

/// Declares what should happen to one account's channel as part of a tx.
///
/// `Open`/`Add`/`Close` are add_vkey (consumer)-authorized. `Claim` is
/// sub_vkey (adaptor)-authorized and dispatches to whichever of the
/// channel's stage-specific claim steps (sub/respond/unlock) currently
/// applies (see `Channel::any_claim`) - the adaptor doesn't need to track
/// which stage-specific call is legal. `Unlock` is also sub_vkey-authorized,
/// for the narrower case of unlocking specific known secrets directly
/// (rather than via a whole `Receipt`).
///
/// `Elapse`/`Expire`/`End` are the "unambiguous" add_vkey actions: pure
/// recovery of funds that are already rightfully the consumer's, with no
/// strategic tradeoff about *whether* to take them - only about whether
/// they're legal *yet*, which `channel.rs`'s own stage/time checks already
/// answer (see `Staging::try_apply_action`, which surfaces "not eligible"
/// as a plain skip rather than an error). `Unlock` is the sub_vkey
/// equivalent: unambiguous once the adaptor actually knows a secret, though
/// nothing here sources secrets - that's up to whatever calls `apply_action`.
pub enum Action {
    /// No bound needed.
    Open { amount: u64 },
    /// No bound needed.
    Add { amount: u64 },
    /// sub_vkey-authorized. Needs an upper bound.
    Claim { receipt: Receipt },
    /// add_vkey-authorized. Needs an upper bound.
    Close,
    /// add_vkey-authorized. Unambiguous: reclaims funds if the adaptor
    /// never staked a `Claim` before the close period elapsed. Needs a
    /// lower bound.
    Elapse,
    /// add_vkey-authorized. Unambiguous, requires some pendings to be
    /// timed-out: frees a Responded channel's never-unlocked pendings back
    /// into its recoverable balance. Needs a lower bound.
    Expire,
    /// add_vkey-authorized. Unambiguous once legal: finalizes the channel
    /// once no pendings remain, or - given a lower bound past every
    /// pending's timeout - forces closure incorporating naturally-expired
    /// pendings. Needs a lower bound only if the channel still has pending
    /// cheques; `None` is fine otherwise.
    End,
    /// sub_vkey-authorized. Unambiguous once the adaptor knows the
    /// secret(s) behind previously-locked cheques - unlocking is then a
    /// pure claim of value that's already theirs. Needs an upper bound.
    Unlock { secrets: Vec<Secret> },
}

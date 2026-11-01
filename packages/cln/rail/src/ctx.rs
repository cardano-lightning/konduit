//! The rail's own state, its instruction, and its `impl Rail`.
//!
//! CLN is the Cardano Lightning Network, so this rail's hop is a konduit
//! channel and its peer is a konduit node: the rail is a konduit client
//! pointed downstream. Its instruction is written in konduit's own
//! addressing, and everything else — the channel, the cheque, the squash
//! — is the rail's own.
//!
//! Everything here is downstream of konduit's inbound verification. The
//! rail is handed `Constraints` — an amount, a relative timeout and a
//! lock — and owns what it does with them. It keeps no margin of its
//! own: what it charges is what the peer charges, passed through. Nor
//! does it hold a volatility buffer, since both sides of this hop are
//! ADA on Cardano's clock and there is nothing to diverge.

use std::sync::Arc;

use crate::{
    Channel, Channels, Commits, Receipt, Signer, channel, channels,
    commits::{Claim, Commit},
    time::now,
};
use konduit_data::{
    ChequeBody, Duration, Lock, Secret, Squash, SquashBody, Verified, VerifyingKey,
};
use konduit_rail::{
    Constraints, Outcome, PreProblem, QuoteProblem, Quoted, Rail, Relative, Resolution,
};
use konduit_tmp::{Keytag, PayOutcome};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// What this rail is instructed with: who to pay, and how much they
/// receive.
///
/// One type for both operations, because a quote and a commit ask about
/// the same thing — konduit names neither the payee nor the amount, so
/// both travel here.
///
/// The amount is what reaches the payee, not what leaves this node: the
/// peer downstream is a konduit too and charges to be paid, so the rail
/// adds the peer's own fee on top and checks the sum against
/// `Constraints.amount` — the inbound cheque net of konduit's fee.
/// Whatever is left over is the consumer's own headroom and not
/// something the rail spends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Outbound {
    #[n(0)]
    pub payee: VerifyingKey,
    #[n(1)]
    pub amount: u64,
}

impl Outbound {
    pub fn new(payee: VerifyingKey, amount: u64) -> Self {
        Self { payee, amount }
    }
}

pub struct Ctx {
    channels: Channels,
    commits: Commits,
    signer: Signer,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("config: {0}")]
    Config(#[from] crate::config::Error),
    #[error("commits: {0}")]
    Commits(#[from] crate::commits::Error),
    #[error("channels: {0}")]
    Channels(#[from] channels::Error),
    #[error("channel: {0}")]
    Channel(#[from] channel::Error),
    #[error("peer sent an unverifiable receipt")]
    BadReceipt,
}

impl Ctx {
    pub fn init(config: crate::Config) -> Result<Self, Error> {
        config.verify()?;
        let signer = Signer::new(config.signer);
        let me = signer.verifying_key();
        let channels = Channels::from_config(
            &config.outbound,
            |entry| Receipt::new(signer.squash(entry.tag.clone(), SquashBody::zero())),
            |entry| Keytag::from_parts(&me, &entry.tag).to_hex(),
        );
        let commits = Commits::from_config(&config.known_commits)?;
        Ok(Self {
            channels,
            commits,
            signer,
        })
    }

    pub fn new(channels: Channels, commits: Commits, signer: Signer) -> Self {
        Self {
            channels,
            commits,
            signer,
        }
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.signer.verifying_key()
    }

    /// The outbound lookup, for seeding and for admin changes while
    /// running.
    pub fn channels(&self) -> &Channels {
        &self.channels
    }

    /// Look up a payment secret already known to this node, wherever it
    /// came from: our own payme, a commit we made, or an unlocked cheque
    /// still sitting in an outbound receipt.
    ///
    /// Distinct from the rail's `reveal`, which answers only for locks
    /// this rail committed under and says what it does not know.
    pub fn known_secret(&self, lock: &Lock) -> Option<Secret> {
        match self.commits.get(lock).map(|commit| commit.state) {
            Some(crate::commits::State::Resolved(secret)) => Some(secret),
            _ => None,
        }
        .or_else(|| {
            self.channels
                .all()
                .iter()
                .find_map(|(_, channel)| channel.reveal(lock))
        })
    }

    /// Drops whatever has elapsed. A resolved secret is kept as long as
    /// the commitment it unlocked, and no longer.
    pub fn prune(&self) {
        self.commits.prune(now());
    }

    /// The locks this rail is still waiting on, for the platform's
    /// resolver to poll.
    pub fn in_flight(&self) -> Vec<Lock> {
        self.commits.in_flight()
    }

    // ------------------------------------------------------------------------
    // -- Channel housekeeping
    // ------------------------------------------------------------------------

    /// Fetches whatever receipt the partner has on file for `channel` and
    /// reconciles it into our local state. A `None` (partner has never
    /// seen a squash from us) is a no-op, not an error.
    async fn recover_receipt(&self, channel: &Channel) -> Result<(), Error> {
        let Some(their_wire) = channel.fetch_receipt().await? else {
            return Ok(());
        };
        let their = Receipt::try_verify(their_wire, &self.signer.verifying_key(), channel.tag())
            .map_err(|_| Error::BadReceipt)?;
        channel.apply_sync(their)?;
        Ok(())
    }

    /// Proposes (and locally applies) a new squash from whatever's pending
    /// right now, then pushes it to the partner in the background. Signing
    /// is cheap/local so it happens synchronously; only the network push
    /// is backgrounded.
    fn spawn_dispatch_squash(&self, channel: &Arc<Channel>) {
        // TODO :: review this logic
        let body = match channel.maybe_propose_squash_body() {
            Ok(Some(body)) => body,
            Ok(None) => return,
            Err(err) => {
                tracing::warn!("proposing squash failed: {err}");
                return;
            }
        };
        let squash = self.signer.squash(channel.tag().clone(), body);
        if let Err(err) = channel.apply_squash(squash.clone()) {
            tracing::warn!("applying our own squash locally failed: {err}");
            return;
        }

        let channel = Arc::clone(channel);
        tokio::spawn(async move {
            if let Err(err) = Self::push_squash(&channel, squash).await {
                tracing::warn!("dispatching squash to partner failed: {err}");
            }
        });
    }

    async fn push_squash(channel: &Channel, squash: Squash<Verified>) -> Result<(), Error> {
        channel.squash(squash.into_unverified()).await?;
        Ok(())
    }

    /// May need to be called more than once to be in sync. Each channel is
    /// synced independently.
    pub async fn sync(&self) -> Vec<(VerifyingKey, Result<(), Error>)> {
        self.prune();
        let mut results = Vec::new();
        for (key, channel) in self.channels.all() {
            let result = self.sync_channel(&channel).await;
            results.push((key, result));
        }
        results
    }

    async fn sync_channel(&self, channel: &Arc<Channel>) -> Result<(), Error> {
        if !channel.has_pending_cheques() {
            return Ok(());
        }
        self.recover_receipt(channel).await?;
        channel.apply_timeout(now().saturating_sub(Duration::from_secs(120)))?;
        if let Some(body) = channel.maybe_propose_squash_body()? {
            let squash = self.signer.squash(channel.tag().clone(), body);
            channel.apply_squash(squash.clone())?;
            Self::push_squash(channel, squash).await?;
        }
        Ok(())
    }
}

/// Konduit supplies the inbound constraints and this rail owns
/// everything after them. One `commit`, no wrapper: the bodies live here
/// because this is where the state is.
impl Rail for Ctx {
    const NAME: &'static str = "cln";
    type QuoteInstruction = Outbound;
    type CommitInstruction = Outbound;

    /// What the peer charges to carry the instruction. The rail adds
    /// nothing of its own.
    async fn quote(&self, instruction: Outbound) -> Result<Quoted, QuoteProblem> {
        let channel = self
            .channels
            .get(&instruction.payee)
            .map_err(|err| QuoteProblem::Unroutable(err.to_string()))?;
        let quote = channel
            .quote(instruction.amount)
            .await
            .map_err(|err| QuoteProblem::RailUnavailable(err.to_string()))?;
        Ok(Quoted {
            fee: quote.fee,
            timeout: Relative(Duration::from_millis(quote.relative_timeout)),
        })
    }

    /// Commit `instruction.amount`, plus whatever the peer charges to
    /// receive it, outbound to `instruction.key`, within `inbound`.
    ///
    /// There is no error arm: once the outbound cheque is dispatched,
    /// anything short of an answer is `Pending`, because an error read as
    /// a non-payment would chase a payment that may well have been made.
    #[tracing::instrument(
        skip(self),
        fields(
            ceiling = inbound.amount,
            amount = instruction.amount,
            timeout = %inbound.timeout,
            lock = %inbound.lock,
        )
    )]
    async fn commit(&self, inbound: Constraints, instruction: Outbound) -> Outcome {
        // Constraints first, since refusing them costs nothing and
        // touches no network.
        if instruction.amount > inbound.amount {
            return Outcome::FailedPre(PreProblem::InsufficientMargin(format!(
                "instructed to pay {} with {} claimable net of fee",
                instruction.amount, inbound.amount
            )));
        }
        if inbound.timeout.is_zero() {
            return Outcome::FailedPre(PreProblem::InsufficientMargin(
                "no time left to make an outbound commitment".to_string(),
            ));
        }
        let channel = match self.channels.get(&instruction.payee) {
            Ok(channel) => channel,
            Err(err) => {
                return Outcome::FailedPre(QuoteProblem::Unroutable(err.to_string()).into());
            }
        };

        // The peer is a konduit, so being paid costs its own fee: the
        // outbound cheque carries the payee's amount plus that. What it
        // comes to is the peer's own quote, the same one this rail's
        // `quote` passed upstream, so the inbound cheque is already sized
        // for it.
        let peer = match channel.quote(instruction.amount).await {
            Ok(quote) => quote,
            Err(err) => {
                return Outcome::FailedPre(QuoteProblem::RailUnavailable(err.to_string()).into());
            }
        };
        let outbound = instruction.amount.saturating_add(peer.fee);
        if outbound > inbound.amount {
            return Outcome::FailedPre(PreProblem::InsufficientMargin(format!(
                "paying {} costs {outbound} with the peer's fee, with {} claimable net of ours",
                instruction.amount, inbound.amount
            )));
        }

        // Our receipt tracking is in-memory only, so it can lag behind
        // whatever the partner already has on file (e.g. after a
        // restart). Recover it before proposing an index, so we don't
        // propose one that collides with state the partner has seen.
        if let Err(err) = self.recover_receipt(&channel).await {
            return Outcome::FailedPre(QuoteProblem::RailUnavailable(err.to_string()).into());
        }

        let timeout = inbound.timeout.absolute(now());
        let index = channel.propose_index();
        match self
            .commits
            .claim(inbound.lock, Commit::new(index, outbound, timeout))
        {
            // The lock is the idempotency key: this is what the commit
            // already made under it came to, not a second payment.
            Claim::Existing(outcome) => return outcome,
            Claim::Fresh => {}
        }

        let body = ChequeBody::new(index, outbound, timeout, inbound.lock);
        let locked = self.signer.locked(channel.tag().clone(), body);
        if let Err(err) = channel.apply_locked(locked.clone()) {
            // Nothing is sent, so the claim is given up and a retry
            // under this lock is free to proceed.
            self.commits.release(&inbound.lock);
            return Outcome::FailedPre(
                QuoteProblem::RailUnavailable(format!("outbound channel: {err}")).into(),
            );
        }

        // The peer is the payee — a lookup keyed by payee is a peer
        // table, not a route table — so this is its terminal pay.
        let outcome = match channel.pay(locked.into_unverified()).await {
            Ok(outcome) => outcome,
            Err(err) => {
                // Dispatched. Whether it paid is not ours to say.
                tracing::warn!("outbound commit unanswered, reporting pending: {err}");
                return Outcome::Pending;
            }
        };
        let PayOutcome::Success(secret) = outcome else {
            return Outcome::Pending;
        };

        // Cache the secret before anything can squash the unlocked
        // cheque that carries it out of the receipt.
        if let Err(err) = self.commits.resolve(&inbound.lock, secret) {
            tracing::warn!("recording the revealed secret failed: {err}");
        }
        if let Err(err) = channel.apply_secret(secret) {
            tracing::warn!("applying the revealed secret locally failed: {err}");
        }
        // Get the resulting squash to the partner as soon as possible,
        // but in the background: the caller is waiting on this answer and
        // doesn't need to wait on that round-trip too.
        self.spawn_dispatch_squash(&channel);
        Outcome::Success(secret)
    }

    /// Delayed resolution for a lock this rail committed under.
    async fn reveal(&self, lock: &Lock) -> Resolution {
        if let Some(commit) = self.commits.get(lock) {
            return commit.resolution(now());
        }
        // Not ours, or pruned — though an outbound receipt may still
        // carry the unlocked cheque.
        match self
            .channels
            .all()
            .iter()
            .find_map(|(_, channel)| channel.reveal(lock))
        {
            Some(secret) => Resolution::Success(secret),
            None => Resolution::Unknown,
        }
    }
}

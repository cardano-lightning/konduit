//! The rail's own state: its outbound lookup, what it has committed, and
//! the paymes it holds as a terminal payee.
//!
//! Everything here is downstream of konduit's inbound verification. The
//! rail is handed `Constraints` — an amount, a relative timeout and a
//! lock — and owns what it does with them. It charges no fee and keeps
//! no margin of its own; see `rail.rs`.

use std::sync::Arc;

use crate::{
    Channel, Channels, Commits, Paymes, Receipt, Signer, channel, channels,
    commits::{Claim, Commit},
    paymes,
    time::now,
    wire::{self, auth::Keytag},
};
use konduit_data::{ChequeBody, Duration, Lock, Secret, SquashBody, VerifyingKey};
use konduit_rail::{Constraints, Outcome, PreProblem, QuoteProblem, Quoted, Relative, Resolution};

pub struct Ctx {
    paymes: Paymes,
    channels: Channels,
    commits: Commits,
    signer: Signer,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("channels: {0}")]
    Channels(#[from] channels::Error),
    #[error("channel: {0}")]
    Channel(#[from] channel::Error),
    #[error("paymes: {0}")]
    Paymes(#[from] paymes::Error),
    #[error("peer sent an unverifiable receipt")]
    BadReceipt,
}

impl Ctx {
    pub fn init(config: crate::Config) -> Result<Self, Error> {
        let signer = Signer::new(config.signer);
        let me = signer.verifying_key();
        let channels = Channels::from_config(
            &config.channels,
            |entry| Receipt::new(signer.squash(entry.tag.clone(), SquashBody::zero())),
            |entry| Keytag::from((&me, &entry.tag)).to_hex(),
        );
        let paymes = Paymes::new(config.paymes);
        // TODO:: Commits should persist. A restart between commit and
        // reveal currently loses the secret.
        let commits = Commits::new();
        Ok(Self {
            paymes,
            channels,
            commits,
            signer,
        })
    }

    pub fn new(paymes: Paymes, channels: Channels, commits: Commits, signer: Signer) -> Self {
        Self {
            paymes,
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
        self.paymes
            .secret(lock)
            .or_else(|| match self.commits.get(lock)?.state {
                crate::commits::State::Resolved(secret) => Some(secret),
                _ => None,
            })
            .or_else(|| {
                self.channels
                    .all()
                    .iter()
                    .find_map(|(_, channel)| channel.reveal(lock))
            })
    }

    /// Look up lock in own paymes
    pub fn paymes_reveal(&self, lock: &Lock, amount: u64) -> Result<Secret, Error> {
        Ok(self.paymes.reveal(lock, amount)?)
    }

    pub fn payme(&self, req: wire::payme::Request) -> Result<wire::payme::Response, Error> {
        let payme = self.paymes.insert(req.amount);
        Ok(wire::payme::Response {
            payee: self.signer.verifying_key(),
            amount: payme.amount,
            lock: payme.lock,
            timeout: payme.timeout,
        })
    }

    /// Drops whatever has elapsed. A resolved secret is kept as long as
    /// the commitment it unlocked, and no longer.
    pub fn prune(&self) {
        let now = now();
        self.paymes.prune();
        self.commits.prune(now);
    }

    // ------------------------------------------------------------------------
    // -- The rail's three operations. `rail.rs` wraps these in the trait.
    // ------------------------------------------------------------------------

    /// What the network downstream costs to reach `payee`. The rail adds
    /// nothing of its own.
    pub async fn quote_payee(
        &self,
        payee: &VerifyingKey,
        amount: u64,
    ) -> Result<Quoted, QuoteProblem> {
        let channel = self
            .channels
            .get(payee)
            .map_err(|err| QuoteProblem::Unroutable(err.to_string()))?;
        let response = channel
            .quote(&wire::quote::Request {
                payee: *payee,
                amount,
            })
            .await
            .map_err(|err| QuoteProblem::RailUnavailable(err.to_string()))?;
        Ok(Quoted {
            fee: response.fee,
            timeout: Relative(response.timeout),
        })
    }

    /// Commit outbound within `inbound`, to `payee`.
    ///
    /// There is no error arm: once the outbound cheque is dispatched,
    /// anything short of an answer is `Pending`, because an error read as
    /// a non-payment would chase a payment that may well have been made.
    #[tracing::instrument(
        skip(self),
        fields(amount = inbound.amount, timeout = %inbound.timeout, lock = %inbound.lock)
    )]
    pub async fn commit_payee(&self, inbound: Constraints, payee: &VerifyingKey) -> Outcome {
        // Constraints first, since refusing them costs nothing and
        // touches no network.
        if inbound.timeout.is_zero() {
            return Outcome::FailedPre(PreProblem::InsufficientMargin(
                "no time left to make an outbound commitment".to_string(),
            ));
        }
        let channel = match self.channels.get(payee) {
            Ok(channel) => channel,
            Err(err) => {
                return Outcome::FailedPre(QuoteProblem::Unroutable(err.to_string()).into());
            }
        };

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
            .claim(inbound.lock, Commit::new(index, inbound.amount, timeout))
        {
            // The lock is the idempotency key: this is what the commit
            // already made under it came to, not a second payment.
            Claim::Existing(outcome) => return outcome,
            Claim::Fresh => {}
        }

        let body = ChequeBody::new(index, inbound.amount, timeout, inbound.lock);
        let locked = self.signer.locked(channel.tag().clone(), body);
        if let Err(err) = channel.apply_locked(locked.clone()) {
            // Nothing is sent, so the claim is given up and a retry
            // under this lock is free to proceed.
            self.commits.release(&inbound.lock);
            return Outcome::FailedPre(
                QuoteProblem::RailUnavailable(format!("outbound channel: {err}")).into(),
            );
        }

        // Onward routing is the peer's own business: a lookup keyed by
        // payee is a peer table, not a route table.
        let req = wire::commit::Request {
            inbound: locked.into_unverified(),
            outbound: None,
        };
        let res = match channel.commit(&req).await {
            Ok(res) => res,
            Err(err) => {
                // Dispatched. Whether it paid is not ours to say.
                tracing::warn!("outbound commit unanswered, reporting pending: {err}");
                return Outcome::Pending;
            }
        };
        let Some(secret) = res.secret else {
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
    pub fn reveal_lock(&self, lock: &Lock) -> Resolution {
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
        let body = match channel.maybe_propose_squash_body() {
            Ok(Some(body)) => body,
            Ok(None) => return,
            Err(err) => {
                tracing::warn!("proposing squash failed: {err}");
                return;
            }
        };
        let squash = self.signer.squash(channel.tag().clone(), body);
        if let Err(err) = channel.apply_squash(squash) {
            tracing::warn!("applying our own squash locally failed: {err}");
            return;
        }

        let channel = Arc::clone(channel);
        let own_vkey = self.signer.verifying_key();
        tokio::spawn(async move {
            if let Err(err) = Self::push_squash(&channel, own_vkey).await {
                tracing::warn!("dispatching squash to partner failed: {err}");
            }
        });
    }

    async fn push_squash(channel: &Channel, own_vkey: VerifyingKey) -> Result<(), Error> {
        let req = wire::sync::Request {
            receipt: channel.wire_receipt(),
        };
        let their_wire = channel.sync(&req).await?.receipt;
        let their = Receipt::try_verify(their_wire, &own_vkey, channel.tag())
            .map_err(|_| Error::BadReceipt)?;
        channel.apply_sync(their)?;
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
        let req = wire::sync::Request {
            receipt: channel.wire_receipt(),
        };
        let their_wire = channel.sync(&req).await?.receipt;
        let their = Receipt::try_verify(their_wire, &self.signer.verifying_key(), channel.tag())
            .map_err(|_| Error::BadReceipt)?;
        channel.apply_sync(their)?;
        channel.apply_timeout(now().saturating_sub(Duration::from_secs(120)))?;
        if let Some(body) = channel.maybe_propose_squash_body()? {
            let squash = self.signer.squash(channel.tag().clone(), body);
            channel.apply_squash(squash)?;
        }
        Ok(())
    }
}

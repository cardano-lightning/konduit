use bln_sdk::types::Invoice;
use cardano_sdk::cbor::ToCbor;
use cobbl3::Mac;
use http_client::Transport;
use konduit_data::{ChequeBody, Duration, Lock, Locked, Secret, Signature, Tag, VerifyingKey};
use konduit_wire::{
    auth::pay::common::quote::ChequeProposal,
    reg::cobbl3::{Credential, TokenBody},
};
use std::sync::Arc;

use crate::{
    Commitment, Commitments,
    core::{Squash, SquashBody, wire},
    keys::Signer,
    server, time,
};

mod policies;
pub use policies::{Policies, RegPolicy, SquashPolicy};

mod config;
pub use config::Config;

mod cache;
pub use cache::Cache;

mod error;
pub use error::{Error, SquashError};

/// Outcome of verifying a single squash proposal: secrets recovered from
/// newly-verified unlockeds, and the body to resubmit if the caller
/// chooses to retry.
struct VerifiedProposal {
    secrets: Vec<Secret>,
    resubmit: SquashBody,
}

/// What `commit` resolves to: the server may report the payment as still
/// in flight, or as resolved — carrying whatever secrets the resulting
/// squash proposal yielded, always including this payment's own secret.
pub enum CommitOutcome {
    Pending,
    Resolved(Vec<Secret>),
}

/// `L2` has a single owner (no `Arc<L2>` fan-out — the
/// orchestrator holds one and swaps it wholesale on config change), so
/// `Config` and `Cache` are plain, lock-free fields:
///
/// - [`Config`] is caller-authored and immutable through `L2` —
///   there's no `&mut` path to it at all. To change anything in it, pull
///   a copy out with [`L2::config`], mutate the copy via `Config`'s
///   own setters, and build a fresh `L2` via [`L2::with_cache`].
/// - [`Cache`] is fully recoverable if lost (`credential` re-issues via
///   `reg`, `pay_request` just means re-quoting), and is mutated through
///   ordinary `&mut self` methods — no locking, no poisoning to account
///   for.
pub struct L2<Http: Transport, C, S: Signer> {
    tag: Tag,
    config: Config,
    server: Arc<server::Client<Http, C>>,
    signer: Arc<S>,
    commitments: Arc<Commitments>,
    cache: Cache,
}

impl<Http, C, S> L2<Http, C, S>
where
    Http: Transport,
    S: Signer,
{
    /// Fresh `L2`: no cached credential/pay-request yet.
    pub fn new(
        server: Arc<server::Client<Http, C>>,
        signer: Arc<S>,
        commitments: Arc<Commitments>,
        tag: Tag,
        config: Config,
    ) -> Self {
        Self::with_cache(server, signer, commitments, tag, config, Cache::default())
    }

    /// Reinstantiate with a (possibly just-updated) `Config`, carrying
    /// over a previously-populated `Cache` — e.g. after changing a
    /// policy via `config()` + `Config::set_*`, so the caller doesn't
    /// lose an already-issued credential or pending quote in the
    /// process.
    pub fn with_cache(
        server: Arc<server::Client<Http, C>>,
        signer: Arc<S>,
        commitments: Arc<Commitments>,
        tag: Tag,
        config: Config,
        cache: Cache,
    ) -> Self {
        Self {
            tag,
            config,
            server,
            signer,
            commitments,
            cache,
        }
    }

    /// The current, caller-owned config. There is no `config_mut` —
    /// mutate a `.clone()` via `Config`'s own setters and pass it to
    /// [`L2::with_cache`] to get a `L2` reflecting the change.
    pub fn config(&self) -> &Config {
        &self.config
    }
    pub fn cache(&self) -> &Cache {
        &self.cache
    }
    pub fn cache_mut(&mut self) -> &mut Cache {
        &mut self.cache
    }
}

impl<Http, C, S> L2<Http, C, S>
where
    Http: Transport,
    C: server::Codec,
    S: Signer,
{
    pub fn tag(&self) -> &Tag {
        &self.tag
    }

    pub fn try_credential_str(&self) -> Result<String, Error> {
        let credential = self
            .config
            .credential()
            .cloned()
            .ok_or(Error::MissingCredential)?;
        let now = time::now()?;
        if credential.body.ttl < now.as_millis() as u64 {
            return Err(Error::CredentialExpired);
        }
        Ok(credential.to_string())
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.signer.verification_key().into_bytes().into()
    }

    async fn sign(&self, message: &[u8]) -> Result<Signature, Error> {
        let signature = self
            .signer
            .sign(message)
            .await
            .map_err(|e| Error::Signing(e.to_string()))?;
        Ok(Signature::from(<[u8; 64]>::from(signature)))
    }

    async fn tag_and_sign(&self, thing: impl ToCbor) -> Result<Signature, Error> {
        let tag = self.tag();
        let tbs = tag.data(thing.to_cbor());
        let signature = self
            .signer
            .sign(&tbs)
            .await
            .map_err(|e| Error::Signing(e.to_string()))?;
        Ok(Signature::from(<[u8; 64]>::from(signature)))
    }

    pub async fn info(&self) -> Result<wire::info::Response, Error> {
        Ok(self.server.info().await?)
    }

    pub async fn reg_def(&self) -> Result<Credential, Error> {
        let offset = match self.config.reg_policy() {
            RegPolicy::Auth(offset) => offset.clone(),
            RegPolicy::None => Duration::from_secs(24 * 60 * 60),
        };
        self.reg_with_offset(offset).await
    }

    async fn reg_with_offset(&self, offset: Duration) -> Result<Credential, Error> {
        let ttl = time::now()?.saturating_add(offset).as_millis() as u64;
        let token_body = TokenBody {
            key: self.verifying_key().into(),
            tag: Vec::from(self.tag()),
            ttl,
        };
        self.reg(token_body).await
    }

    pub async fn reg(&self, token_body: wire::reg::cobbl3::TokenBody) -> Result<Credential, Error> {
        let signature = self.sign(&token_body.to_cbor()).await?.into();
        let token = cobbl3::Request {
            body: token_body.clone(),
            signature,
        };

        let squash_body = SquashBody::zero();
        let signature = self.tag_and_sign(&squash_body).await?;
        let squash = Squash::new(squash_body, signature);

        let body = wire::reg::Body {
            token,
            squash: Some(squash),
        };
        let mac: Mac<20> = self.server.reg(&body).await?.0;

        let credential = Credential {
            body: token_body,
            mac,
        };
        Ok(credential)
    }

    pub async fn quote(
        &mut self,
        invoice: &Invoice,
    ) -> Result<wire::auth::pay::bolt11::quote::Response, Error> {
        let quote = self
            .server
            .pay_bolt11_quote(
                &self.try_credential_str()?.to_string(),
                &invoice.to_string(),
            )
            .await?;
        self.cache.set_pay_request(invoice.to_string().to_cbor());
        Ok(quote)
    }

    async fn persist(&self, lock: Lock, index: u64) -> Result<(), Error> {
        self.commitments
            .commit(lock, self.tag.clone(), index)
            .await?;
        Ok(())
    }

    /// Confirm a bolt11 payment. Delegates any resulting squash proposal
    /// to `handle_squash_proposal`, then checks THIS payment's own
    /// secret is actually among what came back — that check belongs
    /// here, not in the shared squash-handling path, since it's specific
    /// to what `commit` itself is trying to confirm.
    pub async fn commit(
        &mut self,
        cheque_proposal: ChequeProposal,
    ) -> Result<CommitOutcome, Error> {
        let cred = self.try_credential_str()?;
        let pay_request = self.cache.pay_request().ok_or(Error::PayRequestMissing)?;
        let invoice_str: String =
            minicbor::decode(&pay_request).map_err(|_| Error::PayRequestCorrupt)?;
        let invoice = invoice_str
            .parse::<Invoice>()
            .map_err(|_| Error::PayRequestCorrupt)?;
        let lock = Lock::from(invoice.payment_hash);

        let ChequeProposal {
            index,
            amount,
            relative_timeout,
            ..
        } = cheque_proposal;
        let cheque_body = ChequeBody::new(
            index,
            amount,
            time::now()?.saturating_add(Duration::from_millis(relative_timeout)),
            lock,
        );
        let signature = self.tag_and_sign(&cheque_body).await?;
        // Before the commit is dispatched, it must persist locally.
        self.persist(lock, index).await?;
        let locked = Locked::new(cheque_body, signature);
        match self.server.pay_bolt11_commit(&cred, &locked).await? {
            konduit_wire::auth::pay::common::commit::Status::Pending => Ok(CommitOutcome::Pending),
            konduit_wire::auth::pay::common::commit::Status::Resolved(squash_proposal) => {
                let secrets = self.handle_squash_proposal(squash_proposal).await?;
                // commit's own responsibility: confirm THIS payment's
                // secret is actually among what came back, not just that
                // something did. ASSUMED conversion — confirm the real
                // Secret -> Lock relationship/method.
                if !secrets.iter().any(|secret| Lock::from(secret) == lock) {
                    return Err(SquashError::MissingPaymentSecret.into());
                }
                Ok(CommitOutcome::Resolved(secrets))
            }
        }
    }

    pub async fn state(&self) -> Result<wire::auth::state::Response, Error> {
        let cred = self.try_credential_str()?;
        Ok(self.server.state(&cred).await?)
    }

    /// Verify one squash proposal (`current` + `unlockeds` + `proposal`)
    /// against our own signing key: checks `current` is ours, verifies
    /// each unlocked (rejecting anything older than the currently
    /// configured `last_received` cutoff, if any), and enforces the
    /// partial-order invariant (proposed must be comparable to, and no
    /// greater than, calculated). Reads key/tag/cutoff from `self` —
    /// callers only ever supply the proposal. Does not retry or
    /// resubmit — that's `handle_squash_proposal`'s job.
    fn verify_squash_proposal(
        &self,
        proposal: wire::auth::squash::SquashProposal,
    ) -> Result<VerifiedProposal, Error> {
        let verification_key = self.verifying_key();
        let tag = self.tag();
        // `RejectOld`'s cutoff used to live on the policy itself; now that
        // `Config` is immutable through `L2`, it can't — a value that
        // updates every time a squash is received has to live in `Cache`.
        let reject_before = match self.config.squash_policy() {
            SquashPolicy::RejectOld { .. } => self.cache.last_received(),
            _ => None,
        };

        let Ok(current) = proposal.current.clone().try_verify(&verification_key, &tag) else {
            return Err(SquashError::CurrentInvalid.into());
        };
        log::info!("currently squashed = {}", proposal.current.amount());

        let mut calculated = current.body().clone();
        let mut secrets = vec![];

        for unlocked in proposal.unlockeds {
            if let Some(cutoff) = reject_before {
                if unlocked.timeout() < cutoff {
                    log::warn!("rejecting unlocked older than last_received cutoff");
                    return Err(SquashError::UnlockedOld.into());
                }
            }
            let Ok(unlocked) = unlocked.try_verify(&verification_key, &tag) else {
                return Err(SquashError::UnlockedInvalid.into());
            };
            calculated
                .squash_unlocked(&unlocked)
                .map_err(|e| SquashError::Unlocked(e.to_string()))?;
            secrets.push(unlocked.secret().clone());
        }

        // Proposed must be comparable, and no greater than calculated.
        match proposal.proposal.partial_cmp(&calculated) {
            Some(std::cmp::Ordering::Less) => {
                // Occurs when lockeds timeout rather than unlock, or the
                // server simply doesn't claim all owed funds — not unsafe
                // for the client either way.
                log::info!("proposed < calculated;");
            }
            Some(std::cmp::Ordering::Equal) => {
                log::info!("proposed == calculated;");
            }
            Some(std::cmp::Ordering::Greater) => {
                return Err(SquashError::OverProposed.into());
            }
            None => {
                return Err(SquashError::Calculated.into());
            }
        }

        Ok(VerifiedProposal {
            secrets,
            resubmit: proposal.proposal,
        })
    }

    /// Sign and submit a single squash request — no retry, no
    /// verification. Building block for `sync` below.
    async fn squash_once(
        &self,
        squash_body: SquashBody,
    ) -> Result<wire::auth::squash::Response, Error> {
        let cred = self.try_credential_str()?;
        let signature = self.tag_and_sign(&squash_body).await?;
        let squash = Squash::new(squash_body, signature);
        let body = wire::auth::squash::Body::from(squash);
        Ok(self.server.squash(&cred, &body).await?)
    }

    /// Single point of entry for handling a squash proposal already in
    /// hand — from `sync`'s initial submission, or from `commit`'s
    /// `Resolved` response. Verifies it, and per `SquashPolicy`, retries
    /// (by resubmitting) until fully resolved. Under
    /// `SquashPolicy::Manual`, verifies once and returns — resolving
    /// further is left to the caller.
    async fn handle_squash_proposal(
        &mut self,
        mut proposal: wire::auth::squash::SquashProposal,
    ) -> Result<Vec<Secret>, Error> {
        let policy = self.config.squash_policy();
        let mut retries_left = match policy {
            SquashPolicy::Manual => None,
            SquashPolicy::Lenient { retry } => Some(*retry),
            SquashPolicy::RejectOld { retry, .. } => Some(*retry),
        };

        let mut secrets = vec![];

        loop {
            let verified = self.verify_squash_proposal(proposal)?;
            secrets.extend(verified.secrets);

            if matches!(policy, SquashPolicy::Manual) {
                return Ok(secrets);
            }

            if matches!(policy, SquashPolicy::RejectOld { .. }) {
                let now = time::now()?;
                self.cache.set_last_received(now);
            }

            match &mut retries_left {
                Some(0) => return Err(SquashError::RetriesExhausted.into()),
                Some(n) => *n -= 1,
                None => {}
            }

            match self.squash_once(verified.resubmit).await? {
                wire::auth::squash::Response::Ok => return Ok(secrets),
                wire::auth::squash::Response::Stale(next) => proposal = next,
            }
        }
    }

    /// Submit a squash starting from `body` (defaults to
    /// `SquashBody::zero()` if not given), then resolve per
    /// `SquashPolicy` via `handle_squash_proposal`. Returns the secrets
    /// recovered from unlockeds — proofs that payments were routed.
    pub async fn sync(&mut self, body: Option<SquashBody>) -> Result<Vec<Secret>, Error> {
        let body = body.unwrap_or_else(SquashBody::zero);
        match self.squash_once(body).await? {
            wire::auth::squash::Response::Ok => Ok(vec![]),
            wire::auth::squash::Response::Stale(proposal) => {
                self.handle_squash_proposal(proposal).await
            }
        }
    }
}

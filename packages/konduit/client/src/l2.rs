// l2.rs — updated to build from State instead of the old top-level Cache
use bln_sdk::types::Invoice;
use cardano_sdk::cbor::ToCbor;
use cobbl3::Mac;
use http_client::Transport;
use konduit_data::{ChequeBody, Duration, Lock, Locked, Secret, Signature, Tag, VerifyingKey};
use konduit_wire::{
    auth::pay::common::quote::ChequeProposal,
    reg::cobbl3::{Credential, TokenBody},
};
use std::sync::{Arc, RwLock};

use crate::{
    Signer,
    core::{Squash, SquashBody, wire},
    time,
};

mod state;
pub use state::State;

mod commitments;
pub use commitments::{Commitments, Entry, Ko};

mod config;
pub use config::{Policies, RegPolicy, SquashPolicy};

mod cache;
pub use cache::Cache;

mod server;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("time: {0}")]
    Time(#[from] time::Error),
    #[error("no credential set: call reg first")]
    MissingCredential,
    #[error("credential expired: call reg again")]
    CredentialExpired,
    #[error("no pending payment request: must quote before commit")]
    PayRequestMissing,
    #[error("failed to parse pending payment request")]
    PayRequestCorrupt,
    #[error(transparent)]
    Squash(#[from] SquashError),
    #[error(transparent)]
    Server(#[from] server::Error),
    #[error(transparent)]
    State(#[from] state::Error),
    #[error("Signing error {0}")]
    Signing(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SquashError {
    #[error("current squash does not verify against our signing key")]
    CurrentInvalid,
    #[error("unlocked entry does not verify against our signing key")]
    UnlockedInvalid,
    #[error("unlocked entry predates last_received cutoff; rejecting as stale")]
    UnlockedOld,
    #[error("failed to merge verified unlocked into calculated squash: {0}")]
    Unlocked(String),
    #[error(
        "server proposal is not provably covered by calculated squash (proposed must be <= calculated under the partial order)"
    )]
    OverProposed,
    #[error("Calculated squash is incompatible with proposed")]
    Calculated,
    #[error("exhausted retry policy without reaching a complete squash")]
    RetriesExhausted,
    #[error("resolved squash proposal did not include the secret for this payment")]
    MissingPaymentSecret,
}

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

pub struct L2<Http: Transport, C, S: Signer> {
    server: Arc<server::Client<Http, C>>,
    signer: Arc<S>,
    state: RwLock<State>,
}

impl<Http, C, S> L2<Http, C, S>
where
    Http: Transport,
    S: Signer,
{
    pub fn new(server: Arc<server::Client<Http, C>>, signer: Arc<S>, tag: Tag) -> Self {
        Self {
            server,
            signer,
            state: RwLock::new(State::new(tag)),
        }
    }

    pub fn from_state(server: Arc<server::Client<Http, C>>, signer: Arc<S>, state: State) -> Self {
        Self {
            server,
            signer,
            state: RwLock::new(state),
        }
    }

    fn read_state(&self) -> State {
        self.state
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn write_state(&self, f: impl FnOnce(&mut State)) {
        let mut guard = self
            .state
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        f(&mut guard);
    }

    pub fn snapshot(&self) -> State {
        self.read_state()
    }
}

impl<Http, C, S> L2<Http, C, S>
where
    Http: Transport,
    C: server::Codec,
    S: Signer,
{
    pub fn tag(&self) -> Tag {
        self.read_state().tag()
    }

    pub fn reg_policy(&self) -> RegPolicy {
        self.read_state().reg_policy()
    }
    pub fn set_reg_policy(&self, policy: RegPolicy) {
        self.write_state(|s| s.set_reg_policy(policy));
    }
    pub fn squash_policy(&self) -> SquashPolicy {
        self.read_state().squash_policy()
    }
    pub fn set_squash_policy(&self, policy: SquashPolicy) {
        self.write_state(|s| s.set_squash_policy(policy));
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.signer.verification_key().into_bytes().into()
    }

    /// Async because it may trigger a registration with the server.
    async fn get_credential(&self) -> Result<String, Error> {
        let snapshot = self.read_state();
        let needs_reg = match snapshot.credential() {
            None => true,
            Some(credential) => credential.body.ttl <= time::now()?.as_millis() as u64,
        };

        if needs_reg {
            match snapshot.reg_policy() {
                RegPolicy::None => {
                    return Err(match snapshot.credential() {
                        None => Error::MissingCredential,
                        Some(_) => Error::CredentialExpired,
                    });
                }
                RegPolicy::Auth(offset) => {
                    self.reg_with_offset(offset).await?;
                }
            }
        }

        let credential = self.read_state().credential();
        credential
            .map(|c| c.to_string())
            .ok_or(Error::MissingCredential)
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

    pub async fn reg_def(&self) -> Result<(), Error> {
        let offset = match self.reg_policy() {
            RegPolicy::Auth(offset) => offset,
            RegPolicy::None => Duration::from_secs(24 * 60 * 60),
        };
        self.reg_with_offset(offset).await
    }

    async fn reg_with_offset(&self, offset: Duration) -> Result<(), Error> {
        let ttl = time::now()?.saturating_add(offset).as_millis() as u64;
        let token_body = TokenBody {
            key: self.verifying_key().into(),
            tag: Vec::from(&self.tag()),
            ttl,
        };
        self.reg(token_body).await
    }

    pub async fn reg(&self, token_body: wire::reg::cobbl3::TokenBody) -> Result<(), Error> {
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
        self.write_state(|c| c.set_credential(Some(credential)));
        Ok(())
    }

    pub async fn quote(
        &self,
        invoice: &Invoice,
    ) -> Result<wire::auth::pay::bolt11::quote::Response, Error> {
        let cred = self.get_credential().await?;
        let quote = self
            .server
            .pay_bolt11_quote(&cred, &invoice.to_string())
            .await?;
        self.write_state(|c| c.set_pay_request(invoice.to_string().to_cbor()));
        Ok(quote)
    }

    /// Confirm a bolt11 payment. Delegates any resulting squash proposal
    /// to `handle_squash_proposal`, then checks THIS payment's own
    /// secret is actually among what came back — that check belongs
    /// here, not in the shared squash-handling path, since it's specific
    /// to what `commit` itself is trying to confirm.
    pub async fn commit(&self, cheque_proposal: ChequeProposal) -> Result<CommitOutcome, Error> {
        let pay_request = self
            .read_state()
            .pay_request()
            .ok_or(Error::PayRequestMissing)?;
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
        // FIXME :: We should record the the the cheque somewhere persistent.
        // We know we can override this provided conditions are met.
        // We know we cannot (safely) sign a cheque with same lock and different index.
        let locked = Locked::new(cheque_body, signature);
        let cred = self.get_credential().await?;

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
        let cred = self.get_credential().await?;
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
        let reject_before = match self.squash_policy() {
            SquashPolicy::RejectOld { last_received, .. } => Some(last_received),
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
        let signature = self.tag_and_sign(&squash_body).await?;
        let squash = Squash::new(squash_body, signature);
        let body = wire::auth::squash::Body::from(squash);
        let cred = self.get_credential().await?;
        Ok(self.server.squash(&cred, &body).await?)
    }

    /// Single point of entry for handling a squash proposal already in
    /// hand — from `sync`'s initial submission, or from `commit`'s
    /// `Resolved` response. Verifies it, and per `SquashPolicy`, retries
    /// (by resubmitting) until fully resolved. Under
    /// `SquashPolicy::Manual`, verifies once and returns — resolving
    /// further is left to the caller.
    async fn handle_squash_proposal(
        &self,
        mut proposal: wire::auth::squash::SquashProposal,
    ) -> Result<Vec<Secret>, Error> {
        let policy = self.squash_policy();
        let mut retries_left = match policy {
            SquashPolicy::Manual => None,
            SquashPolicy::Lenient { retry } => Some(retry),
            SquashPolicy::RejectOld { retry, .. } => Some(retry),
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
                self.write_state(|c| {
                    c.set_squash_policy(c.squash_policy().update_last_received(now));
                });
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
    pub async fn sync(&self, body: Option<SquashBody>) -> Result<Vec<Secret>, Error> {
        let body = body.unwrap_or_else(SquashBody::zero);
        match self.squash_once(body).await? {
            wire::auth::squash::Response::Ok => Ok(vec![]),
            wire::auth::squash::Response::Stale(proposal) => {
                self.handle_squash_proposal(proposal).await
            }
        }
    }
}

use crate::{
    Signer,
    core::{Squash, SquashBody, wire},
    server, time,
};
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

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no credential set: call reg first")]
    MissingCredential,
    #[error("credential expired: call reg again")]
    CredentialExpired,
    #[error("automatic re-registration failed: {0}")]
    AutoRegFailed(Box<Error>),
    #[error("Expected populated cache, got empty. Must quote before commit")]
    CacheEmpty,
    #[error("Failed to parse Cache.")]
    CacheCorrupt,
    #[error(transparent)]
    Squash(#[from] SquashError),
    #[error(transparent)]
    Server(#[from] server::Error),
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

#[derive(Debug, Clone, Copy, minicbor::Encode, minicbor::Decode)]
pub enum RegPolicy {
    #[n(0)]
    None,
    #[n(1)]
    Auth(#[n(0)] Duration),
}

impl Default for RegPolicy {
    fn default() -> Self {
        RegPolicy::Auth(Duration::from_secs(24 * 60 * 60))
    }
}

#[derive(Debug, Clone, Copy, minicbor::Encode, minicbor::Decode)]
pub enum SquashPolicy {
    /// No automatic squash handling. `sync`/`commit` verify at most one
    /// proposal and return without resubmitting — resolving further is
    /// left to the caller.
    #[n(0)]
    Manual,
    #[n(1)]
    Lenient {
        #[n(0)]
        retry: u8,
    },
    /// Verify signatures, and reject any unlocked whose expiry predates
    /// `last_received` - the last time this client actually received a
    /// squash proposal from the server. Guards against replaying a stale
    /// proposal as current. Clock drift is not a practical concern at the
    /// hours/minutes timescales relevant here.
    #[n(2)]
    RejectOld {
        #[n(0)]
        retry: u8,
        #[n(1)]
        last_received: Duration,
    },
}

impl SquashPolicy {
    pub fn lenient(retry: u8) -> Self {
        SquashPolicy::Lenient { retry }
    }

    pub fn reject_old(retry: u8, last_received: Duration) -> Self {
        SquashPolicy::RejectOld {
            retry,
            last_received,
        }
    }

    pub fn update_last_received(self, last_received: Duration) -> Self {
        let Self::RejectOld { retry, .. } = self else {
            return self;
        };
        Self::RejectOld {
            retry,
            last_received,
        }
    }

    pub fn now_received(self) -> Self {
        self.update_last_received(Duration::from_millis(time::now()))
    }
}

/// `last_received` defaults to POSIX TIME 0.
/// Indicates no squash proposal ever received,
/// and more importantly all unlockeds are more recent.
impl Default for SquashPolicy {
    fn default() -> Self {
        SquashPolicy::reject_old(3, Duration::from_millis(0))
    }
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

#[derive(Debug, Clone, minicbor::Encode, minicbor::Decode)]
pub struct L2Data {
    #[n(0)]
    tag: Tag,
    #[n(1)]
    credential: Option<Credential>,
    #[n(2)]
    reg_policy: RegPolicy,
    #[n(3)]
    squash_policy: SquashPolicy,
    // Type erased bytes representing chached data.
    // Specifically, last quoted invoice.
    #[n(4)]
    cache: Option<Vec<u8>>,
}

impl L2Data {
    pub fn new(tag: Tag) -> Self {
        Self {
            tag,
            credential: None,
            reg_policy: RegPolicy::default(),
            squash_policy: SquashPolicy::default(),
            cache: None,
        }
    }

    pub fn tag(&self) -> &Tag {
        &self.tag
    }
    pub fn set_tag(&mut self, tag: Tag) {
        self.tag = tag;
    }
    pub fn credential(&self) -> Option<&Credential> {
        self.credential.as_ref()
    }
    pub fn set_credential(&mut self, credential: Option<Credential>) {
        self.credential = credential;
    }
    pub fn reg_policy(&self) -> RegPolicy {
        self.reg_policy
    }
    pub fn set_reg_policy(&mut self, policy: RegPolicy) {
        self.reg_policy = policy;
    }
    pub fn squash_policy(&self) -> SquashPolicy {
        self.squash_policy
    }
    pub fn set_squash_policy(&mut self, policy: SquashPolicy) {
        self.squash_policy = policy;
    }
    pub fn cache(&self) -> &Option<Vec<u8>> {
        &self.cache
    }
    pub fn set_cache(&mut self, cache: Vec<u8>) {
        self.cache = Some(cache);
    }
}

pub struct L2<Http: Transport, C, S: Signer> {
    server: Arc<server::Client<Http, C>>,
    signer: S,
    data: RwLock<L2Data>,
}

impl<Http, C, S> L2<Http, C, S>
where
    Http: Transport,
    S: Signer,
{
    pub fn new(server: Arc<server::Client<Http, C>>, signer: S, tag: Tag) -> Self {
        Self {
            server,
            signer,
            data: RwLock::new(L2Data::new(tag)),
        }
    }

    pub fn from_data(server: Arc<server::Client<Http, C>>, signer: S, data: L2Data) -> Self {
        Self {
            server,
            signer,
            data: RwLock::new(data),
        }
    }

    fn read_data(&self) -> L2Data {
        self.data
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn write_data(&self, f: impl FnOnce(&mut L2Data)) {
        let mut guard = self
            .data
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        f(&mut guard);
    }

    pub fn data(&self) -> L2Data {
        self.read_data()
    }
}

impl<Http, C, S> L2<Http, C, S>
where
    Http: Transport,
    C: server::Codec,
    S: Signer,
{
    pub fn tag(&self) -> Tag {
        self.read_data().tag().clone()
    }

    pub fn reg_policy(&self) -> RegPolicy {
        self.read_data().reg_policy()
    }
    pub fn set_reg_policy(&self, policy: RegPolicy) {
        self.write_data(|data| data.set_reg_policy(policy));
    }
    pub fn squash_policy(&self) -> SquashPolicy {
        self.read_data().squash_policy()
    }
    pub fn set_squash_policy(&self, policy: SquashPolicy) {
        self.write_data(|data| data.set_squash_policy(policy));
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.signer.verification_key().into_bytes().into()
    }

    /// Async because it may trigger a registration with the server.
    async fn get_credential(&self) -> std::result::Result<String, Error> {
        let snapshot = self.read_data();
        let needs_reg = match snapshot.credential() {
            None => true,
            Some(credential) => credential.body.ttl <= time::now(),
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
                    self.reg_with_offset(offset)
                        .await
                        .map_err(|e| Error::AutoRegFailed(Box::new(e)))?;
                }
            }
        }

        let credential = self.read_data().credential().cloned();
        credential
            .map(|c| c.to_string())
            .ok_or(Error::MissingCredential)
    }

    async fn sign(&self, message: &[u8]) -> std::result::Result<Signature, Error> {
        let signature = self
            .signer
            .sign(message)
            .await
            .map_err(|e| Error::Signing(e.to_string()))?;
        Ok(Signature::from(<[u8; 64]>::from(signature)))
    }

    async fn tag_and_sign(&self, thing: impl ToCbor) -> std::result::Result<Signature, Error> {
        let tag = self.tag();
        let tbs = tag.data(thing.to_cbor());
        let signature = self
            .signer
            .sign(&tbs)
            .await
            .map_err(|e| Error::Signing(e.to_string()))?;
        Ok(Signature::from(<[u8; 64]>::from(signature)))
    }

    pub async fn info(&self) -> std::result::Result<wire::info::Response, Error> {
        Ok(self.server.info().await?)
    }

    pub async fn reg_def(&self) -> std::result::Result<(), Error> {
        let offset = match self.reg_policy() {
            RegPolicy::Auth(offset) => offset,
            RegPolicy::None => Duration::from_secs(24 * 60 * 60),
        };
        self.reg_with_offset(offset).await
    }

    async fn reg_with_offset(&self, offset: Duration) -> std::result::Result<(), Error> {
        // ASSUMPTION: konduit_data::Duration exposes `.as_millis()` — not
        // verified against its real API.
        let web_offset = web_time::Duration::from_millis(offset.as_millis() as u64);
        let ttl = time::now_plus(web_offset);
        let token_body = TokenBody {
            key: self.verifying_key().into(),
            tag: Vec::from(&self.tag()),
            ttl,
        };
        self.reg(token_body).await
    }

    pub async fn reg(
        &self,
        token_body: wire::reg::cobbl3::TokenBody,
    ) -> std::result::Result<(), Error> {
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
        self.write_data(|data| data.set_credential(Some(credential)));
        Ok(())
    }

    pub async fn quote(
        &self,
        invoice: &Invoice,
    ) -> std::result::Result<wire::auth::pay::bolt11::quote::Response, Error> {
        let cred = self.get_credential().await?;
        let quote = self
            .server
            .pay_bolt11_quote(&cred, &invoice.to_string())
            .await?;
        self.write_data(|data| data.set_cache(invoice.to_string().to_cbor()));
        Ok(quote)
    }

    /// Confirm a bolt11 payment. Delegates any resulting squash proposal
    /// to `handle_squash_proposal`, then checks THIS payment's own
    /// secret is actually among what came back — that check belongs
    /// here, not in the shared squash-handling path, since it's specific
    /// to what `commit` itself is trying to confirm.
    pub async fn commit(
        &self,
        cheque_proposal: ChequeProposal,
    ) -> std::result::Result<CommitOutcome, Error> {
        let cache = self.read_data().cache().clone().ok_or(Error::CacheEmpty)?;
        let invoice_str: String = minicbor::decode(&cache).map_err(|_| Error::CacheCorrupt)?;
        let invoice = invoice_str
            .parse::<Invoice>()
            .map_err(|_| Error::CacheCorrupt)?;
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
            Duration::from_millis(time::now_plus_ms(relative_timeout)),
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
                // commit's own responsibility: confirm THIS payment's
                // secret is actually among what came back, not just that
                // something did. ASSUMED conversion — confirm the real
                // Secret -> Lock relationship/method.
                if !secrets.iter().any(|secret| Lock::from(secret) == lock) {
                    return Err(SquashError::MissingPaymentSecret.into());
                }
                let secrets = self.handle_squash_proposal(squash_proposal).await?;
                Ok(CommitOutcome::Resolved(secrets))
            }
        }
    }

    pub async fn state(&self) -> std::result::Result<wire::auth::state::Response, Error> {
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
    ) -> std::result::Result<VerifiedProposal, Error> {
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
    ) -> std::result::Result<wire::auth::squash::Response, Error> {
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
    ) -> std::result::Result<Vec<Secret>, Error> {
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
                self.write_data(|data| {
                    data.set_squash_policy(data.squash_policy().now_received());
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
    pub async fn sync(&self, body: Option<SquashBody>) -> std::result::Result<Vec<Secret>, Error> {
        let body = body.unwrap_or_else(SquashBody::zero);
        match self.squash_once(body).await? {
            wire::auth::squash::Response::Ok => Ok(vec![]),
            wire::auth::squash::Response::Stale(proposal) => {
                self.handle_squash_proposal(proposal).await
            }
        }
    }
}

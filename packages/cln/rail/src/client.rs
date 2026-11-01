//! Client is a lightweight, single-channel consumer: one outbound
//! Channel to a peer, used to pay merchants by requesting an invoice
//! directly from them, then committing payment through that peer.
//!
//! The peer is whoever this client has a channel with. In the demo that
//! is the gateway, but nothing here needs it to be: the client names a
//! payee and a rail, and the peer's own routing is the peer's business.

use konduit_data::{ChequeBody, Duration, Locked, Secret, SquashBody, Tag, Verified};
use serde::{Deserialize, Serialize};

use crate::{Channel, Outbound, Receipt, Signer, channel, signer, time::now};
use konduit_rail::Rail;
use konduit_tmp::{Keytag, PayOutcome, Payme, PaymeRequest, Quote};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub signer: signer::Config,
    /// The one peer this client pays through: where to reach it, and the
    /// tag of the channel held with it. Split the same way the rail
    /// splits them, so a client's config reads like a rail's.
    pub peer: channel::KnownKey,
    pub tag: Tag,
}

/// The same placeholder a rail's own config defaults to, so `init`
/// writes something that at least parses back.
impl Default for Config {
    fn default() -> Self {
        let entry = channel::Entry::default();
        Self {
            signer: Default::default(),
            peer: Default::default(),
            tag: entry.tag,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("merchant request failed: {0}")]
    Merchant(#[from] reqwest::Error),
    #[error("channel: {0}")]
    Channel(#[from] channel::Error),
    #[error("peer sent an unverifiable receipt")]
    BadReceipt,
    #[error("peer did not return a secret")]
    NoSecret,
}

pub struct Client {
    signer: Signer,
    peer: Channel,
    http: reqwest::Client,
}

impl Client {
    pub fn init(config: Config) -> Self {
        let signer = Signer::new(config.signer);
        let receipt = Receipt::new(signer.squash(config.tag.clone(), SquashBody::zero()));
        let auth = Keytag::from_parts(&signer.verifying_key(), &config.tag).to_hex();
        let peer = Channel::new(&config.peer, config.tag.clone(), receipt, auth);
        Self {
            signer,
            peer,
            http: reqwest::Client::new(),
        }
    }

    /// One reconciliation round. Returns `true` if a squash was proposed
    /// and sent, meaning another round may find more to do.
    #[tracing::instrument(level = "debug", skip(self), err, ret)]
    async fn sync_once(&self) -> Result<bool, Error> {
        self.recover_receipt().await?;
        self.peer
            .apply_timeout(now().saturating_sub(Duration::from_secs(120)))?;

        let Some(body) = self.peer.maybe_propose_squash_body()? else {
            return Ok(false);
        };
        let squash = self.signer.squash(self.peer.tag().clone(), body);
        self.peer.apply_squash(squash.clone())?;
        self.peer.squash(squash.into_unverified()).await?;
        Ok(true)
    }

    #[tracing::instrument(level = "debug", skip(self), err)]
    async fn sync_peer(&self) -> Result<(), Error> {
        if self.sync_once().await? {
            self.sync_once().await?;
        }
        Ok(())
    }

    /// Requests a payme from `merchant_url` — unauthenticated, no channel.
    #[tracing::instrument(skip_all, err)]
    async fn request_payme(&self, merchant_url: &str, amount: u64) -> Result<Payme, Error> {
        Ok(self
            .http
            .post(format!("{merchant_url}/pay/konduit/payme"))
            .json(&PaymeRequest { amount })
            .send()
            .await?
            .json()
            .await?)
    }

    /// Signs a cheque to the peer for the merchant's invoice plus the
    /// peer's quoted fee, and records it locally before dispatching.
    #[tracing::instrument(
        skip(self, payme, quote),
        fields(invoice_amount = payme.amount, fee = quote.fee, total = payme.amount + quote.fee),
        err
    )]
    fn lock_cheque(&self, payme: &Payme, quote: &Quote) -> Result<Locked<Verified>, Error> {
        let pay_amount = payme.amount + quote.fee;
        let buffer = Duration::from_secs(30); // GUESS: arbitrary safety margin
        let timeout = now() + Duration::from_millis(quote.relative_timeout) + buffer;
        let body = ChequeBody::new(self.peer.propose_index(), pay_amount, timeout, payme.lock);
        let locked = self.signer.locked(self.peer.tag().clone(), body);
        self.peer.apply_locked(locked.clone())?;
        Ok(locked)
    }

    /// Fetches whatever receipt the peer already has on file for us and
    /// reconciles it into our local state. A `None` (the peer has never
    /// seen a squash from us) is a no-op, not an error. This client has
    /// no persistence between invocations — local state always starts at
    /// `SquashBody::zero()` (see `init`) — so this always has to run
    /// before anything else: without it, `sync_peer` would instead push
    /// that fresh zero state outward, which only ever works the very
    /// first time (any later call regresses whatever the peer already
    /// has recorded, and gets rejected as a stale squash).
    #[tracing::instrument(level = "debug", skip(self), err)]
    async fn recover_receipt(&self) -> Result<(), Error> {
        let Some(their_wire) = self.peer.fetch_receipt().await? else {
            return Ok(());
        };
        let their = Receipt::try_verify(their_wire, &self.signer.verifying_key(), self.peer.tag())
            .map_err(|_| Error::BadReceipt)?;
        self.peer.apply_sync(their)?;
        Ok(())
    }

    /// Requests a payme from `merchant_url`, pays it through the peer,
    /// and returns the revealed secret.
    #[tracing::instrument(skip(self), err)]
    pub async fn pay(&self, merchant_url: &str, amount: u64) -> Result<Paid, Error> {
        // 0. Recover whatever receipt the peer already has on file, then
        // establish/reconcile our view of the channel with it.
        self.recover_receipt().await?;
        self.sync_peer().await?;

        // 1. payme, direct from merchant — unauthenticated, no channel.
        let payme = self.request_payme(merchant_url, amount).await?;

        // 2. Quote the peer's cln route to the merchant. The rail is the
        //    consumer's pick, and it goes in the path. One instruction
        //    serves both the quote and the pay: it names the payee and
        //    what they receive, which is all either needs.
        let rail = <crate::Ctx as Rail>::NAME;
        let instruction = Outbound::new(payme.payee, payme.amount);
        let encoded = self.peer.encode_instruction(&instruction)?;
        // The amount goes to konduit as well as in the instruction: the
        // instruction is for the rail, and konduit needs its own copy to
        // size the cheque it asks for and check the channel can back it.
        let quote = self
            .peer
            .quote_via(rail, payme.amount, encoded.clone())
            .await?;

        // 3. Sign our own cheque to the peer, and record it locally
        //    before dispatching.
        let locked = self.lock_cheque(&payme, &quote)?;
        let committed_amount = locked.amount();

        // 4. Pay, instructing the peer's rail to carry it onward.
        let outcome = self
            .peer
            .pay_via(rail, locked.into_unverified(), encoded)
            .await?;

        let PayOutcome::Success(secret) = outcome else {
            return Err(Error::NoSecret);
        };
        self.peer.apply_secret(secret)?;
        Ok(Paid {
            secret,
            payme_amount: payme.amount,
            committed_amount,
        })
    }
}

/// Result of a successful `pay`: the revealed secret plus the amounts
/// actually involved, since the merchant's invoice (and so the amount
/// committed, which also includes the routing fee) may differ from the
/// amount requested.
#[derive(Debug, Clone, Copy)]
pub struct Paid {
    pub secret: Secret,
    pub payme_amount: u64,
    pub committed_amount: u64,
}

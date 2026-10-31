//! Client is a lightweight, single-channel consumer: one outbound
//! Channel to a gateway, used to pay merchants by requesting an
//! invoice directly from them, then committing payment through the
//! gateway.

use konduit_data::{ChequeBody, Duration, Locked, Secret, SquashBody, Verified};
use serde::{Deserialize, Serialize};

use crate::{Channel, Receipt, Signer, channel, signer, time::now, wire, wire::auth::Keytag};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub signer: signer::Config,
    pub gateway: channel::Config,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("merchant request failed: {0}")]
    Merchant(#[from] reqwest::Error),
    #[error("channel: {0}")]
    Channel(#[from] channel::Error),
    #[error("gateway sent an unverifiable receipt")]
    BadReceipt,
    #[error("gateway did not return a secret")]
    NoSecret,
}

pub struct Client {
    signer: Signer,
    gateway: Channel,
    http: reqwest::Client,
}

impl Client {
    pub fn init(config: Config) -> Self {
        let signer = Signer::new(config.signer);
        let receipt = Receipt::new(signer.squash(config.gateway.tag.clone(), SquashBody::zero()));
        let auth = Keytag::from((&signer.verifying_key(), &config.gateway.tag)).to_hex();
        let gateway = Channel::new(&config.gateway, receipt, auth);
        Self {
            signer,
            gateway,
            http: reqwest::Client::new(),
        }
    }

    /// One sync round-trip. Returns `true` if a squash was proposed and
    /// applied locally — meaning a follow-up round-trip is needed to send it.
    #[tracing::instrument(level = "debug", skip(self), err, ret)]
    async fn sync_once(&self) -> Result<bool, Error> {
        let req = wire::sync::Request {
            receipt: self.gateway.wire_receipt(),
        };
        let their = Receipt::try_verify(
            self.gateway.sync(&req).await?.receipt,
            &self.signer.verifying_key(),
            self.gateway.tag(),
        )
        .map_err(|_| Error::BadReceipt)?;
        self.gateway.apply_sync(their)?;
        self.gateway
            .apply_timeout(now().saturating_sub(Duration::from_secs(120)))?;

        let Some(body) = self.gateway.maybe_propose_squash_body()? else {
            return Ok(false);
        };
        self.gateway
            .apply_squash(self.signer.squash(self.gateway.tag().clone(), body))?;
        Ok(true)
    }

    #[tracing::instrument(level = "debug", skip(self), err)]
    async fn sync_gateway(&self) -> Result<(), Error> {
        if self.sync_once().await? {
            self.sync_once().await?;
            self.sync_once().await?;
        }
        Ok(())
    }

    /// Requests a payme from `merchant_url` — unauthenticated, no channel.
    #[tracing::instrument(skip_all, err)]
    async fn request_payme(
        &self,
        merchant_url: &str,
        amount: u64,
    ) -> Result<wire::payme::Response, Error> {
        Ok(self
            .http
            .post(format!("{merchant_url}{}", wire::payme::PATH))
            .json(&wire::payme::Request { amount })
            .send()
            .await?
            .json()
            .await?)
    }

    /// Signs a cheque to the gateway for the merchant's invoice plus the
    /// gateway's routing fee, and records it locally before dispatching.
    #[tracing::instrument(
        skip(self, payme, quote),
        fields(invoice_amount = payme.amount, fee = quote.fee, total = payme.amount + quote.fee),
        err
    )]
    fn lock_cheque(
        &self,
        payme: &wire::payme::Response,
        quote: &wire::quote::Response,
    ) -> Result<Locked<Verified>, Error> {
        let pay_amount = payme.amount + quote.fee;
        let buffer = Duration::from_secs(30); // GUESS: arbitrary safety margin
        let timeout = now() + quote.timeout + buffer;
        let body = ChequeBody::new(
            self.gateway.propose_index(),
            pay_amount,
            timeout,
            payme.lock,
        );
        let locked = self.signer.locked(self.gateway.tag().clone(), body);
        self.gateway.apply_locked(locked.clone())?;
        Ok(locked)
    }

    /// Requests an payme from `merchant_url`, pays it through the
    /// gateway, and returns the revealed secret.
    #[tracing::instrument(skip(self), err)]
    pub async fn pay(&self, merchant_url: &str, amount: u64) -> Result<Paid, Error> {
        // 0. Establish/reconcile our view of the gateway channel.
        self.sync_gateway().await?;

        // 1. payme, direct from merchant — unauthenticated, no channel.
        let payme = self.request_payme(merchant_url, amount).await?;

        // 2. Quote the gateway's route to the merchant.
        let quote = self
            .gateway
            .quote(&wire::quote::Request {
                payee: payme.payee,
                amount: payme.amount,
            })
            .await?;

        // 3. Sign our own cheque to the gateway, and record it locally
        //    before dispatching.
        let locked = self.lock_cheque(&payme, &quote)?;
        let committed_amount = locked.amount();

        // 5. Commit, routed onward to the merchant.
        let req = wire::commit::Request {
            inbound: locked.into_unverified(),
            outbound: Some(wire::commit::Outbound {
                key: payme.payee,
                amount: payme.amount,
            }),
        };
        let res = self.gateway.commit(&req).await?;

        let secret = res.secret.ok_or(Error::NoSecret)?;
        self.gateway.apply_secret(secret)?;
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

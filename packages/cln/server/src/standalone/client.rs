//! Client is a lightweight, single-channel consumer: one outbound
//! Channel to a gateway, used to pay merchants by requesting an
//! invoice directly from them, then committing payment through the
//! gateway.

use konduit_data::{ChequeBody, Duration, Secret, SquashBody};
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

    async fn sync_gateway(&self) -> Result<(), Error> {
        if self.sync_once().await? {
            self.sync_once().await?;
            self.sync_once().await?;
        }
        Ok(())
    }

    /// Requests an payme from `merchant_url`, pays it through the
    /// gateway, and returns the revealed secret.
    pub async fn pay(&self, merchant_url: &str, amount: u64) -> Result<Secret, Error> {
        // 0. Establish/reconcile our view of the gateway channel.
        self.sync_gateway().await?;

        // 1. payme, direct from merchant — unauthenticated, no channel.
        let payme: wire::payme::Response = self
            .http
            .post(format!("{merchant_url}{}", wire::payme::PATH))
            .json(&wire::payme::Request { amount })
            .send()
            .await?
            .json()
            .await?;

        // 2. Quote the gateway's route to the merchant.
        let quote = self
            .gateway
            .quote(&wire::quote::Request {
                payee: payme.payee.clone(),
                amount: payme.amount,
            })
            .await?;

        // 3. Sign our own cheque to the gateway.
        let pay_amount = payme.amount + quote.fee;
        let buffer = Duration::from_secs(30); // GUESS: arbitrary safety margin
        let timeout = now() + quote.timeout + buffer;
        let body = ChequeBody::new(
            self.gateway.propose_index(),
            pay_amount,
            timeout,
            payme.lock.clone(),
        );
        let locked = self.signer.locked(self.gateway.tag().clone(), body);

        // 4. Record before dispatching.
        self.gateway.apply_locked(locked.clone())?;

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
        self.gateway.apply_secret(secret.clone())?;
        Ok(secret)
    }
}

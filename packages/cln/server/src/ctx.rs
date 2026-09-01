use std::sync::Arc;

use crate::{
    Channel, Channels, Commits, Paymes, Receipt, Signer, channel, channels,
    commits::{self, Commit},
    paymes,
    time::now,
    wire::{self, auth::Keytag, commit::Outbound},
};
use konduit_data::{ChequeBody, Duration, SquashBody, VerifyingKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Params {
    // Fee is a simple flat fee
    pub fee: u64,
    // This the _relative timeout of single_ hop for handling a cheque.
    // This is distinct from paymes timeout which is the absolute timeout.
    pub hop_timeout: Duration,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            fee: 1414,
            hop_timeout: Duration::from_secs(300),
        }
    }
}

pub struct Ctx {
    params: Params,
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
    #[error("commits: {0}")]
    Commits(#[from] commits::Error),
    #[error("retries not supported, lock already used")]
    NoRetries,
    #[error("inbound does not cover outbound")]
    NegativeFee,
    #[error("inbound does not cover outbound + fee")]
    InsufficientFee,
    #[error("inbound has insufficient timebound")]
    InsufficientTime,
    #[error("peer sent an unverifiable receipt")]
    BadReceipt,
}

impl Ctx {
    pub fn init(config: crate::Config) -> Result<Self, Error> {
        let signer = Signer::new(config.signer);
        let me = signer.verifying_key();
        let mut channels = Channels::new();
        for channel_config in config.channels.channels.iter() {
            let receipt =
                Receipt::new(signer.squash(channel_config.tag.clone(), SquashBody::zero()));
            channels.insert(
                channel_config.key,
                Channel::new(
                    channel_config,
                    receipt,
                    Keytag::from((&me, &channel_config.tag)).to_hex(),
                ),
            );
        }

        let paymes = Paymes::new(config.paymes);
        // TODO:: Commits should persist.
        let commits = Commits::new();
        Ok(Self {
            params: config.params,
            paymes,
            channels,
            commits,
            signer,
        })
    }

    pub fn new(
        params: Params,
        paymes: Paymes,
        channels: Channels,
        commits: Commits,
        signer: Signer,
    ) -> Self {
        Self {
            params,
            channels,
            paymes,
            commits,
            signer,
        }
    }

    /// Currently fee is a fixed flat fee.
    fn fee(&self) -> u64 {
        self.params.fee
    }

    fn timeout(&self) -> &Duration {
        &self.params.hop_timeout
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

    pub async fn quote(&self, req: wire::quote::Request) -> Result<wire::quote::Response, Error> {
        let payee = req.payee.clone();
        if payee == self.signer.verifying_key() {
            Ok(wire::quote::Response {
                fee: 0,
                timeout: self.timeout().clone(),
            })
        } else {
            let res = self.channels.get(&payee)?.quote(&req).await?;
            let fee = res.fee + self.fee();
            let timeout = res.timeout.saturating_add(self.timeout().clone());
            Ok(wire::quote::Response { fee, timeout })
        }
    }

    pub async fn commit(
        &self,
        req: wire::commit::Request,
    ) -> Result<wire::commit::Response, Error> {
        let wire::commit::Request { inbound, outbound } = req;

        // 1. Verify inbound Locked.
        // OOB:
        // - L1: cheque is backed.
        // - L2: cheque is valid wrt to current receipt.
        //
        // In bounds:
        //
        // - Timeout.
        // - If routed, then fee.
        let timeout = inbound.timeout().saturating_sub(self.timeout().clone());
        if timeout < now() {
            return Err(Error::InsufficientTime);
        }

        let Some(Outbound { key, amount }) = outbound else {
            // 2... Self is terminal
            let secret = self.paymes.reveal(inbound.lock(), inbound.amount())?;
            return Ok(wire::commit::Response {
                secret: Some(secret),
            });
        };

        // If routed then routing fee.
        let fee = inbound
            .amount()
            .checked_sub(amount)
            .ok_or(Error::NegativeFee)?;
        if fee < self.fee() {
            return Err(Error::InsufficientFee);
        }

        // 3. Get existing commit to Lock.
        if let Some(_commit) = self.commits.get(inbound.lock()) {
            return Err(Error::NoRetries);
        };

        // 2. Get outbound route
        // 2... Only single path, one-hop allowed.
        let channel = self.channels.get(&key)?;

        // 3. Make outbound Locked.
        let body = ChequeBody::new(
            channel.propose_index(),
            amount,
            timeout,
            inbound.lock().clone(),
        );
        let locked = self.signer.locked(channel.tag().clone(), body);

        // 4. Record commit to Lock.
        self.commits.insert(inbound.lock().clone(), Commit::new())?;
        channel.apply_locked(locked.clone())?;

        // 5. Dispatch and await.
        let req = wire::commit::Request {
            inbound: locked.into_unverified(),
            outbound: None,
        };
        // TODO :: handle errors. For now just propogate them
        let res = channel.commit(&req).await?;
        // 6. If secret then record secret.
        if let Some(secret) = res.secret {
            channel.apply_secret(secret)?;
        }

        // TODOD :: call sync on channel but do not await!
        //
        // 7. Relay response.
        Ok(res)
    }

    /// May need to be called more than once to be in sync. Each channel is
    /// synced independently.
    pub async fn sync(&self) -> Vec<(VerifyingKey, Result<(), Error>)> {
        let mut results = Vec::new();
        for (key, channel) in self.channels.iter() {
            let result = self.sync_channel(channel).await;
            results.push((key.clone(), result));
        }
        results
    }

    async fn sync_channel(&self, channel: &Arc<Channel>) -> Result<(), Error> {
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

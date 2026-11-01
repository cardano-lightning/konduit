use std::sync::Arc;

use crate::{
    Channel, Channels, Commits, Paymes, Receipt, Signer, channel, channels,
    commits::{self, Commit},
    paymes,
    time::now,
    wire::{self, auth::Keytag, commit::Outbound},
};
use konduit_data::{ChequeBody, Duration, Lock, SquashBody, VerifyingKey};
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

    pub fn verifying_key(&self) -> VerifyingKey {
        self.signer.verifying_key()
    }

    /// Look up a payment secret already known to this node.
    pub fn reveal(&self, lock: &konduit_data::Lock) -> Option<konduit_data::Secret> {
        self.paymes.secret(lock).or_else(|| {
            self.channels
                .iter()
                .find_map(|(_, channel)| channel.reveal(lock))
        })
    }

    /// Look up lock in own paymes
    pub fn paymes_reveal(
        &self,
        lock: &konduit_data::Lock,
        amount: u64,
    ) -> Result<konduit_data::Secret, Error> {
        Ok(self.paymes.reveal(lock, amount)?)
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
        let payee = req.payee;
        if payee == self.signer.verifying_key() {
            Ok(wire::quote::Response {
                index: 0, // FIXME :: Placeholder, 0 is an Impossible index.
                fee: 0,
                timeout: Duration::from_millis(0),
            })
        } else {
            Ok(self.channels.get(&payee)?.quote(&req).await?)
        }
    }

    pub async fn commit2(
        &self,
        payee: &VerifyingKey,
        amount: u64,
        timeout: Duration,
        lock: &Lock,
    ) -> Result<wire::commit::Response, Error> {
        // 1. Get existing commit to Lock.
        if let Some(_commit) = self.commits.get(lock) {
            return Err(Error::NoRetries);
        };

        // 2. Get outbound route
        let channel = self.channels.get(payee)?;

        // 2a. Our own receipt tracking is in-memory only (see Ctx::init),
        // so it can lag behind whatever the partner already has on file
        // (e.g. after a restart). Recover it before proposing the next
        // index, so we don't propose one that collides with state the
        // partner has already seen.
        self.recover_receipt(channel).await?;

        // 3. Make outbound Locked.
        let body = ChequeBody::new(channel.propose_index(), amount, timeout, lock.clone());
        let locked = self.signer.locked(channel.tag().clone(), body);

        // 4. Record commit to Lock.
        self.commits.insert(*lock, Commit::new())?;
        channel.apply_locked(locked.clone())?;

        // 5. Dispatch and await.
        let req = wire::commit::Request {
            inbound: locked.into_unverified(),
            outbound: None,
        };
        // TODO :: handle errors. For now just propogate them
        let res = channel.commit(&req).await?;
        // 6. If secret then record secret, and get the resulting squash to
        // the partner as soon as possible — but as a background task:
        // the caller is waiting on this response and doesn't need to wait
        // on that extra network round-trip too.
        if let Some(secret) = res.secret {
            channel.apply_secret(secret)?;
            self.spawn_dispatch_squash(channel);
        }
        Ok(res)
    }

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
    pub async fn commit(
        &self,
        req: wire::commit::Request,
    ) -> Result<wire::commit::Response, Error> {
        // payee
        // amount
        // lock
        // timeout
        let wire::commit::Request { inbound, outbound } = req;

        // 1. Verify inbound Locked.
        // OOB/ responsibility of caller:
        // - L1: channel / cheque is backed.
        // - L2: cheque is valid wrt to current receipt.
        //
        // In bounds:
        //
        // - Timeout.
        // - If routed, then fee.
        let timeout = inbound.timeout().saturating_sub(*self.timeout());
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
        let body = ChequeBody::new(channel.propose_index(), amount, timeout, *inbound.lock());
        let locked = self.signer.locked(channel.tag().clone(), body);

        // 4. Record commit to Lock.
        self.commits.insert(*inbound.lock(), Commit::new())?;
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
            results.push((*key, result));
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

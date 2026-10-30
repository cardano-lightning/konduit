//! Reusable state and protocol operations for a CLN peer node.
//!
//! This composes inbound peer accounting with the core outbound routing
//! context. Process hosts can provide their own transport, listener, and
//! configuration while sharing this protocol state machine.

use std::collections::BTreeMap;

use crate::{
    Outbound, ctx,
    inbounds::{self, Inbounds},
    receipt::WireReceipt,
    wire::{self, auth::Keytag},
};
use konduit_data::VerifyingKey;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("inbound: {0}")]
    Inbound(#[from] inbounds::Error),
    #[error("outbound: {0}")]
    Outbound(#[from] ctx::Error),
}

pub struct Ctx {
    inbounds: Inbounds,
    outbounds: Outbound,
}

impl Ctx {
    pub fn init(inbound: inbounds::Config, outbound: crate::Config) -> Result<Self, Error> {
        Ok(Self::new(Inbounds::new(inbound), Outbound::init(outbound)?))
    }

    pub fn new(inbounds: Inbounds, outbounds: Outbound) -> Self {
        Self {
            inbounds,
            outbounds,
        }
    }

    pub fn payme(&self, req: wire::payme::Request) -> Result<wire::payme::Response, Error> {
        Ok(self.outbounds.payme(req)?)
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.outbounds.verifying_key()
    }

    pub fn reveal(&self, lock: &konduit_data::Lock) -> Option<konduit_data::Secret> {
        self.outbounds.reveal(lock)
    }

    pub async fn quote_outbound(
        &self,
        req: wire::quote::Request,
    ) -> Result<wire::quote::Response, Error> {
        Ok(self.outbounds.quote(req).await?)
    }

    pub async fn commit_outbound(
        &self,
        req: wire::commit::Request,
    ) -> Result<wire::commit::Response, Error> {
        Ok(self.outbounds.commit(req).await?)
    }

    pub async fn sync_outbound(&self) -> Vec<(VerifyingKey, Result<(), Error>)> {
        self.outbounds
            .sync()
            .await
            .into_iter()
            .map(|(key, result)| (key, result.map_err(Error::from)))
            .collect()
    }

    /// Authenticates the peer keytag before forwarding its quote request.
    pub async fn quote(
        &self,
        keytag: &Keytag,
        req: wire::quote::Request,
    ) -> Result<wire::quote::Response, Error> {
        self.inbounds.contains(keytag)?;
        Ok(self.outbounds.quote(req).await?)
    }

    pub async fn commit(
        &self,
        keytag: &Keytag,
        req: wire::commit::Request,
    ) -> Result<wire::commit::Response, Error> {
        let wire::commit::Request { inbound, outbound } = req;
        self.inbounds.apply_locked(keytag, inbound.clone())?;
        let outbound = if outbound
            .as_ref()
            .is_some_and(|route| route.key == self.outbounds.verifying_key())
        {
            None
        } else {
            outbound
        };
        let res = self
            .outbounds
            .commit(wire::commit::Request { inbound, outbound })
            .await?;
        if let Some(secret) = res.secret {
            self.inbounds.apply_secret(keytag, secret)?;
        }
        Ok(res)
    }

    pub fn sync(
        &self,
        keytag: &Keytag,
        req: wire::sync::Request,
    ) -> Result<wire::sync::Response, Error> {
        self.inbounds.apply_sync(keytag, req.receipt)?;
        let receipt = self.inbounds.wire_receipt(keytag)?;
        Ok(wire::sync::Response { receipt })
    }

    pub fn receipts(&self) -> BTreeMap<Keytag, Option<WireReceipt>> {
        self.inbounds
            .keytags()
            .iter()
            .map(|keytag| (keytag.clone(), self.inbounds.wire_receipt(keytag).ok()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{receipt::Receipt, signer::Signer};
    use konduit_data::{ChequeBody, Duration, SquashBody, Tag};

    #[tokio::test]
    async fn peer_context_tracks_inbound_receipts_and_settles_local_paymes() {
        let client_signer = Signer::new(crate::signer::Config { key: [1; 32] });
        let client_key = client_signer.verifying_key();
        let tag = Tag::from(b"cln-peer-context-test".to_vec());
        let keytag = Keytag::from((&client_key, &tag));
        let peer = Ctx::init(
            inbounds::Config {
                inbounds: vec![crate::inbound::Config {
                    key: client_key,
                    tag: tag.clone(),
                    backing: (100_000_000, 0),
                }],
            },
            crate::Config {
                signer: crate::signer::Config { key: [2; 32] },
                channels: crate::channels::Config { channels: vec![] },
                paymes: crate::paymes::Config::default(),
                params: crate::ctx::Params::default(),
            },
        )
        .expect("peer context initializes");

        peer.sync(
            &keytag,
            wire::sync::Request {
                receipt: WireReceipt::from(Receipt::new(
                    client_signer.squash(tag.clone(), SquashBody::zero()),
                )),
            },
        )
        .expect("initial receipt seeds the inbound");

        let payme = peer
            .payme(wire::payme::Request { amount: 1_000_000 })
            .expect("local payme is created");
        let locked = client_signer.locked(
            tag,
            ChequeBody::new(
                1,
                payme.amount,
                crate::time::now() + Duration::from_secs(600),
                payme.lock,
            ),
        );
        let result = peer
            .commit(
                &keytag,
                wire::commit::Request {
                    inbound: locked.into_unverified(),
                    outbound: Some(wire::commit::Outbound {
                        key: payme.payee,
                        amount: payme.amount,
                    }),
                },
            )
            .await
            .expect("peer settles its own payme");
        assert!(result.secret.is_some());
    }
}

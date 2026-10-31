use std::collections::BTreeMap;

use crate::{
    receipt::WireReceipt,
    wire::{self, auth::Keytag},
};
use konduit_data::VerifyingKey;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("inbound: {0}")]
    Inbound(#[from] super::inbounds::Error),
    #[error("outbound: {0}")]
    Outbound(#[from] crate::ctx::Error),
}

pub struct Ctx {
    inbounds: super::Inbounds,
    outbounds: crate::Ctx,
}

impl Ctx {
    pub fn init(config: super::Config) -> Result<Self, Error> {
        let inbounds = super::Inbounds::new(config.inbound);
        let outbounds = crate::Ctx::init(config.outbound)?;
        Ok(Self {
            inbounds,
            outbounds,
        })
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

    /// Proactively reconciles our own outbound channels. Pure delegation.
    pub async fn sync_outbound(&self) -> Vec<(VerifyingKey, Result<(), Error>)> {
        self.outbounds
            .sync()
            .await
            .into_iter()
            .map(|(key, result)| (key, result.map_err(Error::from)))
            .collect()
    }

    /// Stateless w.r.t. Inbounds, but still gated: confirms `keytag`
    /// names a known sender before servicing, same as commit/sync.
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
        self.inbounds.apply_locked(keytag, req.inbound.clone())?;
        let res = self.outbounds.commit(req).await?;
        if let Some(secret) = res.secret {
            self.inbounds.apply_secret(keytag, secret)?;
        }
        Ok(res)
    }

    /// Serves the wire sync endpoint against whichever Inbounds entry
    /// `keytag` names.
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
            .map(|kt| (kt.clone(), self.inbounds.wire_receipt(kt).ok()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::standalone::{Config, config::ServerConfig, inbound, inbounds};
    use crate::{
        receipt::{Receipt, WireReceipt},
        signer::Signer,
    };
    use konduit_data::{ChequeBody, Duration, SquashBody, Tag};

    #[tokio::test]
    async fn local_payme_commits_through_the_standalone_client_path() {
        let client_signer = Signer::new(crate::signer::Config { key: [1; 32] });
        let client_key = client_signer.verifying_key();
        let server_key = [2; 32];
        let tag = Tag::from(b"cln-standalone-test".to_vec());
        let keytag = Keytag::from((&client_key, &tag));
        let initial_receipt = Receipt::new(client_signer.squash(tag.clone(), SquashBody::zero()));
        let config = Config {
            server: ServerConfig::default(),
            inbound: inbounds::Config {
                inbounds: vec![inbound::Config {
                    key: client_key,
                    tag: tag.clone(),
                    backing: (100_000_000, 0),
                }],
            },
            outbound: crate::Config {
                signer: crate::signer::Config { key: server_key },
                channels: crate::channels::Config { channels: vec![] },
                paymes: crate::paymes::Config::default(),
                params: crate::ctx::Params::default(),
            },
        };
        let ctx = Ctx::init(config).expect("standalone context initializes");
        ctx.sync(
            &keytag,
            wire::sync::Request {
                receipt: WireReceipt::from(initial_receipt),
            },
        )
        .expect("initial receipt seeds the inbound");

        let payme = ctx
            .payme(wire::payme::Request { amount: 1_000_000 })
            .expect("payme is created");
        assert_eq!(payme.payee, ctx.verifying_key());
        let body = ChequeBody::new(
            1,
            payme.amount,
            crate::time::now() + Duration::from_secs(600),
            payme.lock,
        );
        let locked = client_signer.locked(tag, body);
        let response = ctx
            .commit(
                &keytag,
                wire::commit::Request {
                    inbound: locked.into_unverified(),
                    outbound: None,
                },
            )
            .await
            .expect("local payme is settled");
        assert!(response.secret.is_some());
    }
}

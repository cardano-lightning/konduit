use std::collections::BTreeMap;

use crate::{
    Ctx as LibraryCtx,
    wire::{self, auth::Keytag},
};
use konduit_data::VerifyingKey;

pub type Error = crate::peer::Error;

/// Standalone process facade over the reusable CLN library peer context.
pub struct Ctx(LibraryCtx);

impl Ctx {
    pub fn init(config: super::Config) -> Result<Self, Error> {
        Ok(Self(LibraryCtx::init(config.inbound, config.outbound)?))
    }

    pub fn payme(&self, req: wire::payme::Request) -> Result<wire::payme::Response, Error> {
        self.0.payme(req)
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.0.verifying_key()
    }

    pub fn reveal(&self, lock: &konduit_data::Lock) -> Option<konduit_data::Secret> {
        self.0.reveal(lock)
    }

    pub async fn quote_outbound(
        &self,
        req: wire::quote::Request,
    ) -> Result<wire::quote::Response, Error> {
        self.0.quote_outbound(req).await
    }

    pub async fn commit_outbound(
        &self,
        req: wire::commit::Request,
    ) -> Result<wire::commit::Response, Error> {
        self.0.commit_outbound(req).await
    }

    pub async fn sync_outbound(&self) -> Vec<(VerifyingKey, Result<(), Error>)> {
        self.0.sync_outbound().await
    }

    pub async fn quote(
        &self,
        keytag: &Keytag,
        req: wire::quote::Request,
    ) -> Result<wire::quote::Response, Error> {
        self.0.quote(keytag, req).await
    }

    pub async fn commit(
        &self,
        keytag: &Keytag,
        req: wire::commit::Request,
    ) -> Result<wire::commit::Response, Error> {
        self.0.commit(keytag, req).await
    }

    pub fn sync(
        &self,
        keytag: &Keytag,
        req: wire::sync::Request,
    ) -> Result<wire::sync::Response, Error> {
        self.0.sync(keytag, req)
    }

    pub fn receipts(&self) -> BTreeMap<Keytag, Option<crate::receipt::WireReceipt>> {
        self.0.receipts()
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
                    outbound: Some(wire::commit::Outbound {
                        key: payme.payee,
                        amount: payme.amount,
                    }),
                },
            )
            .await
            .expect("local payme is settled");
        assert!(response.secret.is_some());
    }
}

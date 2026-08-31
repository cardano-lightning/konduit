use crate::wire::{self, auth::Keytag};
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
        if let Some(secret) = res.secret.clone() {
            self.inbounds.apply_secret(keytag, secret)?;
        }
        Ok(res)
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
}

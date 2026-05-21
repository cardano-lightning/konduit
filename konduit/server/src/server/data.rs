use crate::db;
use cardano_sdk::VerificationKey;
use konduit_api::endpoints::info;
/// Actix web server "Data" ie the context of handlers.
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Data {
    bln: Arc<dyn bln_client::Api + Send + Sync>,
    db: Arc<dyn db::Api + Send + Sync + 'static>,
    fx: Arc<RwLock<fx_client::State>>,
    info: Arc<info::Response>,
}

impl Data {
    pub fn new(
        bln: Arc<dyn bln_client::Api + Send + Sync>,
        db: Arc<dyn db::Api + Send + Sync + 'static>,
        fx: Arc<RwLock<fx_client::State>>,
        info: Arc<info::Response>,
    ) -> Self {
        Self { bln, db, fx, info }
    }

    pub fn fx(&self) -> Arc<tokio::sync::RwLock<fx_client::State>> {
        self.fx.clone()
    }

    pub fn db(&self) -> Arc<dyn db::Api + Send + Sync + 'static> {
        self.db.clone()
    }

    pub fn bln(&self) -> Arc<dyn bln_client::Api + Send + Sync + 'static> {
        self.bln.clone()
    }

    pub fn info(&self) -> Arc<info::Response> {
        self.info.clone()
    }

    /// The adaptor's Ed25519 verification key, used to verify PoP auth signatures.
    pub fn adapter_key(&self) -> VerificationKey {
        self.info.channel_parameters.adaptor_key
    }
}

use cobbl3::HmacKey;
use konduit_wire::info;
use std::sync::Arc;

use crate::Db;
// use tokio::sync::RwLock;

pub struct State {
    /// Cobbl3/HMAC key for signing tokens.
    cobbl3_key: Arc<HmacKey>,
    // bln: Arc<dyn bln_client::Api + Send + Sync>,
    db: Arc<Db>,
    // fx: Arc<RwLock<fx_client::State>>,
    info: Arc<info::Response>,
    // admin: Arc<dyn admin::SyncApi + Send + Sync + 'static>,
}

impl State {
    pub fn new(
        cobbl3_key: Arc<HmacKey>,
        // bln: Arc<dyn bln_client::Api + Send + Sync>,
        db: Arc<Db>,
        // fx: Arc<RwLock<fx_client::State>>,
        info: Arc<info::Response>,
        // admin: Arc<dyn admin::SyncApi + Send + Sync + 'static>,
    ) -> Self {
        Self {
            cobbl3_key,
            db,
            info,
        }
    }

    pub fn cobbl3_key(&self) -> Arc<HmacKey> {
        self.cobbl3_key.clone()
    }

    // pub fn fx(&self) -> Arc<tokio::sync::RwLock<fx_client::State>> {
    //     self.fx.clone()
    // }

    pub fn db(&self) -> Arc<Db> {
        self.db.clone()
    }

    // pub fn bln(&self) -> Arc<dyn bln_client::Api + Send + Sync + 'static> {
    //     self.bln.clone()
    // }

    pub fn info(&self) -> Arc<info::Response> {
        self.info.clone()
    }

    // pub fn admin(&self) -> Arc<dyn admin::SyncApi + Send + Sync + 'static> {
    //     self.admin.clone()
    // }
}

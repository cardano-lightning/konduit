use konduit_wire::info;
use std::sync::Arc;
// use tokio::sync::RwLock;

pub struct State {
    // bln: Arc<dyn bln_client::Api + Send + Sync>,
    // db: Arc<dyn db::Api + Send + Sync + 'static>,
    // fx: Arc<RwLock<fx_client::State>>,
    info: Arc<info::Response>,
    // admin: Arc<dyn admin::SyncApi + Send + Sync + 'static>,
}

impl State {
    pub fn new(
        // bln: Arc<dyn bln_client::Api + Send + Sync>,
        // db: Arc<dyn db::Api + Send + Sync + 'static>,
        // fx: Arc<RwLock<fx_client::State>>,
        info: Arc<info::Response>,
        // admin: Arc<dyn admin::SyncApi + Send + Sync + 'static>,
    ) -> Self {
        Self { info }
    }

    // pub fn fx(&self) -> Arc<tokio::sync::RwLock<fx_client::State>> {
    //     self.fx.clone()
    // }

    // pub fn db(&self) -> Arc<dyn db::Api + Send + Sync + 'static> {
    //     self.db.clone()
    // }

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

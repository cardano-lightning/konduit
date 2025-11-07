use cardano_connect_blockfrost::Blockfrost;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{bln, db, fx, info};

pub struct AppState {
    pub info: Arc<info::Info>,
    pub db: Arc<dyn db::DbInterface + Send + Sync + 'static>,
    pub bln: Arc<dyn bln::BlnInterface + Send + Sync>,
    pub fx: Arc<RwLock<Option<fx::Fx>>>,
    // TODO: Not sure how hard it would be to turn CardanoConnect into a dyn compatible trait
    // object. For now we use Blockfrost directly. In the future we can either share
    // share the object or pass custom config of the connector via AppState.
    pub connector: Arc<Blockfrost>,
}

impl AppState {
    pub fn new<Db, Bln>(
        info: info::Info,
        db: Db,
        bln: Bln,
        fx: Option<fx::Fx>,
        connector: Blockfrost,
    ) -> Self
    where
        Db: db::DbInterface + Send + Sync + 'static,
        Bln: bln::BlnInterface + Send + Sync + 'static,
    {
        AppState {
            info: Arc::new(info),
            db: Arc::new(db),
            bln: Arc::new(bln),
            fx: Arc::new(RwLock::new(fx)),
            connector: Arc::new(connector),
        }
    }
}

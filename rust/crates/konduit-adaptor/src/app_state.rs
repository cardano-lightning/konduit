use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{bln, db, fx, info};

pub struct AppState {
    pub info: info::Info,
    pub db: Arc<dyn db::DbInterface + Send + Sync>,
    pub bln: Arc<dyn bln::BlnInterface + Send + Sync>,
    pub fx: Arc<RwLock<Option<fx::Fx>>>,
}

impl AppState {
    pub fn new<Db, Bln>(info: info::Info, db: Db, bln: Bln, fx: Option<fx::Fx>) -> Self
    where
        Db: db::DbInterface + Send + Sync + 'static,
        Bln: bln::BlnInterface + Send + Sync + 'static,
    {
        AppState {
            info,
            db: Arc::new(db),
            bln: Arc::new(bln),
            fx: Arc::new(RwLock::new(fx)),
        }
    }
}

/// Actix web server "Data" ie the context of handlers.
use cardano_connect_blockfrost::Blockfrost;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{cardano, db, info};

pub struct Data {
    bln: Arc<dyn bln_client::Api + Send + Sync>,
    cardano: Arc<cardano::Cardano>,
    db: Arc<dyn db::Api + Send + Sync + 'static>,
    fx: Arc<RwLock<fx_client::State>>,
    info: Arc<info::Info>,
}

impl Data {
    pub fn new(
        bln: Arc<dyn bln_client::Api + Send + Sync>,
        cardano: Arc<cardano::Cardano>,
        db: Arc<dyn db::Api + Send + Sync + 'static>,
        fx: Arc<RwLock<fx_client::State>>,
        info: Arc<info::Info>,
    ) -> Self {
        Self {
            bln,
            cardano,
            db,
            fx,
            info,
        }
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

    pub fn cardano(&self) -> Arc<Blockfrost> {
        self.cardano.clone()
    }

    pub fn info(&self) -> Arc<crate::info::Info> {
        self.info.clone()
    }
}

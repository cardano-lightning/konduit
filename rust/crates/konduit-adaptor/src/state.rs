use cardano_connect_blockfrost::Blockfrost;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{Args, bln, cardano, db, fx, info};

pub struct State {
    info: Arc<info::Info>,
    db: Arc<dyn db::DbInterface + Send + Sync + 'static>,
    bln: Arc<dyn bln::BlnInterface + Send + Sync>,
    fx: Arc<RwLock<Option<fx::Fx>>>,
    // TODO: Not sure how hard it would be to turn CardanoConnect into a dyn compatible trait
    // object. For now we use Blockfrost directly. In the future we can either share
    // share the object or pass custom config of the connector via State.
    cardano: Arc<Blockfrost>,
}

impl State {
    pub fn new<Db, Bln>(
        info: info::Info,
        db: Db,
        bln: Bln,
        fx: Option<fx::Fx>,
        cardano: Blockfrost,
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
            cardano: Arc::new(cardano),
        }
    }

    pub fn fx(&self) -> Arc<tokio::sync::RwLock<Option<crate::Fx>>> {
        self.fx.clone()
    }

    pub fn db(&self) -> Arc<dyn db::DbInterface + Send + Sync + 'static> {
        self.db.clone()
    }

    pub fn cardano(&self) -> Arc<Blockfrost> {
        self.cardano.clone()
    }

    pub fn info(&self) -> Arc<crate::info::Info> {
        self.info.clone()
    }

    pub async fn from_args(args: &Args) -> Self {
        let db = args.db.build().expect("Failed to open database");
        let bln = args.bln.build().expect("Failed to setup bln");
        let cardano = {
            let res = cardano::new().await;
            res.map_err(|e| CliError::BlockfrostInitFailed(e.to_string()))?
        };
        let info = args.info.clone();
        Ok(Self {
            db,
            bln,
            fx,
            cardano,
        })
    }
}

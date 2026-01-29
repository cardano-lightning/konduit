use cardano_connect_blockfrost::Blockfrost;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{args::Args, bln, cardano, db, fx, info};

pub struct State {
    info: Arc<info::Info>,
    db: Arc<dyn db::Api + Send + Sync + 'static>,
    bln: Arc<dyn bln::Api + Send + Sync>,
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
        Db: db::Api + Send + Sync + 'static,
        Bln: bln::Api + Send + Sync + 'static,
    {
        Self {
            info: Arc::new(info),
            db: Arc::new(db),
            bln: Arc::new(bln),
            fx: Arc::new(RwLock::new(fx)),
            cardano: Arc::new(cardano),
        }
    }

    pub fn fx(&self) -> Arc<tokio::sync::RwLock<Option<fx::Fx>>> {
        self.fx.clone()
    }

    pub fn db(&self) -> Arc<dyn db::Api + Send + Sync + 'static> {
        self.db.clone()
    }

    pub fn bln(&self) -> Arc<dyn bln::Api + Send + Sync + 'static> {
        self.bln.clone()
    }

    pub fn cardano(&self) -> Arc<Blockfrost> {
        self.cardano.clone()
    }

    pub fn info(&self) -> Arc<crate::info::Info> {
        self.info.clone()
    }

    pub async fn from_args(args: &Args) -> Self {
        let db = args.db.build().expect("Failed to setup database");
        let bln = args.bln.build().expect("Failed to setup bln");
        let fx = args.fx.build().expect("Failed to setup fx");
        let cardano = cardano::new()
            .await
            .expect("Failed to setup cardano connect");
        let info = info::Info::from_args(args.common);
        Ok(Self {
            db,
            bln,
            fx,
            cardano,
            info,
        })
    }
}

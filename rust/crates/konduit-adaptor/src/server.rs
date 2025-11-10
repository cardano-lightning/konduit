use crate::connector;
use crate::keytag_middleware::KeytagAuth;
use crate::{app_state::AppState, Cmd};
use crate::{db, handlers};
use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpServer};
use cardano_connect_blockfrost::Blockfrost;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum CliError {
    BadConfig,
    BlockfrostInitFailed(String),
}

pub struct Server {
    app_state: AppState,
    bind_address: String,
}

impl Server {
    pub fn fx(&self) -> Arc<tokio::sync::RwLock<Option<crate::Fx>>> {
        self.app_state.fx.clone()
    }

    pub fn db(&self) -> Arc<dyn db::DbInterface + Send + Sync + 'static> {
        self.app_state.db.clone()
    }

    pub fn connector(&self) -> Arc<Blockfrost> {
        self.app_state.connector.clone()
    }

    pub fn info(&self) -> Arc<crate::info::Info> {
        self.app_state.info.clone()
    }

    pub async fn from_cmd(cmd: Cmd) -> Result<Self, CliError> {
        let bind_address = format!("{}:{}", cmd.host.host, cmd.host.port);
        let db = cmd.db.build().expect("Failed to open database");
        let bln = cmd.bln.build().expect("Failed to setup bln");
        let connector = {
            let res = connector::new().await;
            res.map_err(|e| CliError::BlockfrostInitFailed(e.to_string()))?
        };
        let info = cmd.info.clone();
        let app_state = AppState::new(info, db, bln, None, connector);
        Ok(Self {
            app_state,
            bind_address,
        })
    }

    pub async fn run(self) -> std::io::Result<()> {
        // FIXME :: Handle error
        let app_state = web::Data::new(self.app_state);
        log::info!("Starting server on http://{}...", self.bind_address);
        HttpServer::new(move || {
            App::new()
                .wrap(Logger::default())
                .wrap(
                    Cors::default()
                        .allow_any_origin()
                        .allow_any_method()
                        .allow_any_header(),
                )
                .app_data(app_state.clone())
                .route("/info", web::get().to(handlers::info))
                .service(
                    // FIXME : Implement auth
                    web::scope("/ch")
                        .wrap(KeytagAuth::new("KONDUIT"))
                        .route("/squash", web::post().to(handlers::squash))
                        .route("/quote", web::post().to(handlers::quote)), // .route("/pay", web::post().to(handlers::pay))
                )
                .service(web::scope("/opt").route("/fx", web::get().to(handlers::fx)))
                .service(
                    // THIS SHOULD BE EXPOSED ONLY TO TRUSTED SOURCES.
                    web::scope("/admin")
                        .route("/tip", web::post().to(handlers::tip))
                        .route("/show", web::get().to(handlers::show)),
                )
        })
        .bind(self.bind_address)?
        .run()
        .await
    }
}

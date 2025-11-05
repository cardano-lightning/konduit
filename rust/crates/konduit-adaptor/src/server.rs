use crate::handlers;
use crate::keytag_middleware::KeytagAuth;
use crate::{Cmd, app_state::AppState};
use actix_web::{App, HttpServer, middleware::Logger, web};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum CliError {
    BadConfig,
}

pub struct Server {
    app_state: AppState,
    bind_address: String,
}

impl Server {
    pub fn fx(&self) -> Arc<tokio::sync::RwLock<Option<crate::Fx>>> {
        self.app_state.fx.clone()
    }

    pub async fn from_cmd(cmd: Cmd) -> Result<Self, CliError> {
        let bind_address = format!("{}:{}", cmd.host.host, cmd.host.port);
        let info = cmd.info;
        let db = cmd.db.build().expect("Failed to open database");
        let bln = cmd.bln.build().expect("Failed to setup bln");
        let app_state = AppState::new(info, db, bln, None);
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
                .app_data(app_state.clone())
                .route("/info", web::get().to(handlers::info))
                .service(
                    // FIXME : Implement auth
                    web::scope("/ch")
                        .wrap(KeytagAuth::new("KONDUIT"))
                        .route("/squash", web::post().to(handlers::squash))
                        .route("/quote", web::post().to(handlers::quote))
                        // .route("/pay", web::post().to(handlers::pay))
                        .route("/fx", web::get().to(handlers::fx)),
                )
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

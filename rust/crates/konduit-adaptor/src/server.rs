<<<<<<< HEAD
use crate::config::AppState;
use crate::db::DbInterface;
use crate::env;
use crate::handlers::{constants, pay, quote, squash};
use actix_web::{App, HttpServer, middleware::Logger, web};
use std::sync::Arc; // Import the trait

/// Configures and starts the Actix Web server.
/// Now takes the `DbInterface` trait object.
pub async fn run(
    db: Arc<dyn DbInterface + Send + Sync>,
    bind_address: String,
) -> std::io::Result<()> {
    // Create shared state using the trait object
    let app_state = web::Data::new(AppState { db });

    log::info!("Starting server on http://{}...", bind_address);

    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default()) // Add logging middleware
            .app_data(app_state.clone()) // Share the database state
            .service(web::resource("/constants").route(web::get().to(constants)))
            .service(web::resource("/quote").route(web::post().to(quote)))
            .service(web::resource("/pay").route(web::post().to(pay)))
            .service(web::resource("/squash").route(web::post().to(squash)))
    })
    .bind(bind_address)?
    .run()
    .await
}

pub async fn init_on_new(db: &impl DbInterface) -> Result<(), std::io::Error> {
    match db.get_constants().await {
        Err(_) => {
            let constants = env::constants()?;
            db.init(&constants).await.unwrap();
            Ok(())
        }
        Ok(_) => Ok(()),
=======
use crate::keytag_middleware::KeytagAuth;
use crate::{Cmd, app_state::AppState};
use crate::{bln, db, fx, handlers};
use actix_web::{App, HttpServer, middleware::Logger, web};

#[derive(Debug, Clone)]
pub enum CliError {
    BadConfig,
}

pub struct Server {
    app_state: AppState,
    bind_address: String,
}

impl Server {
    pub async fn from_cmd(cmd: Cmd) -> Result<Self, CliError> {
        let bind_address = format!("{}:{}", cmd.host.host, cmd.host.port);
        let info = cmd.info;
        let db = cmd.db.into().expect("Failed to open database");
        let bln = cmd.bln.into().expect("Failed to setup bln");
        let fx = cmd.fx.into();
        // Bln
        // let macaroon = hex::decode(cmd.lnd_macaroon).unwrap();
        // , None, &macaroon).unwrap();

        // FX
        // let fx = Arc::new(RwLock::new(None));
        // tokio::spawn(cron_fx(15 * 60, fx.clone()));
        // APP
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
>>>>>>> e3cb13e (Updates to konduit data.)
    }
}

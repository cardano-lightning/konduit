use crate::{args::Args, state::State};
use actix_cors::Cors;
use actix_web::{App, HttpServer, middleware::Logger, web};

mod args;
pub use args::Args as ServerArgs;

mod cbor;
mod handlers;
mod middleware;

#[derive(Debug, Clone)]
pub enum CliError {
    BadConfig,
    BlockfrostInitFailed(String),
}

pub struct Server {
    state: State,
    bind_address: String,
}

impl Server {
    pub fn new(state: State, host: String, port: Option<u16>) -> Self {
        let bind_address = format!("{}:{}", host, port);
        Self {
            state,
            bind_address,
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub async fn run(self) -> std::io::Result<()> {
        // FIXME :: Handle error
        let state = web::Data::new(self.state);
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
                .app_data(state.clone())
                .route("/info", web::get().to(handlers::info))
                .service(
                    // FIXME : Implement auth
                    web::scope("/ch")
                        .wrap(middleware::KeytagAuth::new("KONDUIT"))
                        .route("/squash", web::post().to(handlers::squash))
                        .route("/quote", web::post().to(handlers::quote))
                        .route("/pay", web::post().to(handlers::pay)),
                )
                .service(web::scope("/opt").route("/fx", web::get().to(handlers::fx)))
                .service(
                    // THIS SHOULD BE EXPOSED ONLY TO TRUSTED SOURCES.
                    web::scope("/admin").route("/show", web::get().to(handlers::show)),
                )
        })
        .bind(self.bind_address)?
        .run()
        .await
    }
}

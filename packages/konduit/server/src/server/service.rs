use actix_cors::Cors;
use actix_web::{
    App, HttpServer,
    middleware::{self, Logger},
    web,
};

use crate::server::{Data, auth, handlers, mediation};

pub struct Service {
    data: Data,
    bind_address: String,
    session_check_url: String,
}

impl Service {
    pub fn new(args: super::Args, data: super::Data) -> Self {
        let bind_address = format!("{}:{:?}", args.host, args.port);
        Self {
            data,
            bind_address,
            session_check_url: args.session_check_url,
        }
    }

    pub fn data(&self) -> &Data {
        &self.data
    }

    pub async fn run(self) -> std::io::Result<()> {
        let data = web::Data::new(self.data);
        let session_check_url = self.session_check_url;
        log::info!("Starting server on http://{}...", self.bind_address);
        HttpServer::new(move || {
            let session_check_url = session_check_url.clone();
            App::new()
                .wrap(Logger::default())
                .wrap(
                    Cors::default()
                        .allow_any_origin()
                        .allow_any_method()
                        .allow_any_header(),
                )
                .app_data(data.clone())
                .wrap(middleware::from_fn(mediation::content_negotiation))
                .route("/info", web::get().to(handlers::info))
                .service(
                    web::scope("/ch")
                        .wrap(middleware::from_fn(auth::no_auth))
                        .route("/receipt", web::get().to(handlers::receipt))
                        .service(
                            web::resource("/squash")
                                .wrap(auth::LeaseAuth::new(session_check_url.clone()))
                                .route(web::post().to(handlers::squash)),
                        )
                        .service(
                            web::resource("/quote")
                                .wrap(auth::LeaseAuth::new(session_check_url.clone()))
                                .route(web::post().to(handlers::quote)),
                        )
                        .service(
                            web::resource("/pay")
                                .wrap(auth::LeaseAuth::new(session_check_url.clone()))
                                .route(web::post().to(handlers::pay)),
                        ),
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

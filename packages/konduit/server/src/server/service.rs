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
}

impl Service {
    pub fn new(args: super::Args, data: super::Data) -> Self {
        let bind_address = format!("{}:{:?}", args.host, args.port);
        Self { data, bind_address }
    }

    pub fn data(&self) -> &Data {
        &self.data
    }

    pub async fn run(self) -> std::io::Result<()> {
        let db = self.data.db();
        let data = web::Data::new(self.data);
        log::info!("Starting server on http://{}...", self.bind_address);
        HttpServer::new(move || {
            let db = db.clone();
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
                .route("/session/claim", web::post().to(handlers::claim_session))
                .service(
                    web::scope("/ch")
                        .wrap(middleware::from_fn(auth::no_auth))
                        .route("/receipt", web::get().to(handlers::receipt))
                        .service(
                            web::resource("/squash")
                                .wrap(auth::LeaseAuth::new(db.clone()))
                                .route(web::post().to(handlers::squash)),
                        )
                        .service(
                            web::resource("/quote")
                                .wrap(auth::LeaseAuth::new(db.clone()))
                                .route(web::post().to(handlers::quote)),
                        )
                        .service(
                            web::resource("/pay")
                                .wrap(auth::LeaseAuth::new(db.clone()))
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

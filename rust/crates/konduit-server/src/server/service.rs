use actix_cors::Cors;
use actix_web::{App, HttpServer, middleware::Logger, web};

use crate::server::{Data, handlers, middleware};

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
        // FIXME :: Handle error
        let data = web::Data::new(self.data);
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
                .app_data(data.clone())
                .route("/info", web::get().to(handlers::info))
                .service(
                    // FIXME : Implement auth
                    web::scope("/ch")
                        .wrap(middleware::KeytagAuth::new("KONDUIT"))
                        .route("/squash", web::get().to(handlers::get_squash))
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

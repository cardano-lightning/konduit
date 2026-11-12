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
        // FIXME :: Handle error
        let data = web::Data::new(self.data);
        tracing::info!("Starting server on http://{}...", self.bind_address);
        HttpServer::new(move || {
            App::new()
                .wrap(Logger::default().exclude("/info"))
                .wrap(
                    Cors::default()
                        .allow_any_origin()
                        .allow_any_method()
                        .allow_any_header(),
                )
                .app_data(data.clone())
                .wrap(middleware::from_fn(mediation::content_negotiation))
                .route("/info", web::get().to(handlers::info))
                // An invoice is asked for by whoever wants to pay, who may
                // hold no channel here yet, so it carries no keytag.
                .route("/pay/konduit/invoice", web::post().to(handlers::invoice))
                .service(
                    // Everything a consumer or a routing peer does,
                    // authenticated per channel by keytag. The rail is
                    // in the path, and `konduit` is reserved for what no
                    // rail owns.
                    web::scope("/pay")
                        .wrap(middleware::from_fn(auth::keytag_auth))
                        .route("/konduit/receipt", web::get().to(handlers::receipt))
                        .route("/konduit/squash", web::get().to(handlers::squash_status))
                        .route("/konduit/squash", web::post().to(handlers::squash))
                        .route("/{rail}/quote", web::post().to(handlers::quote))
                        .route("/{rail}/pay", web::post().to(handlers::pay)),
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

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
        let adapter_key = self.data.adapter_key();
        let hmac_key = *self.data.hmac_key();
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
                .route("/version", web::get().to(handlers::version))
                .route("/info", web::get().to(handlers::info))
                // Token issuance: client exchanges a signed proof for a session token.
                .route("/auth", web::post().to(handlers::issue_token))
                .service(
                    web::scope("/channel")
                        .wrap(middleware::HmacToken::new(hmac_key, adapter_key))
                        .route("/sync", web::get().to(handlers::sync))
                        .route("/squash", web::post().to(handlers::squash))
                        .route("/pay/quoted", web::post().to(handlers::pay_quoted))
                        .route("/quote/bolt11", web::post().to(handlers::quote_bolt11)),
                )
        })
        .bind(self.bind_address)?
        .run()
        .await
    }
}

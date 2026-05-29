use std::sync::Arc;

use actix_web::{App, HttpMessage, HttpRequest, HttpServer, dev::Service, web};
use cardano_sdk::Hash;
use konduit_server::{Never, State, ToMedia, get_media_type, handlers, pick_media_type};

async fn info(req: HttpRequest, state: web::Data<State>) -> actix_web::HttpResponse {
    Ok::<_, Never>(handlers::info(&state).await)
        .to_media(&get_media_type(&req))
        .into()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let state = web::Data::new(State::new(Arc::new(sample_response())));

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap_fn(|req, srv| {
                req.extensions_mut().insert(pick_media_type(&req));
                srv.call(req)
            })
            .route("/info", web::get().to(info))
        // .service(
        //     web::scope("/channel")
        //         .wrap(AuthMiddleware)
        //         .route("/info", web::get().to(channel_info))
        // )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

fn sample_response() -> konduit_wire::info::Response {
    konduit_wire::info::Response {
        tos: konduit_wire::info::TosInfo {
            flat_fee: 1_000_000, // 1 ADA in lovelace
        },
        channel_parameters: konduit_wire::info::ChannelParameters {
            adaptor_key: konduit_data::VerifyingKey::from([0xabu8; 32]),
            close_period: konduit_data::Duration::from_secs(3600),
            tag_length: 8,
        },
        tx_help: konduit_wire::info::TxHelp {
            host_address: "addr_test1qqdxeujed4f77u82rslna9gtsrwnxqww8f3zxz8w4uz87vwy68rhqmmahtetcv2wcvsqe0ct9h6gmd8g5nsuw38rq4sqvh9dvw"
                .parse()
                .expect("valid address"),
            validator: Hash::from([0x01; 28]),
        },
    }
}

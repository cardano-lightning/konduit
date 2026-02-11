use actix_web::{App, test, web};
use cardano_tx_builder::{SigningKey, address_test, key_credential};
use konduit_adaptor::*;
use konduit_data::Duration;
use std::sync::Arc;
use tokio::sync::RwLock;

// Assuming you have a mock implementation for your traits
// If you don't have mocks, you can use the 'mockall' crate or
// real ephemeral sled instances for the DB.

#[actix_web::test]
async fn test_handler_scenario() {
    let bln = Arc::new(bln_client::mock::Client::new());
    let db = Arc::new(db::with_sled::WithSled::open_temporary().unwrap());
    let fx = Arc::new(RwLock::new(fx_client::State::new(
        fx_client::BaseCurrency::Eur,
        0.1,
        100000.0,
    )));
    let common_args = common::Args {
        signing_key: SigningKey::from([0; 32]),
        close_period: Duration::from_secs(1),
        fee: 1000,
        tag_length: 32,
        host_address: address_test!(
            key_credential!("bd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb777"),
            key_credential!("bd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb777"),
        ),
    };
    let info = Arc::new(info::Info::from_args(&common_args));

    // 2. Initialize the Data struct
    let data = server::Data::new(bln, db, fx, info);

    // 3. Init the App with the specific handler
    let app = test::init_service(
        App::new()
            .app_data(data)
            .route("/your-endpoint", web::get().to(server::handlers::info)),
    )
    .await;

    // 4. Create a request and execute it
    let req = test::TestRequest::get().uri("/your-endpoint").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());
}

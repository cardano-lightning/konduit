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
    }
}

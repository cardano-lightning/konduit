use actix_web::{App, HttpResponse, HttpServer, ResponseError, middleware::from_fn, web};
use clap::{Parser, Subcommand};
use cln_server::{
    mock::{self, Ctx, init_config, keytag::auth, load_config},
    wire::{self, auth::Keytag},
};
use std::path::PathBuf;

struct ApiError(mock::ctx::Error);
impl From<mock::ctx::Error> for ApiError {
    fn from(err: mock::ctx::Error) -> Self {
        Self(err)
    }
}
impl std::fmt::Debug for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        log::warn!("{}", self.0);
        HttpResponse::BadRequest().json(serde_json::json!({ "error": self.0.to_string() }))
    }
}
type ApiResult<T> = Result<web::Json<T>, ApiError>;

#[derive(Parser)]
#[command(name = "cln-server")]
struct Cli {
    #[arg(short, long, default_value = "cln-server-config.toml")]
    config: PathBuf,
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Init {
        #[arg(long)]
        force: bool,
    },
    Run {
        #[arg(long, default_value = "127.0.0.1:2567")]
        listen: String,
        #[arg(long, default_value_t = 30)]
        sync_interval_secs: u64,
    },
}

async fn healthz() -> HttpResponse {
    HttpResponse::Ok().finish()
}

async fn payme(
    ctx: web::Data<Ctx>,
    req: web::Json<wire::payme::Request>,
) -> ApiResult<wire::payme::Response> {
    Ok(web::Json(ctx.payme(req.into_inner())?))
}

async fn quote(
    ctx: web::Data<Ctx>,
    keytag: Keytag,
    req: web::Json<wire::quote::Request>,
) -> ApiResult<wire::quote::Response> {
    Ok(web::Json(ctx.quote(&keytag, req.into_inner()).await?))
}

async fn commit(
    ctx: web::Data<Ctx>,
    keytag: Keytag,
    req: web::Json<wire::commit::Request>,
) -> ApiResult<wire::commit::Response> {
    Ok(web::Json(ctx.commit(&keytag, req.into_inner()).await?))
}

async fn sync(
    ctx: web::Data<Ctx>,
    keytag: Keytag,
    req: web::Json<wire::sync::Request>,
) -> ApiResult<wire::sync::Response> {
    Ok(web::Json(ctx.sync(&keytag, req.into_inner())?))
}

fn spawn_periodic_sync(ctx: web::Data<Ctx>, interval: std::time::Duration) {
    actix_web::rt::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            for (key, result) in ctx.sync_outbound().await {
                if let Err(err) = result {
                    log::warn!("sync failed for {} {err}", hex::encode(key.as_ref()));
                }
            }
        }
    });
}

async fn run(config: PathBuf, listen: String, sync_interval_secs: u64) -> anyhow::Result<()> {
    let ctx = web::Data::new(Ctx::init(load_config(&config)?).map_err(anyhow::Error::from)?);
    spawn_periodic_sync(
        ctx.clone(),
        std::time::Duration::from_secs(sync_interval_secs),
    );
    log::info!("listening on {listen}");
    HttpServer::new(move || {
        App::new()
            .app_data(ctx.clone())
            .route("/healthz", web::get().to(healthz))
            .route(wire::payme::PATH, web::post().to(payme))
            .service(
                web::scope("")
                    .wrap(from_fn(auth))
                    .route(wire::quote::PATH, web::post().to(quote))
                    .route(wire::commit::PATH, web::post().to(commit))
                    .route(wire::sync::PATH, web::post().to(sync)),
            )
    })
    .bind(&listen)?
    .run()
    .await?;
    Ok(())
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    match cli.command {
        Cmd::Init { force } => init_config(&cli.config, force),
        Cmd::Run {
            listen,
            sync_interval_secs,
        } => run(cli.config, listen, sync_interval_secs).await,
    }
}

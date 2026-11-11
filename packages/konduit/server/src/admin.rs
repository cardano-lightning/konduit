use async_trait::async_trait;

mod args;
pub use args::AdminArgs as Args;

mod config;
pub use config::Config;

mod service;
pub use service::Service;

#[allow(clippy::double_must_use)] // async_trait adds must_use to its generated Future return type.
#[async_trait(?Send)]
pub trait SyncApi: Send + Sync {
    async fn sync(&self) -> Result<(), anyhow::Error>;
}

use async_trait::async_trait;

mod args;
pub use args::AdminArgs as Args;

mod config;
pub use config::Config;

mod service;
pub use service::Service;

#[async_trait(?Send)]
pub trait SyncApi: Send + Sync {
    async fn sync(&self) -> Result<(), anyhow::Error>;
}

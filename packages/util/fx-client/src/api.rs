use async_trait::async_trait;

#[allow(clippy::double_must_use)] // async_trait adds must_use to its generated Future return type.
#[async_trait]
pub trait Api: Send + Sync {
    async fn get(&self) -> Result<super::State, super::Error>;
}

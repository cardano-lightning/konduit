use async_trait::async_trait;

#[async_trait]
pub trait Api: Send + Sync {
    async fn get(&self) -> Result<super::State, super::Error>;
}

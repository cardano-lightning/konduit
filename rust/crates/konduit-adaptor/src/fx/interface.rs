use async_trait::async_trait;

#[async_trait]
pub trait FxInterface: Send + Sync {
    async fn get(&self) -> Result<super::Fx, super::Error>;
}

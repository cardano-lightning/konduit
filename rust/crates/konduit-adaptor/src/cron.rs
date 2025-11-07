use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::interval;

pub fn cron<F, Fut, T>(update_fn: F, period: Duration) -> JoinHandle<()>
where
    F: Fn() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Option<T>> + Send + 'static,
    T: Send + Sync + 'static,
{
    tokio::spawn(async move {
        let mut ticker = interval(period);

        loop {
            ticker.tick().await;
            update_fn().await;
            
        }
    })
}

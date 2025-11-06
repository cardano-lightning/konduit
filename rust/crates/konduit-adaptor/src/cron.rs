use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::time::interval;

pub fn cron<F, Fut, T>(
    data_lock: Arc<RwLock<Option<T>>>,
    update_fn: F,
    period: Duration,
) -> JoinHandle<()>
where
    F: Fn() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Option<T>> + Send + 'static,
    T: Send + Sync + 'static,
{
    tokio::spawn(async move {
        let mut ticker = interval(period);

        loop {
            ticker.tick().await;
            let new_value = update_fn().await;
            let mut data_guard = data_lock.write().await;
            *data_guard = new_value;
        }
    })
}

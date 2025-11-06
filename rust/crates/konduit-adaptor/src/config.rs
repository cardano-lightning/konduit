use std::sync::Arc;

use crate::db::DbInterface;

pub struct AppState {
    pub db: Arc<dyn DbInterface + Send + Sync>,
}

use rusqlite::Connection;

/// Storage abstraction for the konduit indexer.
///
/// Implementations are expected to be cheaply `Clone` (or wrap an `Arc`)
/// since query operations typically borrow the underlying connection.
pub trait Store {
    /// Borrow the underlying SQLite connection.
    fn connection(&self) -> &Connection;
}

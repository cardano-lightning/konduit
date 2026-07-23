use konduit_data::{Keytag, Receipt};
use minicbor::{Decode, Encode};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};

use crate::channel::{self, Aux, Channel, Retainer};

mod args;
pub use args::DbArgs as Args;

const TABLE: TableDefinition<&[u8], Value> = TableDefinition::new("channels");

// ---------------------------------------------------------------------------
// Value
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Encode, Decode)]
pub struct Value {
    #[n(0)]
    retainer: Option<Retainer>,
    #[n(1)]
    receipt: Option<Receipt>,
    #[n(2)]
    aux: Aux,
}

impl redb::Value for Value {
    type SelfType<'a> = Value;
    type AsBytes<'a> = Vec<u8>;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        minicbor::decode::<Value>(data).expect("corrupt Entry bytes")
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        minicbor::to_vec(&value).expect("Entry encode failed")
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("Entry")
    }
}

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid key length: {0} (expected 32–64)")]
    InvalidKey(usize),

    #[error("entry already exists")]
    AlreadyExists,

    #[error("entry not found")]
    NotFound,

    #[error("transaction conflict")]
    Contended,

    #[error("channel: {0}")]
    Channel(#[from] channel::ChannelError),

    #[error("backend: {0}")]
    Database(#[from] redb::DatabaseError),

    #[error("backend: {0}")]
    Transaction(#[from] redb::TransactionError),

    #[error("backend: {0}")]
    Table(#[from] redb::TableError),

    #[error("backend: {0}")]
    Storage(#[from] redb::StorageError),

    #[error("backend: {0}")]
    Commit(#[from] redb::CommitError),
}

// ---------------------------------------------------------------------------
// Db
// ---------------------------------------------------------------------------

pub struct Db(Database);

impl Db {
    pub fn open(path: &str) -> Result<Self, Error> {
        Ok(Self(Database::create(path)?))
    }

    /// Fetch a channel by key.
    pub fn get(&self, keytag: &Keytag) -> Result<Option<Channel>, Error> {
        let tx = self.0.begin_read()?;
        let table = tx.open_table(TABLE)?;
        Ok(table.get(keytag.as_ref())?.map(|v| v.value().into_inner()))
    }

    /// Insert a new channel. Errors if the keytag. already exists.
    pub fn insert(&self, keytag: &Keytag, channel: Channel) -> Result<(), Error> {
        let tx = self.0.begin_write()?;
        {
            let mut table = tx.open_table(TABLE)?;
            if table.get(keytag.as_ref())?.is_some() {
                return Err(Error::AlreadyExists);
            }
            table.insert(keytag.as_ref(), Value(channel))?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Remove a channel by key. Errors if the key does not exist.
    pub fn remove(&self, keytag: &Keytag) -> Result<(), Error> {
        let tx = self.0.begin_write()?;
        {
            let mut table = tx.open_table(TABLE)?;
            if table.remove(keytag.as_ref())?.is_none() {
                return Err(Error::NotFound);
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Iterate all channels.
    pub fn iter(&self) -> Result<Vec<(Vec<u8>, Channel)>, Error> {
        let tx = self.0.begin_read()?;
        let table = tx.open_table(TABLE)?;
        table
            .iter()?
            .map(|r| {
                let (k, v) = r?;
                Ok((k.value().to_vec(), v.value().into_inner()))
            })
            .collect()
    }

    /// Modify an existing entry. Fails if absent.
    pub fn update<F, T>(&self, keytag: &Keytag, f: F) -> Result<T, Error>
    where
        F: FnOnce(Channel) -> Result<(Channel, T), channel::ChannelError>,
    {
        let tx = self.0.begin_write()?;
        let result = {
            let mut table = tx.open_table(TABLE)?;
            let current = table
                .get(keytag.as_ref())?
                .map(|v| v.value().into_inner())
                .ok_or(Error::NotFound)?;
            let (updated, result) = f(current)?;
            table.insert(keytag.as_ref(), Value(updated))?;
            result
        };
        tx.commit()?;
        Ok(result)
    }
}

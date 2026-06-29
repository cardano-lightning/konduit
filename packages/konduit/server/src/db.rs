use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};

use crate::channel::{self, Channel};

const TABLE: TableDefinition<&[u8], Entry> = TableDefinition::new("channels");

// ---------------------------------------------------------------------------
// Entry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Entry(Channel);

impl Entry {
    fn into_inner(self) -> Channel {
        self.0
    }
}

impl redb::Value for Entry {
    type SelfType<'a> = Entry;
    type AsBytes<'a> = Vec<u8>;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        Entry(minicbor::decode(data).expect("corrupt Entry bytes"))
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        minicbor::to_vec(&value.0).expect("Entry encode failed")
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
    Channel(#[from] channel::Error),

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

    /// DB keys are Keytags, and so should be between 32 and 64 bytes inclusive.
    fn validate_key(key: &[u8]) -> Result<(), Error> {
        match key.len() {
            32..=64 => Ok(()),
            n => Err(Error::InvalidKey(n)),
        }
    }

    /// Fetch a channel by key.
    pub fn get(&self, key: &[u8]) -> Result<Option<Channel>, Error> {
        Self::validate_key(key)?;
        let tx = self.0.begin_read()?;
        let table = tx.open_table(TABLE)?;
        Ok(table.get(key)?.map(|v| v.value().into_inner()))
    }

    /// Insert a new channel. Errors if the key already exists.
    pub fn insert(&self, key: &[u8], channel: Channel) -> Result<(), Error> {
        Self::validate_key(key)?;
        let tx = self.0.begin_write()?;
        {
            let mut table = tx.open_table(TABLE)?;
            if table.get(key)?.is_some() {
                return Err(Error::AlreadyExists);
            }
            table.insert(key, Entry(channel))?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Remove a channel by key. Errors if the key does not exist.
    pub fn remove(&self, key: &[u8]) -> Result<(), Error> {
        Self::validate_key(key)?;
        let tx = self.0.begin_write()?;
        {
            let mut table = tx.open_table(TABLE)?;
            if table.remove(key)?.is_none() {
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
    pub fn update<F, T>(&self, key: &[u8], f: F) -> Result<Option<T>, Error>
    where
        F: FnOnce(Channel) -> Result<(Channel, Option<T>), channel::Error>,
    {
        Self::validate_key(key)?;
        let tx = self.0.begin_write()?;
        let result = {
            let mut table = tx.open_table(TABLE)?;
            let current = table
                .get(key)?
                .map(|v| v.value().into_inner())
                .ok_or(Error::NotFound)?;
            let (updated, result) = f(current)?;
            table.insert(key, Entry(updated))?;
            result
        };
        tx.commit()?;
        Ok(result)
    }
}

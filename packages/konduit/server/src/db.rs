use konduit_tmp::{Keytag, Receipt};
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
        minicbor::to_vec(value).expect("Entry encode failed")
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("Entry")
    }
}

impl Value {
    pub fn to_channel(self, keytag: &Keytag) -> Channel {
        let Self {
            retainer,
            receipt,
            aux,
        } = self;
        Channel::new_with(keytag, retainer, receipt, aux)
    }

    pub fn from_channel(val: Channel) -> Self {
        let retainer = val.retainer().to_owned();
        let receipt = val.receipt().to_owned();
        let aux = val.aux().to_owned();
        Self {
            retainer,
            receipt,
            aux,
        }
    }
}

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("entry not found")]
    NotFound,
    #[error("entry already exists")]
    AlreadyExists,
    #[error("channel: {0}")]
    Channel(#[from] channel::Error),
    #[error("transaction conflict")]
    Contended,
    #[error("backend: {0}")]
    Backend(String),
}

impl From<redb::DatabaseError> for Error {
    fn from(e: redb::DatabaseError) -> Self {
        Error::Backend(e.to_string())
    }
}

impl From<redb::TransactionError> for Error {
    fn from(e: redb::TransactionError) -> Self {
        Error::Backend(e.to_string())
    }
}

impl From<redb::TableError> for Error {
    fn from(e: redb::TableError) -> Self {
        Error::Backend(e.to_string())
    }
}

impl From<redb::StorageError> for Error {
    fn from(e: redb::StorageError) -> Self {
        Error::Backend(e.to_string())
    }
}

impl From<redb::CommitError> for Error {
    fn from(e: redb::CommitError) -> Self {
        Error::Backend(e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Db
// ---------------------------------------------------------------------------

pub struct Db(Database);

impl Db {
    pub fn open(path: &str) -> Result<Self, Error> {
        Ok(Self(Database::create(path)?))
    }

    /// All keys
    pub fn keys(&self) -> Result<Vec<Keytag>, Error> {
        let tx = self.0.begin_read()?;
        let table = tx.open_table(TABLE)?;
        table
            .iter()?
            .map(|r| {
                let (k, _v) = r?;
                Ok(Keytag::try_from(k.value().to_vec()).expect("illegal key"))
            })
            .collect()
    }

    /// Fetch a channel by key.
    pub fn get(&self, keytag: &Keytag) -> Result<Option<Channel>, Error> {
        let tx = self.0.begin_read()?;
        let table = tx.open_table(TABLE)?;
        Ok(table
            .get(keytag.as_ref())?
            .map(|v| v.value().to_channel(keytag)))
    }

    /// Insert a new channel. Errors if the keytag. already exists.
    pub fn insert(&self, channel: Channel) -> Result<(), Error> {
        let tx = self.0.begin_write()?;
        {
            let mut table = tx.open_table(TABLE)?;
            if table.get(channel.keytag().as_ref())?.is_some() {
                return Err(Error::AlreadyExists);
            }
            table.insert(channel.keytag().as_ref(), Value::from_channel(channel))?;
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

    /// Modify an existing entry. Fails if absent.
    pub fn update<F, T>(&self, keytag: &Keytag, f: F) -> Result<T, Error>
    where
        F: FnOnce(Channel) -> Result<(Channel, T), channel::Error>,
    {
        let tx = self.0.begin_write()?;
        let result = {
            let mut table = tx.open_table(TABLE)?;
            let current = table
                .get(keytag.as_ref())?
                .map(|v| v.value().to_channel(keytag))
                .ok_or(Error::NotFound)?;
            let (updated, result) = f(current)?;
            table.insert(keytag.as_ref(), Value::from_channel(updated))?;
            result
        };
        tx.commit()?;
        Ok(result)
    }
}

/// FIXME :: this should be upstreamed
pub fn from_key(v: &[u8]) -> Keytag {
    Keytag::try_from(v.to_vec()).expect("illegal key")
}

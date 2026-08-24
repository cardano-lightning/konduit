use konduit_data::AssetDefinition;
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

#[derive(Debug, Clone, Encode, Decode, Default)]
pub struct Value {
    #[n(0)]
    retainer: Option<Retainer>,
    #[n(1)]
    receipt: Option<Receipt>,
    #[n(2)]
    aux: Aux,
    #[n(3)]
    definition: AssetDefinition,
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
            definition,
        } = self;
        Channel::new_with(keytag, definition, retainer, receipt, aux)
    }

    pub fn from_channel(val: Channel) -> Self {
        let retainer = val.retainer().to_owned();
        let receipt = val.receipt().to_owned();
        let aux = val.aux().to_owned();
        let definition = val.asset_definition().to_owned();
        Self {
            retainer,
            receipt,
            aux,
            definition,
        }
    }
}

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("transaction conflict")]
    Contended,
    #[error("backend: {0}")]
    Backend(String),
    #[error("entry not found")]
    NoChannel,
    #[error("channel: {0}")]
    Channel(#[from] channel::Error),
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
        let db = Database::create(path)?;
        let tx = db.begin_write()?;
        {
            let _ = tx.open_table(TABLE)?;
        }
        tx.commit()?;
        Ok(Self(db))
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

    /// Insert or modify a channel entry. Uses `Channel::default()` if the key does not exist.
    pub fn upsert<F>(&self, keytag: &Keytag, f: F) -> Result<(), Error>
    where
        F: FnOnce(Channel) -> Result<Channel, channel::Error>,
    {
        let tx = self.0.begin_write()?;
        {
            let mut table = tx.open_table(TABLE)?;
            let current = table
                .get(keytag.as_ref())?
                .map(|guard| guard.value())
                .unwrap_or_default()
                .to_channel(keytag);
            let updated = f(current)?;
            table.insert(keytag.as_ref(), Value::from_channel(updated))?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Modify an existing entry. Fails if absent.
    pub fn update<F>(&self, keytag: &Keytag, f: F) -> Result<(), Error>
    where
        F: FnOnce(Channel) -> Result<Channel, channel::Error>,
    {
        let tx = self.0.begin_write()?;
        {
            let mut table = tx.open_table(TABLE)?;
            let current = table
                .get(keytag.as_ref())?
                .map(|v| v.value().to_channel(keytag))
                .ok_or(Error::NoChannel)?;
            let updated = f(current)?;
            table.insert(keytag.as_ref(), Value::from_channel(updated))?;
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
                return Err(Error::NoChannel);
            }
        }
        tx.commit()?;
        Ok(())
    }
}

/// FIXME :: this should be upstreamed
pub fn from_key(v: &[u8]) -> Keytag {
    Keytag::try_from(v.to_vec()).expect("illegal key")
}

#[cfg(test)]
mod tests {
    use konduit_data::{AssetCatalog, Tag, VerifyingKey};

    use super::*;

    #[test]
    fn asset_definition_roundtrips_and_cannot_be_repriced() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = Db::open(file.path().to_str().unwrap()).unwrap();
        let catalog = AssetCatalog::builtins();
        let ada = catalog.by_alias("ada").unwrap().clone();
        let usdm = catalog.by_alias("usdm").unwrap().clone();
        let channel = Channel::new(
            VerifyingKey::from_bytes([1; 32]),
            Tag::from(b"asset-test".as_slice()),
            ada.clone(),
        );
        let keytag = channel.keytag();
        db.insert(channel).unwrap();

        let recovered = db.get(&keytag).unwrap().unwrap();
        assert_eq!(recovered.asset_definition(), &ada);
        assert!(matches!(
            db.update(&keytag, channel::update(usdm, vec![])),
            Err(Error::Channel(channel::Error::AssetDefinitionMismatch))
        ));
    }
}

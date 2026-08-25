use konduit_data::AssetDefinition;
use konduit_tmp::{Keytag, Receipt, SessionClaimRequest};
use minicbor::{Decode, Encode};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use subtle::ConstantTimeEq;

use crate::channel::{self, Aux, Channel, Retainer};

mod args;
pub use args::DbArgs as Args;

const TABLE: TableDefinition<&[u8], Value> = TableDefinition::new("channels");
const LEASES: TableDefinition<&[u8], LeaseValue> = TableDefinition::new("leases");

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

#[derive(Debug, Clone, Encode, Decode)]
struct LeaseValue {
    #[n(0)]
    generation: u64,
    #[n(1)]
    backup_hash: [u8; 32],
    #[n(2)]
    device_public_key: [u8; 32],
    #[n(3)]
    token: [u8; 32],
    #[n(4)]
    expires_at_epoch_millis: u64,
}

impl redb::Value for LeaseValue {
    type SelfType<'a> = LeaseValue;
    type AsBytes<'a> = Vec<u8>;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        minicbor::decode(data).expect("corrupt lease bytes")
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        minicbor::to_vec(value).expect("lease encode failed")
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("Lease")
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
    #[error("entry already exists")]
    AlreadyExists,
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

#[derive(Debug, thiserror::Error)]
pub enum LeaseClaimError {
    #[error("lease claim conflicts with active generation")]
    Conflict,
    #[error(transparent)]
    Database(#[from] Error),
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
            let _ = tx.open_table(LEASES)?;
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

    /// Insert a new channel. Errors if the keytag already exists.
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

    pub fn claim_lease(
        &self,
        claim: &SessionClaimRequest,
        token: [u8; 32],
        expires_at_epoch_millis: u64,
    ) -> Result<(), LeaseClaimError> {
        let tx = self.0.begin_write().map_err(Error::from)?;
        {
            let mut table = tx.open_table(LEASES).map_err(Error::from)?;
            if let Some(current) = table
                .get(claim.wallet_verification_key_hex.as_slice())
                .map_err(Error::from)?
                .map(|value| value.value())
            {
                if claim.generation < current.generation
                    || (claim.generation == current.generation
                        && (claim.backup_hash_hex != current.backup_hash
                            || claim.device_public_key_hex != current.device_public_key))
                {
                    return Err(LeaseClaimError::Conflict);
                }
            }
            table
                .insert(
                    claim.wallet_verification_key_hex.as_slice(),
                    LeaseValue {
                        generation: claim.generation,
                        backup_hash: claim.backup_hash_hex,
                        device_public_key: claim.device_public_key_hex,
                        token,
                        expires_at_epoch_millis,
                    },
                )
                .map_err(Error::from)?;
        }
        tx.commit().map_err(Error::from)?;
        Ok(())
    }

    pub fn validate_lease(
        &self,
        wallet_key: &[u8; 32],
        token: &[u8; 32],
        now_epoch_millis: u64,
    ) -> Result<bool, Error> {
        let tx = self.0.begin_read()?;
        let table = tx.open_table(LEASES)?;
        Ok(table
            .get(wallet_key.as_slice())?
            .map(|value| {
                let lease = value.value();
                lease.expires_at_epoch_millis > now_epoch_millis
                    && bool::from(lease.token.ct_eq(token))
            })
            .unwrap_or(false))
    }
}

/// FIXME :: this should be upstreamed
pub fn from_key(v: &[u8]) -> Keytag {
    Keytag::try_from(v.to_vec()).expect("illegal key")
}

#[cfg(test)]
mod tests {
    use cardano_sdk::SigningKey;
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

    fn lease_db() -> (tempfile::NamedTempFile, Db) {
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = Db::open(file.path().to_str().unwrap()).unwrap();
        (file, db)
    }

    fn claim(generation: u64) -> SessionClaimRequest {
        SessionClaimRequest::signed(&SigningKey::from([1; 32]), generation, [2; 32], [3; 32], 0)
    }

    #[test]
    fn first_lease_claim_succeeds() {
        let (_file, db) = lease_db();
        let claim = claim(1);
        db.claim_lease(&claim, [4; 32], 100).unwrap();
        assert!(
            db.validate_lease(&claim.wallet_verification_key_hex, &[4; 32], 99)
                .unwrap()
        );
    }

    #[test]
    fn same_generation_and_identity_refreshes() {
        let (_file, db) = lease_db();
        let claim = claim(1);
        db.claim_lease(&claim, [4; 32], 100).unwrap();
        db.claim_lease(&claim, [5; 32], 200).unwrap();
        assert!(
            db.validate_lease(&claim.wallet_verification_key_hex, &[5; 32], 150)
                .unwrap()
        );
    }

    #[test]
    fn lower_generation_conflicts() {
        let (_file, db) = lease_db();
        db.claim_lease(&claim(2), [4; 32], 100).unwrap();
        assert!(matches!(
            db.claim_lease(&claim(1), [5; 32], 200),
            Err(LeaseClaimError::Conflict)
        ));
    }

    #[test]
    fn equal_generation_with_different_device_conflicts() {
        let (_file, db) = lease_db();
        let claim = claim(1);
        db.claim_lease(&claim, [4; 32], 100).unwrap();
        let mut changed = claim;
        changed.device_public_key_hex[0] ^= 1;
        assert!(matches!(
            db.claim_lease(&changed, [5; 32], 200),
            Err(LeaseClaimError::Conflict)
        ));
    }

    #[test]
    fn higher_generation_replaces_lease() {
        let (_file, db) = lease_db();
        let old = claim(1);
        let new = claim(2);
        db.claim_lease(&old, [4; 32], 100).unwrap();
        db.claim_lease(&new, [5; 32], 200).unwrap();
        assert!(
            db.validate_lease(&new.wallet_verification_key_hex, &[5; 32], 150)
                .unwrap()
        );
    }

    #[test]
    fn old_token_is_rejected_after_refresh() {
        let (_file, db) = lease_db();
        let claim = claim(1);
        db.claim_lease(&claim, [4; 32], 100).unwrap();
        db.claim_lease(&claim, [5; 32], 200).unwrap();
        assert!(
            !db.validate_lease(&claim.wallet_verification_key_hex, &[4; 32], 50)
                .unwrap()
        );
    }

    #[test]
    fn expired_token_is_rejected() {
        let (_file, db) = lease_db();
        let claim = claim(1);
        db.claim_lease(&claim, [4; 32], 100).unwrap();
        assert!(
            !db.validate_lease(&claim.wallet_verification_key_hex, &[4; 32], 100)
                .unwrap()
        );
    }

    #[test]
    fn lease_validates_after_database_reopens() {
        let (file, db) = lease_db();
        let claim = claim(1);
        db.claim_lease(&claim, [4; 32], 100).unwrap();
        drop(db);
        let db = Db::open(file.path().to_str().unwrap()).unwrap();
        assert!(
            db.validate_lease(&claim.wallet_verification_key_hex, &[4; 32], 99)
                .unwrap()
        );
    }
}

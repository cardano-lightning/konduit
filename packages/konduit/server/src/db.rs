use cardano_sdk::Hash;
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
    #[n(5)]
    last_claim_timestamp: Option<u64>,
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
    #[error("channel lease is invalid")]
    LeaseInvalid,
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
    #[error("no channel for wallet")]
    UnknownWallet,
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
    ) -> Result<([u8; 32], u64), LeaseClaimError> {
        if claim.generation == 0 {
            return Err(LeaseClaimError::Conflict);
        }

        let tx = self.0.begin_write().map_err(Error::from)?;
        {
            let channels = tx.open_table(TABLE).map_err(Error::from)?;
            let wallet = claim.wallet_verification_key_hex;
            let has_channel = match next_wallet_prefix(&wallet) {
                Some(end) => channels
                    .range(wallet.as_slice()..end.as_slice())
                    .map_err(Error::from)?
                    .next()
                    .is_some(),
                None => channels
                    .range(wallet.as_slice()..)
                    .map_err(Error::from)?
                    .next()
                    .is_some(),
            };
            if !has_channel {
                return Err(LeaseClaimError::UnknownWallet);
            }
        }

        {
            let mut leases = tx.open_table(LEASES).map_err(Error::from)?;
            if let Some(current) = leases
                .get(claim.wallet_verification_key_hex.as_slice())
                .map_err(Error::from)?
                .map(|value| value.value())
            {
                let same_identity = claim.backup_hash_hex == current.backup_hash
                    && claim.device_public_key_hex == current.device_public_key;
                if claim.generation < current.generation
                    || (claim.generation == current.generation && !same_identity)
                    || (claim.generation == current.generation
                        && claim.timestamp <= current.last_claim_timestamp.unwrap_or(0))
                {
                    return Err(LeaseClaimError::Conflict);
                }
            }
            leases
                .insert(
                    claim.wallet_verification_key_hex.as_slice(),
                    LeaseValue {
                        generation: claim.generation,
                        backup_hash: claim.backup_hash_hex,
                        device_public_key: claim.device_public_key_hex,
                        token: hash_token(&token),
                        expires_at_epoch_millis,
                        last_claim_timestamp: Some(claim.timestamp),
                    },
                )
                .map_err(Error::from)?;
        }
        tx.commit().map_err(Error::from)?;
        Ok((token, expires_at_epoch_millis))
    }

    pub fn update_with_lease<F>(
        &self,
        keytag: &Keytag,
        token: &[u8; 32],
        now_epoch_millis: u64,
        f: F,
    ) -> Result<(), Error>
    where
        F: FnOnce(Channel) -> Result<Channel, channel::Error>,
    {
        let wallet_key: [u8; 32] = keytag
            .as_ref()
            .get(..32)
            .and_then(|key| key.try_into().ok())
            .ok_or(Error::LeaseInvalid)?;
        let tx = self.0.begin_write()?;
        let current = {
            let table = tx.open_table(TABLE)?;
            table
                .get(keytag.as_ref())?
                .map(|value| value.value().to_channel(keytag))
                .ok_or(Error::NoChannel)?
        };
        let lease_valid = {
            let leases = tx.open_table(LEASES)?;
            leases
                .get(wallet_key.as_slice())?
                .map(|value| {
                    let lease = value.value();
                    lease.expires_at_epoch_millis > now_epoch_millis
                        && bool::from(lease.token.ct_eq(&hash_token(token)))
                })
                .unwrap_or(false)
        };
        if !lease_valid {
            return Err(Error::LeaseInvalid);
        }
        {
            let mut table = tx.open_table(TABLE)?;
            table.insert(keytag.as_ref(), Value::from_channel(f(current)?))?;
        }
        tx.commit()?;
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
                    && bool::from(lease.token.ct_eq(&hash_token(token)))
            })
            .unwrap_or(false))
    }
}

fn hash_token(token: &[u8; 32]) -> [u8; 32] {
    Hash::<32>::new(token).into()
}

fn next_wallet_prefix(prefix: &[u8; 32]) -> Option<[u8; 32]> {
    let mut end = *prefix;
    for i in (0..32).rev() {
        if end[i] != 0xff {
            end[i] += 1;
            for byte in end.iter_mut().skip(i + 1) {
                *byte = 0;
            }
            return Some(end);
        }
    }
    None
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
        let wallet = SigningKey::from([1; 32]);
        let keytag = Keytag::new(
            &wallet.to_verification_key(),
            &Tag::from(b"lease-test".as_slice()),
        );
        db.insert(
            channel::open(
                keytag,
                AssetCatalog::builtins().by_alias("ada").unwrap().clone(),
                vec![],
            )
            .unwrap(),
        )
        .unwrap();
        (file, db)
    }

    fn claim(generation: u64, timestamp: u64) -> SessionClaimRequest {
        SessionClaimRequest::signed(
            &SigningKey::from([1; 32]),
            generation,
            [2; 32],
            [3; 32],
            timestamp,
        )
    }

    #[test]
    fn first_lease_claim_succeeds() {
        let (_file, db) = lease_db();
        let claim = claim(1, 1);
        db.claim_lease(&claim, [4; 32], 100).unwrap();
        assert!(
            db.validate_lease(&claim.wallet_verification_key_hex, &[4; 32], 99)
                .unwrap()
        );
    }

    #[test]
    fn same_generation_requires_newer_timestamp() {
        let (_file, db) = lease_db();
        let newer = claim(1, 2);
        db.claim_lease(&newer, [4; 32], 100).unwrap();
        assert!(matches!(
            db.claim_lease(&claim(1, 1), [5; 32], 200),
            Err(LeaseClaimError::Conflict)
        ));
    }

    #[test]
    fn exact_claim_retry_conflicts() {
        let (_file, db) = lease_db();
        let claim = claim(1, 1);
        db.claim_lease(&claim, [4; 32], 100).unwrap();
        assert!(matches!(
            db.claim_lease(&claim, [5; 32], 200),
            Err(LeaseClaimError::Conflict)
        ));
    }

    #[test]
    fn zero_generation_conflicts() {
        let (_file, db) = lease_db();
        let mut claim = claim(1, 1);
        claim.generation = 0;
        assert!(matches!(
            db.claim_lease(&claim, [4; 32], 100),
            Err(LeaseClaimError::Conflict)
        ));
    }

    #[test]
    fn unknown_wallet_does_not_create_lease() {
        let (_file, db) = lease_db();
        let claim = SessionClaimRequest::signed(&SigningKey::from([9; 32]), 1, [2; 32], [3; 32], 1);
        assert!(matches!(
            db.claim_lease(&claim, [4; 32], 100),
            Err(LeaseClaimError::UnknownWallet)
        ));
        assert!(
            !db.validate_lease(&claim.wallet_verification_key_hex, &[4; 32], 99)
                .unwrap()
        );
    }

    #[test]
    fn fenced_update_rejects_stale_lease() {
        let (_file, db) = lease_db();
        let claim = claim(1, 1);
        db.claim_lease(&claim, [4; 32], 100).unwrap();
        let keytag = db.keys().unwrap().pop().unwrap();
        assert!(matches!(
            db.update_with_lease(&keytag, &[5; 32], 99, |channel| Ok(channel)),
            Err(Error::LeaseInvalid)
        ));
        db.update_with_lease(&keytag, &[4; 32], 99, |channel| Ok(channel))
            .unwrap();
    }

    #[test]
    fn equal_generation_with_different_device_conflicts() {
        let (_file, db) = lease_db();
        let claim = claim(1, 1);
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
        let old = claim(1, 1);
        let new = claim(2, 1);
        db.claim_lease(&old, [4; 32], 100).unwrap();
        db.claim_lease(&new, [5; 32], 200).unwrap();
        assert!(
            db.validate_lease(&new.wallet_verification_key_hex, &[5; 32], 150)
                .unwrap()
        );
    }

    #[test]
    fn expired_token_is_rejected() {
        let (_file, db) = lease_db();
        let claim = claim(1, 1);
        db.claim_lease(&claim, [4; 32], 100).unwrap();
        assert!(
            !db.validate_lease(&claim.wallet_verification_key_hex, &[4; 32], 100)
                .unwrap()
        );
    }

    #[test]
    fn lease_validates_after_database_reopens() {
        let (file, db) = lease_db();
        let claim = claim(1, 1);
        db.claim_lease(&claim, [4; 32], 100).unwrap();
        drop(db);
        let db = Db::open(file.path().to_str().unwrap()).unwrap();
        assert!(
            db.validate_lease(&claim.wallet_verification_key_hex, &[4; 32], 99)
                .unwrap()
        );
    }
}

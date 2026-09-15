use cardano_sdk::Hash;
use konduit_data::{AssetDefinition, Lock, Locked, Secret};
use konduit_tmp::{Keytag, Receipt, SessionClaimRequest};
use minicbor::{Decode, Encode};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::channel::{self, Aux, Channel, Retainer};

mod args;
pub use args::DbArgs as Args;

const TABLE: TableDefinition<&[u8], Value> = TableDefinition::new("channels");
const LEASES: TableDefinition<&[u8], LeaseValue> = TableDefinition::new("leases");
const CHANNEL_OPERATIONS: TableDefinition<&[u8], ChannelOperationValue> =
    TableDefinition::new("channel_operations");
const PAYMENT_AUTHORIZATIONS: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("payment_authorizations");

pub(crate) fn payment_identity(keytag: &Keytag, index: u64) -> [u8; 32] {
    let mut identity = Sha256::new();
    identity.update(keytag.as_ref());
    identity.update(index.to_be_bytes());
    identity.finalize().into()
}

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
        redb::TypeName::new("EntryV2")
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

#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
pub struct ChannelOperationValue {
    #[n(0)]
    pub keytag: Vec<u8>,
    #[n(1)]
    pub expected_transaction_id: [u8; 32],
    #[n(2)]
    pub transaction_digest: [u8; 32],
    #[n(3)]
    pub transaction: Vec<u8>,
    #[n(4)]
    pub status: String,
    #[n(5)]
    pub transaction_id: Option<[u8; 32]>,
    #[n(6)]
    pub depth: Option<u64>,
}

impl redb::Value for ChannelOperationValue {
    type SelfType<'a> = ChannelOperationValue;
    type AsBytes<'a> = Vec<u8>;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        minicbor::decode(data).expect("corrupt channel operation bytes")
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        minicbor::to_vec(value).expect("channel operation encode failed")
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("ChannelOperationV1")
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
    #[error("operation id conflicts with an existing request")]
    OperationConflict,
    #[error("failed payment not found")]
    PaymentNotFound,
    #[error("multiple records match the failed payment")]
    PaymentAmbiguous,
    #[error("invalid persisted payment authorization")]
    PaymentAuthorizationInvalid,
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
            let _ = tx.open_table(CHANNEL_OPERATIONS)?;
            let _ = tx.open_table(PAYMENT_AUTHORIZATIONS)?;
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
            let mut leases = tx.open_table(LEASES).map_err(Error::from)?;
            if let Some(current) = leases
                .get(claim.wallet_verification_key_hex.as_slice())
                .map_err(Error::from)?
                .map(|value| value.value())
            {
                let same_device = claim.device_public_key_hex == current.device_public_key;
                if claim.generation < current.generation
                    || (claim.generation == current.generation && !same_device)
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

    pub fn reserve_channel_operation(
        &self,
        operation_id: &[u8; 16],
        keytag: &Keytag,
        token: &[u8; 32],
        now_epoch_millis: u64,
        expected_transaction_id: [u8; 32],
        transaction_digest: [u8; 32],
        transaction: Vec<u8>,
    ) -> Result<ChannelOperationValue, Error> {
        let wallet_key: [u8; 32] = keytag
            .as_ref()
            .get(..32)
            .and_then(|key| key.try_into().ok())
            .ok_or(Error::LeaseInvalid)?;
        let tx = self.0.begin_write()?;
        {
            let leases = tx.open_table(LEASES)?;
            let valid = leases
                .get(wallet_key.as_slice())?
                .map(|value| {
                    let lease = value.value();
                    lease.expires_at_epoch_millis > now_epoch_millis
                        && bool::from(lease.token.ct_eq(&hash_token(token)))
                })
                .unwrap_or(false);
            if !valid {
                return Err(Error::LeaseInvalid);
            }
        }
        let operation = {
            let mut operations = tx.open_table(CHANNEL_OPERATIONS)?;
            if let Some(existing) = operations.get(operation_id.as_slice())? {
                let existing = existing.value();
                if existing.keytag != keytag.as_ref()
                    || existing.expected_transaction_id != expected_transaction_id
                    || existing.transaction_digest != transaction_digest
                    || existing.transaction != transaction
                {
                    return Err(Error::OperationConflict);
                }
                existing
            } else {
                let operation = ChannelOperationValue {
                    keytag: keytag.as_ref().to_vec(),
                    expected_transaction_id,
                    transaction_digest,
                    transaction,
                    status: "reserved".into(),
                    transaction_id: None,
                    depth: None,
                };
                operations.insert(operation_id.as_slice(), operation.clone())?;
                operation
            }
        };
        tx.commit()?;
        Ok(operation)
    }

    pub fn channel_operation(
        &self,
        operation_id: &[u8; 16],
        keytag: &Keytag,
    ) -> Result<Option<ChannelOperationValue>, Error> {
        let tx = self.0.begin_read()?;
        let operations = tx.open_table(CHANNEL_OPERATIONS)?;
        Ok(operations
            .get(operation_id.as_slice())?
            .map(|value| value.value())
            .filter(|operation| operation.keytag == keytag.as_ref()))
    }

    pub fn update_channel_operation(
        &self,
        operation_id: &[u8; 16],
        keytag: &Keytag,
        status: String,
        transaction_id: Option<[u8; 32]>,
        depth: Option<u64>,
    ) -> Result<ChannelOperationValue, Error> {
        let tx = self.0.begin_write()?;
        let updated = {
            let mut operations = tx.open_table(CHANNEL_OPERATIONS)?;
            let mut operation = operations
                .get(operation_id.as_slice())?
                .map(|value| value.value())
                .filter(|operation| operation.keytag == keytag.as_ref())
                .ok_or(Error::NoChannel)?;
            operation.status = status;
            operation.transaction_id = transaction_id;
            operation.depth = depth;
            operations.insert(operation_id.as_slice(), operation.clone())?;
            operation
        };
        tx.commit()?;
        Ok(updated)
    }

    pub fn reserve_payment(
        &self,
        identity: &[u8; 32],
        request_digest: &[u8; 32],
        payment_hash: &[u8; 32],
        keytag: &Keytag,
        token: &[u8; 32],
        now_epoch_millis: u64,
        locked: Locked,
    ) -> Result<bool, Error> {
        let wallet_key: [u8; 32] = keytag
            .as_ref()
            .get(..32)
            .and_then(|key| key.try_into().ok())
            .ok_or(Error::LeaseInvalid)?;
        let tx = self.0.begin_write()?;
        {
            let leases = tx.open_table(LEASES)?;
            let valid = leases
                .get(wallet_key.as_slice())?
                .map(|value| {
                    let lease = value.value();
                    lease.expires_at_epoch_millis > now_epoch_millis
                        && bool::from(lease.token.ct_eq(&hash_token(token)))
                })
                .unwrap_or(false);
            if !valid {
                return Err(Error::LeaseInvalid);
            }
        }
        let mut persisted = Vec::with_capacity(64);
        persisted.extend_from_slice(request_digest);
        persisted.extend_from_slice(payment_hash);
        let inserted = {
            let mut authorizations = tx.open_table(PAYMENT_AUTHORIZATIONS)?;
            let existing = authorizations
                .get(identity.as_slice())?
                .map(|value| value.value().to_vec());
            match existing.as_deref() {
                Some(existing) if existing == persisted => false,
                Some(_) => return Err(Error::OperationConflict),
                None => {
                    authorizations.insert(identity.as_slice(), persisted.as_slice())?;
                    true
                }
            }
        };
        if inserted {
            let mut channels = tx.open_table(TABLE)?;
            let mut channel = channels
                .get(keytag.as_ref())?
                .map(|value| value.value().to_channel(keytag))
                .ok_or(Error::NoChannel)?;
            channel.apply_locked(locked)?;
            channels.insert(keytag.as_ref(), Value::from_channel(channel))?;
        }
        tx.commit()?;
        Ok(inserted)
    }

    pub fn complete_payment(
        &self,
        identity: &[u8; 32],
        request_digest: &[u8; 32],
        payment_hash: &[u8; 32],
        keytag: &Keytag,
        secret: Secret,
    ) -> Result<(), Error> {
        let mut expected = Vec::with_capacity(64);
        expected.extend_from_slice(request_digest);
        expected.extend_from_slice(payment_hash);
        let tx = self.0.begin_write()?;
        {
            let authorizations = tx.open_table(PAYMENT_AUTHORIZATIONS)?;
            let existing = authorizations
                .get(identity.as_slice())?
                .map(|value| value.value().to_vec())
                .ok_or(Error::OperationConflict)?;
            if existing != expected {
                return Err(Error::OperationConflict);
            }
            let mut channels = tx.open_table(TABLE)?;
            let mut channel = channels
                .get(keytag.as_ref())?
                .map(|value| value.value().to_channel(keytag))
                .ok_or(Error::NoChannel)?;
            let already_complete = channel.receipt().as_ref().is_some_and(|receipt| {
                receipt
                    .unlockeds()
                    .any(|unlocked| unlocked.secret() == &secret)
            });
            if !already_complete {
                channel.apply_secret(secret)?;
                channels.insert(keytag.as_ref(), Value::from_channel(channel))?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn cancel_payment(
        &self,
        identity: &[u8; 32],
        request_digest: &[u8; 32],
        payment_hash: &[u8; 32],
        keytag: &Keytag,
        index: u64,
        lock: &Lock,
    ) -> Result<(), Error> {
        let mut expected = Vec::with_capacity(64);
        expected.extend_from_slice(request_digest);
        expected.extend_from_slice(payment_hash);
        let tx = self.0.begin_write()?;
        {
            let mut authorizations = tx.open_table(PAYMENT_AUTHORIZATIONS)?;
            let existing = authorizations
                .get(identity.as_slice())?
                .map(|value| value.value().to_vec())
                .ok_or(Error::OperationConflict)?;
            if existing != expected {
                return Err(Error::OperationConflict);
            }
            let mut channels = tx.open_table(TABLE)?;
            let mut channel = channels
                .get(keytag.as_ref())?
                .map(|value| value.value().to_channel(keytag))
                .ok_or(Error::NoChannel)?;
            channel.cancel_locked(index, lock)?;
            channels.insert(keytag.as_ref(), Value::from_channel(channel))?;
            authorizations.remove(identity.as_slice())?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn cancel_failed_payment(
        &self,
        payment_hash: &[u8; 32],
    ) -> Result<(Keytag, u64, u64), Error> {
        let authorizations = {
            let tx = self.0.begin_read()?;
            let table = tx.open_table(PAYMENT_AUTHORIZATIONS)?;
            table
                .iter()?
                .filter_map(|entry| match entry {
                    Ok((identity, value)) if value.value().get(32..) == Some(payment_hash) => Some(
                        Ok((identity.value().to_vec(), value.value()[..32].to_vec())),
                    ),
                    Ok(_) => None,
                    Err(error) => Some(Err(Error::from(error))),
                })
                .collect::<Result<Vec<_>, _>>()?
        };
        let [(identity, request_digest)] = authorizations.as_slice() else {
            return Err(if authorizations.is_empty() {
                Error::PaymentNotFound
            } else {
                Error::PaymentAmbiguous
            });
        };
        let identity: [u8; 32] = identity
            .as_slice()
            .try_into()
            .map_err(|_| Error::PaymentAuthorizationInvalid)?;
        let request_digest: [u8; 32] = request_digest
            .as_slice()
            .try_into()
            .map_err(|_| Error::PaymentAuthorizationInvalid)?;
        let mut matches = Vec::new();
        for keytag in self.keys()? {
            let channel = self.get(&keytag)?.ok_or(Error::NoChannel)?;
            if let Some(receipt) = channel.receipt() {
                matches.extend(
                    receipt
                        .lockeds()
                        .filter(|locked| {
                            locked.lock().as_ref() == payment_hash
                                && payment_identity(&keytag, locked.index()) == identity
                        })
                        .map(|locked| {
                            (
                                keytag.clone(),
                                locked.index(),
                                locked.amount(),
                                *locked.lock(),
                            )
                        }),
                );
            }
        }
        let [(keytag, index, amount, lock)] = matches.as_slice() else {
            return Err(if matches.is_empty() {
                Error::PaymentNotFound
            } else {
                Error::PaymentAmbiguous
            });
        };
        self.cancel_payment(
            &identity,
            &request_digest,
            payment_hash,
            keytag,
            *index,
            lock,
        )?;
        Ok((keytag.clone(), *index, *amount))
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

#[cfg(test)]
mod tests {
    use cardano_sdk::SigningKey;
    use konduit_data::{
        AssetCatalog, ChequeBody, Duration, Lock, Locked, Secret, SigningKey as ProtocolSigningKey,
        Squash, SquashBody, Tag, VerifyingKey,
    };

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
            [9; 32],
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
    fn first_claim_does_not_require_an_open_channel() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = Db::open(file.path().to_str().unwrap()).unwrap();
        let claim = SessionClaimRequest::signed(
            &SigningKey::from([9; 32]),
            [8; 32],
            1,
            [2; 32],
            [3; 32],
            1,
        );
        db.claim_lease(&claim, [4; 32], 100).unwrap();
        assert!(
            db.validate_lease(&claim.wallet_verification_key_hex, &[4; 32], 99)
                .unwrap()
        );
        assert!(db.keys().unwrap().is_empty());
    }

    #[test]
    fn fenced_update_rejects_stale_lease() {
        let (_file, db) = lease_db();
        let claim = claim(1, 1);
        db.claim_lease(&claim, [4; 32], 100).unwrap();
        let keytag = db.keys().unwrap().pop().unwrap();
        assert!(matches!(
            db.update_with_lease(&keytag, &[5; 32], 99, Ok),
            Err(Error::LeaseInvalid)
        ));
        db.update_with_lease(&keytag, &[4; 32], 99, Ok).unwrap();
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
    fn equal_generation_same_device_may_advance_backup() {
        let (_file, db) = lease_db();
        let original = claim(1, 1);
        db.claim_lease(&original, [4; 32], 100).unwrap();
        let mut advanced = claim(1, 2);
        advanced.backup_hash_hex[0] ^= 1;
        db.claim_lease(&advanced, [5; 32], 200).unwrap();
        assert!(
            db.validate_lease(&advanced.wallet_verification_key_hex, &[5; 32], 199)
                .unwrap()
        );
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
    fn channel_operation_reservation_is_idempotent_and_fenced() {
        let (_file, db) = lease_db();
        let original = claim(1, 1);
        db.claim_lease(&original, [4; 32], 100).unwrap();
        let keytag = db.keys().unwrap().pop().unwrap();
        let first = db
            .reserve_channel_operation(&[1; 16], &keytag, &[4; 32], 99, [2; 32], [3; 32], vec![4])
            .unwrap();
        assert_eq!(
            first,
            db.reserve_channel_operation(
                &[1; 16],
                &keytag,
                &[4; 32],
                99,
                [2; 32],
                [3; 32],
                vec![4]
            )
            .unwrap()
        );
        assert!(matches!(
            db.reserve_channel_operation(
                &[1; 16],
                &keytag,
                &[4; 32],
                99,
                [2; 32],
                [3; 32],
                vec![5]
            ),
            Err(Error::OperationConflict)
        ));

        db.claim_lease(&claim(2, 2), [5; 32], 200).unwrap();
        assert!(matches!(
            db.reserve_channel_operation(
                &[6; 16],
                &keytag,
                &[4; 32],
                99,
                [2; 32],
                [3; 32],
                vec![4]
            ),
            Err(Error::LeaseInvalid)
        ));
        assert!(db.channel_operation(&[1; 16], &keytag).unwrap().is_some());
    }

    #[test]
    fn maintenance_cleanup_removes_matching_lock_and_authorization() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = Db::open(file.path().to_str().unwrap()).unwrap();
        let signing = ProtocolSigningKey::from_bytes([6; 32]);
        let tag = Tag::from(b"cleanup-test".as_slice());
        let mut channel = Channel::new(
            signing.verifying_key(),
            tag.clone(),
            AssetCatalog::builtins().by_alias("ada").unwrap().clone(),
        );
        channel
            .apply_retainer(vec![channel::Retainer {
                amount: 1_000,
                subbed: 0,
                useds: vec![],
            }])
            .unwrap();
        channel
            .apply_squash(Squash::make(&signing, &tag, SquashBody::zero()).into_unverified())
            .unwrap();
        let payment_hash = [7; 32];
        channel
            .apply_locked(
                Locked::make(
                    &signing,
                    &tag,
                    ChequeBody::new(1, 10, Duration::from_secs(60), Lock(payment_hash)),
                )
                .into_unverified(),
            )
            .unwrap();
        let keytag = channel.keytag();
        let identity = payment_identity(&keytag, 1);
        let request_digest = [8; 32];
        db.insert(channel).unwrap();
        let tx = db.0.begin_write().unwrap();
        {
            let mut table = tx.open_table(PAYMENT_AUTHORIZATIONS).unwrap();
            let mut persisted = Vec::from(request_digest);
            persisted.extend_from_slice(&payment_hash);
            table
                .insert(identity.as_slice(), persisted.as_slice())
                .unwrap();
        }
        tx.commit().unwrap();

        assert_eq!(
            db.cancel_failed_payment(&payment_hash).unwrap(),
            (keytag.clone(), 1, 10)
        );
        assert_eq!(
            db.get(&keytag)
                .unwrap()
                .unwrap()
                .receipt()
                .as_ref()
                .unwrap()
                .lockeds()
                .count(),
            0
        );
        assert!(matches!(
            db.cancel_failed_payment(&payment_hash),
            Err(Error::PaymentNotFound)
        ));
    }

    #[test]
    fn successful_payment_persists_unlocked_cheque_idempotently() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = Db::open(file.path().to_str().unwrap()).unwrap();
        let signing = ProtocolSigningKey::from_bytes([6; 32]);
        let tag = Tag::from(b"payment-test".as_slice());
        let mut channel = Channel::new(
            signing.verifying_key(),
            tag.clone(),
            AssetCatalog::builtins().by_alias("ada").unwrap().clone(),
        );
        channel
            .apply_retainer(vec![channel::Retainer {
                amount: 1_000,
                subbed: 0,
                useds: vec![],
            }])
            .unwrap();
        channel
            .apply_squash(Squash::make(&signing, &tag, SquashBody::zero()).into_unverified())
            .unwrap();
        let secret = Secret([7; 32]);
        let payment_hash = Lock::from(&secret).0;
        channel
            .apply_locked(
                Locked::make(
                    &signing,
                    &tag,
                    ChequeBody::new(1, 10, Duration::from_secs(60), Lock(payment_hash)),
                )
                .into_unverified(),
            )
            .unwrap();
        let keytag = channel.keytag();
        let identity = payment_identity(&keytag, 1);
        let request_digest = [8; 32];
        db.insert(channel).unwrap();
        let tx = db.0.begin_write().unwrap();
        {
            let mut table = tx.open_table(PAYMENT_AUTHORIZATIONS).unwrap();
            let mut persisted = Vec::from(request_digest);
            persisted.extend_from_slice(&payment_hash);
            table
                .insert(identity.as_slice(), persisted.as_slice())
                .unwrap();
        }
        tx.commit().unwrap();

        db.complete_payment(&identity, &request_digest, &payment_hash, &keytag, secret)
            .unwrap();
        db.complete_payment(&identity, &request_digest, &payment_hash, &keytag, secret)
            .unwrap();

        let receipt = db.get(&keytag).unwrap().unwrap().receipt().clone().unwrap();
        assert_eq!(receipt.lockeds().count(), 0);
        assert_eq!(receipt.unlockeds().count(), 1);
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

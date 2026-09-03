use crate::error::ApiError;
use crate::providers::{Ledger, SubmitResult, TxPresence};
use crate::tx::SignedTx;
use crate::wire::OperationResponse;
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const OPERATIONS: TableDefinition<&[u8], &[u8]> = TableDefinition::new("operations");
const TX_IDS: TableDefinition<&[u8], &[u8]> = TableDefinition::new("transaction_ids");
const CONFIRMED_DEPTH: u64 = 5;
const SETTLED_DEPTH: u64 = 2160;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum InternalState {
    Prepared,
    Submitting,
    Accepted,
    Confirmed,
    Settled,
    Rejected,
}

impl InternalState {
    pub fn public(self) -> &'static str {
        match self {
            Self::Prepared | Self::Submitting => "pending",
            Self::Accepted => "accepted",
            Self::Confirmed => "confirmed",
            Self::Settled => "settled",
            Self::Rejected => "rejected",
        }
    }

    fn is_pending(self) -> bool {
        matches!(self, Self::Prepared | Self::Submitting | Self::Accepted)
    }

    fn rank(self) -> u8 {
        match self {
            Self::Prepared => 0,
            Self::Submitting => 1,
            Self::Accepted => 2,
            Self::Confirmed => 3,
            Self::Settled | Self::Rejected => 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub operation_id: String,
    pub expected_transaction_id: String,
    pub digest: String,
    pub cbor: Option<String>,
    pub ttl: Option<u64>,
    pub state: InternalState,
    pub inclusion_height: Option<u64>,
    pub attempts: u32,
    #[serde(default)]
    pub revision: u64,
    #[serde(default)]
    pub submit_started_at: Option<u64>,
    #[serde(default)]
    pub created_at_epoch_secs: u64,
}

#[derive(Clone, Copy)]
pub struct OperationKey([u8; 17]);

impl OperationKey {
    fn new(namespace: u8, operation_id: &[u8; 16]) -> Self {
        let mut key = [0; 17];
        key[0] = namespace;
        key[1..].copy_from_slice(operation_id);
        Self(key)
    }

    fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

struct StoreLimits {
    path: PathBuf,
    max_pending: usize,
    max_bytes: u64,
}

pub struct OpsStore {
    db: Arc<Database>,
    path: PathBuf,
    max_pending: usize,
    max_bytes: u64,
    inflight: AtomicUsize,
    max_inflight: usize,
}

impl OpsStore {
    pub fn open(
        path: &Path,
        max_pending: usize,
        max_bytes: u64,
        max_inflight: usize,
    ) -> anyhow::Result<Self> {
        let db = Arc::new(Database::create(path)?);
        {
            let tx = db.begin_write()?;
            let _ = tx.open_table(OPERATIONS)?;
            let _ = tx.open_table(TX_IDS)?;
            tx.commit()?;
        }
        Ok(Self {
            db,
            path: path.to_path_buf(),
            max_pending,
            max_bytes,
            inflight: AtomicUsize::new(0),
            max_inflight,
        })
    }

    pub fn client_key(operation_id: &[u8; 16]) -> OperationKey {
        OperationKey::new(b'C', operation_id)
    }

    pub fn legacy_key(operation_id: &[u8; 16]) -> OperationKey {
        OperationKey::new(b'L', operation_id)
    }

    pub fn admit_write(&self) -> Result<InflightGuard<'_>, ApiError> {
        let mut current = self.inflight.load(Ordering::Acquire);
        loop {
            if current >= self.max_inflight {
                return Err(ApiError::too_many());
            }
            match self.inflight.compare_exchange_weak(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(InflightGuard { store: self }),
                Err(value) => current = value,
            }
        }
    }

    pub async fn get(&self, key: OperationKey) -> Result<Option<Record>, ApiError> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || Self::get_blocking(&db, key))
            .await
            .map_err(|_| ApiError::unavailable())?
    }

    fn get_blocking(db: &Database, key: OperationKey) -> Result<Option<Record>, ApiError> {
        let tx = db.begin_read().map_err(|_| ApiError::unavailable())?;
        let table = tx
            .open_table(OPERATIONS)
            .map_err(|_| ApiError::unavailable())?;
        match table.get(key.as_slice()) {
            Ok(Some(value)) => serde_json::from_slice(value.value())
                .map(Some)
                .map_err(|_| ApiError::unexpected()),
            Ok(None) => Ok(None),
            Err(_) => Err(ApiError::unavailable()),
        }
    }

    pub async fn persist_new(
        &self,
        key: OperationKey,
        txid: [u8; 32],
        signed: SignedTx,
        uuid: String,
    ) -> Result<Record, ApiError> {
        let db = self.db.clone();
        let limits = StoreLimits {
            path: self.path.clone(),
            max_pending: self.max_pending,
            max_bytes: self.max_bytes,
        };
        tokio::task::spawn_blocking(move || {
            Self::persist_new_blocking(&db, &limits, key, txid, &signed, &uuid)
        })
        .await
        .map_err(|_| ApiError::unavailable())?
    }

    fn persist_new_blocking(
        db: &Database,
        limits: &StoreLimits,
        key: OperationKey,
        txid: [u8; 32],
        signed: &SignedTx,
        uuid: &str,
    ) -> Result<Record, ApiError> {
        let digest = hex::encode(signed.digest);
        let expected = hex::encode(txid);
        let tx = db.begin_write().map_err(|_| ApiError::unavailable())?;
        let result = {
            let mut operations = tx
                .open_table(OPERATIONS)
                .map_err(|_| ApiError::unavailable())?;
            let mut tx_ids = tx.open_table(TX_IDS).map_err(|_| ApiError::unavailable())?;
            if let Some(existing) = operations
                .get(key.as_slice())
                .map_err(|_| ApiError::unavailable())?
            {
                let record: Record =
                    serde_json::from_slice(existing.value()).map_err(|_| ApiError::unexpected())?;
                if record.digest == digest && record.expected_transaction_id == expected {
                    Ok(record)
                } else {
                    Err(ApiError::conflict())
                }
            } else if let Some(owner) = tx_ids
                .get(txid.as_slice())
                .map_err(|_| ApiError::unavailable())?
            {
                if owner.value() == key.as_slice() {
                    Err(ApiError::unexpected())
                } else {
                    Err(ApiError::conflict())
                }
            } else if Self::count_pending(&operations)? >= limits.max_pending
                || Self::file_len(&limits.path)? > limits.max_bytes
            {
                Err(ApiError::too_many())
            } else {
                let record = Record {
                    operation_id: uuid.to_string(),
                    expected_transaction_id: expected,
                    digest,
                    cbor: Some(hex::encode(&signed.bytes)),
                    ttl: signed.ttl,
                    state: InternalState::Prepared,
                    inclusion_height: None,
                    attempts: 0,
                    revision: 0,
                    submit_started_at: None,
                    created_at_epoch_secs: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map_err(|_| ApiError::unexpected())?
                        .as_secs(),
                };
                let bytes = serde_json::to_vec(&record).map_err(|_| ApiError::unexpected())?;
                operations
                    .insert(key.as_slice(), bytes.as_slice())
                    .map_err(|_| ApiError::unavailable())?;
                tx_ids
                    .insert(txid.as_slice(), key.as_slice())
                    .map_err(|_| ApiError::unavailable())?;
                Ok(record)
            }
        };
        tx.commit().map_err(|_| ApiError::unavailable())?;
        result
    }

    pub async fn claim_submit(&self, key: OperationKey) -> Result<Option<Record>, ApiError> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || Self::claim_submit_blocking(&db, key))
            .await
            .map_err(|_| ApiError::unavailable())?
    }

    fn claim_submit_blocking(db: &Database, key: OperationKey) -> Result<Option<Record>, ApiError> {
        let tx = db.begin_write().map_err(|_| ApiError::unavailable())?;
        let result = {
            let mut operations = tx
                .open_table(OPERATIONS)
                .map_err(|_| ApiError::unavailable())?;
            let Some(existing) = operations
                .get(key.as_slice())
                .map_err(|_| ApiError::unavailable())?
            else {
                return Ok(None);
            };
            let bytes = existing.value().to_vec();
            drop(existing);
            let mut record: Record =
                serde_json::from_slice(&bytes).map_err(|_| ApiError::unexpected())?;
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| ApiError::unexpected())?
                .as_secs();
            let lease_expired = record
                .submit_started_at
                .is_none_or(|started| now.saturating_sub(started) >= 30);
            if record.state != InternalState::Prepared
                && !(record.state == InternalState::Submitting && lease_expired)
            {
                Ok(None)
            } else {
                record.state = InternalState::Submitting;
                record.submit_started_at = Some(now);
                record.attempts = record.attempts.saturating_add(1);
                record.revision = record.revision.saturating_add(1);
                let stored = serde_json::to_vec(&record).map_err(|_| ApiError::unexpected())?;
                operations
                    .insert(key.as_slice(), stored.as_slice())
                    .map_err(|_| ApiError::unavailable())?;
                Ok(Some(record))
            }
        };
        tx.commit().map_err(|_| ApiError::unavailable())?;
        result
    }

    pub async fn put(&self, key: OperationKey, record: &mut Record) -> Result<(), ApiError> {
        let db = self.db.clone();
        let proposed = record.clone();
        *record = tokio::task::spawn_blocking(move || Self::put_blocking(&db, key, proposed))
            .await
            .map_err(|_| ApiError::unavailable())??;
        Ok(())
    }

    fn put_blocking(
        db: &Database,
        key: OperationKey,
        mut proposed: Record,
    ) -> Result<Record, ApiError> {
        let tx = db.begin_write().map_err(|_| ApiError::unavailable())?;
        let result = {
            let mut operations = tx
                .open_table(OPERATIONS)
                .map_err(|_| ApiError::unavailable())?;
            if let Some(existing) = operations
                .get(key.as_slice())
                .map_err(|_| ApiError::unavailable())?
            {
                let current: Record =
                    serde_json::from_slice(existing.value()).map_err(|_| ApiError::unexpected())?;
                if current.revision != proposed.revision
                    || current.state == InternalState::Settled
                    || current.state == InternalState::Rejected
                    || (current.state.rank() > proposed.state.rank()
                        && !(matches!(
                            proposed.state,
                            InternalState::Prepared | InternalState::Accepted
                        ) && matches!(
                            current.state,
                            InternalState::Accepted | InternalState::Confirmed
                        )))
                {
                    return Ok(current);
                }
            }
            proposed.revision = proposed.revision.saturating_add(1);
            let bytes = serde_json::to_vec(&proposed).map_err(|_| ApiError::unexpected())?;
            operations
                .insert(key.as_slice(), bytes.as_slice())
                .map_err(|_| ApiError::unavailable())?;
            proposed
        };
        tx.commit().map_err(|_| ApiError::unavailable())?;
        Ok(result)
    }

    pub fn response(record: &Record, depth: u64) -> OperationResponse {
        let transaction_id = if matches!(
            record.state,
            InternalState::Accepted | InternalState::Confirmed | InternalState::Settled
        ) {
            Some(record.expected_transaction_id.clone())
        } else {
            None
        };
        OperationResponse {
            operation_id: record.operation_id.clone(),
            expected_transaction_id: record.expected_transaction_id.clone(),
            transaction_id,
            status: record.state.public(),
            depth,
        }
    }

    pub async fn reconcile_one<L: Ledger>(
        &self,
        ledger: &L,
        key: OperationKey,
        record: &mut Record,
        tip_height: u64,
        tip_slot: u64,
        submit: bool,
    ) -> Result<u64, ApiError> {
        if record.state == InternalState::Settled {
            return Ok(record
                .inclusion_height
                .map(|height| tip_height.saturating_sub(height))
                .unwrap_or(0));
        }
        let txid =
            hex::decode(&record.expected_transaction_id).map_err(|_| ApiError::unexpected())?;
        let txid: [u8; 32] = txid.try_into().map_err(|_| ApiError::unexpected())?;
        let presence = ledger.read_tx(&txid).await?;
        match presence {
            Some(TxPresence { height }) => {
                let depth = tip_height.saturating_sub(height);
                record.inclusion_height = Some(height);
                record.state = finality(depth);
                if record.state == InternalState::Settled {
                    record.cbor = None;
                }
                self.put(key, record).await?;
                Ok(depth)
            }
            None => {
                if record.state == InternalState::Settled {
                    return Ok(0);
                }
                if record.ttl.is_none() {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map_err(|_| ApiError::unexpected())?
                        .as_secs();
                    if record.created_at_epoch_secs == 0
                        || now.saturating_sub(record.created_at_epoch_secs) >= 3600
                    {
                        record.state = InternalState::Rejected;
                        record.cbor = None;
                        self.put(key, record).await?;
                        return Ok(0);
                    }
                }
                if record.ttl.is_some_and(|ttl| tip_slot >= ttl) {
                    record.state = InternalState::Rejected;
                    record.cbor = None;
                    self.put(key, record).await?;
                    return Ok(0);
                }
                if record.inclusion_height.is_some() {
                    record.inclusion_height = None;
                    record.state = InternalState::Prepared;
                    self.put(key, record).await?;
                }
                if record.state == InternalState::Accepted {
                    return Ok(0);
                }
                if submit && record.cbor.is_some() {
                    let Some(claimed) = self.claim_submit(key).await? else {
                        return Ok(0);
                    };
                    *record = claimed;
                    let Some(cbor_hex) = record.cbor.as_ref() else {
                        return Ok(0);
                    };
                    let cbor = hex::decode(cbor_hex).map_err(|_| ApiError::unexpected())?;
                    match ledger.submit_cbor(&cbor).await? {
                        SubmitResult::Accepted(hash) if hash == txid => {
                            record.state = InternalState::Accepted;
                        }
                        SubmitResult::Accepted(_) => return Err(ApiError::unavailable()),
                        SubmitResult::Rejected => {
                            record.state = InternalState::Rejected;
                            record.cbor = None;
                        }
                        SubmitResult::AlreadyKnown
                        | SubmitResult::InputsSpent
                        | SubmitResult::Indeterminate => {}
                    }
                    record.submit_started_at = None;
                    self.put(key, record).await?;
                }
                Ok(0)
            }
        }
    }

    pub async fn pending_ids(&self) -> Result<Vec<OperationKey>, ApiError> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || Self::pending_ids_blocking(&db))
            .await
            .map_err(|_| ApiError::unavailable())?
    }

    pub async fn ready(&self) -> Result<(), ApiError> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let tx = db.begin_read().map_err(|_| ApiError::unavailable())?;
            let _ = tx
                .open_table(OPERATIONS)
                .map_err(|_| ApiError::unavailable())?;
            Ok(())
        })
        .await
        .map_err(|_| ApiError::unavailable())?
    }

    fn pending_ids_blocking(db: &Database) -> Result<Vec<OperationKey>, ApiError> {
        let tx = db.begin_read().map_err(|_| ApiError::unavailable())?;
        let table = tx
            .open_table(OPERATIONS)
            .map_err(|_| ApiError::unavailable())?;
        let mut ids = Vec::new();
        for entry in table.iter().map_err(|_| ApiError::unavailable())? {
            let (key, value) = entry.map_err(|_| ApiError::unavailable())?;
            let record: Record =
                serde_json::from_slice(value.value()).map_err(|_| ApiError::unexpected())?;
            if (record.state.is_pending() || record.state == InternalState::Confirmed)
                && key.value().len() == 17
            {
                let mut id = [0; 17];
                id.copy_from_slice(key.value());
                ids.push(OperationKey(id));
            }
        }
        Ok(ids)
    }

    fn count_pending(operations: &redb::Table<'_, &[u8], &[u8]>) -> Result<usize, ApiError> {
        let mut count = 0;
        for entry in operations.iter().map_err(|_| ApiError::unavailable())? {
            let (_, value) = entry.map_err(|_| ApiError::unavailable())?;
            let record: Record =
                serde_json::from_slice(value.value()).map_err(|_| ApiError::unexpected())?;
            if record.state.is_pending() {
                count += 1;
            }
        }
        Ok(count)
    }

    fn file_len(path: &Path) -> Result<u64, ApiError> {
        match std::fs::metadata(path) {
            Ok(meta) => Ok(meta.len()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
            Err(_) => Err(ApiError::unavailable()),
        }
    }
}

pub struct InflightGuard<'a> {
    store: &'a OpsStore,
}

impl Drop for InflightGuard<'_> {
    fn drop(&mut self) {
        self.store.inflight.fetch_sub(1, Ordering::Release);
    }
}

pub fn finality(depth: u64) -> InternalState {
    if depth >= SETTLED_DEPTH {
        InternalState::Settled
    } else if depth >= CONFIRMED_DEPTH {
        InternalState::Confirmed
    } else {
        InternalState::Accepted
    }
}

#[cfg(test)]
mod tests {
    use super::{InternalState, finality};

    #[test]
    fn depth_boundaries() {
        assert_eq!(finality(4), InternalState::Accepted);
        assert_eq!(finality(5), InternalState::Confirmed);
        assert_eq!(finality(2159), InternalState::Confirmed);
        assert_eq!(finality(2160), InternalState::Settled);
    }
}

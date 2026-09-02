use crate::error::ApiError;
use crate::providers::{Ledger, SubmitResult, TxPresence};
use crate::tx::SignedTx;
use crate::wire::OperationResponse;
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

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
}

pub struct OpsStore {
    db: Database,
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
        let db = Database::create(path)?;
        {
            let tx = db.begin_write()?;
            let _ = tx.open_table(OPERATIONS)?;
            let _ = tx.open_table(TX_IDS)?;
            tx.commit()?;
        }
        Ok(Self {
            db,
            max_pending,
            max_bytes,
            inflight: AtomicUsize::new(0),
            max_inflight,
        })
    }

    pub fn admit_write(&self) -> Result<InflightGuard<'_>, ApiError> {
        let current = self.inflight.load(Ordering::Relaxed);
        if current >= self.max_inflight {
            return Err(ApiError::too_many());
        }
        self.inflight.fetch_add(1, Ordering::Relaxed);
        Ok(InflightGuard { store: self })
    }

    pub fn get(&self, operation_id: &[u8; 16]) -> Result<Option<Record>, ApiError> {
        let tx = self.db.begin_read().map_err(|_| ApiError::unavailable())?;
        let table = tx
            .open_table(OPERATIONS)
            .map_err(|_| ApiError::unavailable())?;
        match table.get(operation_id.as_slice()) {
            Ok(Some(value)) => serde_json::from_slice(value.value())
                .map(Some)
                .map_err(|_| ApiError::unexpected()),
            Ok(None) => Ok(None),
            Err(_) => Err(ApiError::unavailable()),
        }
    }

    pub fn persist_new(
        &self,
        operation_id: &[u8; 16],
        txid: &[u8; 32],
        signed: &SignedTx,
        uuid: &str,
    ) -> Result<Record, ApiError> {
        if self.file_len()? > self.max_bytes {
            return Err(ApiError::too_many());
        }
        let digest = hex::encode(signed.digest);
        let expected = hex::encode(txid);
        let tx = self.db.begin_write().map_err(|_| ApiError::unavailable())?;
        {
            let mut operations = tx
                .open_table(OPERATIONS)
                .map_err(|_| ApiError::unavailable())?;
            let mut tx_ids = tx.open_table(TX_IDS).map_err(|_| ApiError::unavailable())?;
            if let Some(existing) = operations
                .get(operation_id.as_slice())
                .map_err(|_| ApiError::unavailable())?
            {
                let record: Record =
                    serde_json::from_slice(existing.value()).map_err(|_| ApiError::unexpected())?;
                if record.digest == digest && record.expected_transaction_id == expected {
                    return Ok(record);
                }
                return Err(ApiError::conflict());
            }
            if let Some(owner) = tx_ids
                .get(txid.as_slice())
                .map_err(|_| ApiError::unavailable())?
            {
                if owner.value() != operation_id.as_slice() {
                    return Err(ApiError::conflict());
                }
            }
            if self.count_pending(&operations)? >= self.max_pending {
                return Err(ApiError::too_many());
            }
            let record = Record {
                operation_id: uuid.to_string(),
                expected_transaction_id: expected,
                digest,
                cbor: Some(hex::encode(&signed.bytes)),
                ttl: signed.ttl,
                state: InternalState::Prepared,
                inclusion_height: None,
                attempts: 0,
            };
            let bytes = serde_json::to_vec(&record).map_err(|_| ApiError::unexpected())?;
            operations
                .insert(operation_id.as_slice(), bytes.as_slice())
                .map_err(|_| ApiError::unavailable())?;
            tx_ids
                .insert(txid.as_slice(), operation_id.as_slice())
                .map_err(|_| ApiError::unavailable())?;
            drop(operations);
            drop(tx_ids);
            tx.commit().map_err(|_| ApiError::unavailable())?;
            Ok(record)
        }
    }

    pub fn claim_submit(&self, operation_id: &[u8; 16]) -> Result<Option<Record>, ApiError> {
        let tx = self.db.begin_write().map_err(|_| ApiError::unavailable())?;
        let record = {
            let mut operations = tx
                .open_table(OPERATIONS)
                .map_err(|_| ApiError::unavailable())?;
            let Some(existing) = operations
                .get(operation_id.as_slice())
                .map_err(|_| ApiError::unavailable())?
            else {
                return Ok(None);
            };
            let bytes = existing.value().to_vec();
            drop(existing);
            let mut record: Record =
                serde_json::from_slice(&bytes).map_err(|_| ApiError::unexpected())?;
            if record.state != InternalState::Prepared {
                return Ok(Some(record));
            }
            record.state = InternalState::Submitting;
            record.attempts += 1;
            let stored = serde_json::to_vec(&record).map_err(|_| ApiError::unexpected())?;
            operations
                .insert(operation_id.as_slice(), stored.as_slice())
                .map_err(|_| ApiError::unavailable())?;
            record
        };
        tx.commit().map_err(|_| ApiError::unavailable())?;
        Ok(Some(record))
    }

    pub fn put(&self, operation_id: &[u8; 16], record: &Record) -> Result<(), ApiError> {
        let tx = self.db.begin_write().map_err(|_| ApiError::unavailable())?;
        {
            let mut operations = tx
                .open_table(OPERATIONS)
                .map_err(|_| ApiError::unavailable())?;
            let bytes = serde_json::to_vec(record).map_err(|_| ApiError::unexpected())?;
            operations
                .insert(operation_id.as_slice(), bytes.as_slice())
                .map_err(|_| ApiError::unavailable())?;
        }
        tx.commit().map_err(|_| ApiError::unavailable())?;
        Ok(())
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
        operation_id: &[u8; 16],
        record: &mut Record,
        tip_height: u64,
        tip_slot: u64,
        submit: bool,
    ) -> Result<u64, ApiError> {
        let txid =
            hex::decode(&record.expected_transaction_id).map_err(|_| ApiError::unexpected())?;
        let txid: [u8; 32] = txid.try_into().map_err(|_| ApiError::unexpected())?;
        let presence = ledger.read_tx(&txid).await?;
        match presence {
            Some(TxPresence { height }) => {
                let depth = tip_height.saturating_sub(height);
                record.inclusion_height = Some(height);
                record.state = finality(depth);
                if matches!(
                    record.state,
                    InternalState::Confirmed | InternalState::Settled
                ) {
                    record.cbor = None;
                }
                self.put(operation_id, record)?;
                Ok(depth)
            }
            None => {
                if record.ttl.is_some_and(|ttl| tip_slot > ttl) {
                    record.state = InternalState::Rejected;
                    record.cbor = None;
                    self.put(operation_id, record)?;
                    return Ok(0);
                }
                if record.inclusion_height.is_some() {
                    record.inclusion_height = None;
                    record.state = InternalState::Prepared;
                    self.put(operation_id, record)?;
                    return Ok(0);
                }
                if submit && record.cbor.is_some() && record.state.is_pending() {
                    if record.state == InternalState::Prepared {
                        if let Some(claimed) = self.claim_submit(operation_id)? {
                            *record = claimed;
                        }
                    }
                    let Some(cbor_hex) = record.cbor.as_ref() else {
                        return Ok(0);
                    };
                    let cbor = hex::decode(cbor_hex).map_err(|_| ApiError::unexpected())?;
                    match ledger.submit_cbor(&cbor).await? {
                        SubmitResult::Accepted(hash) if hash == txid => {
                            record.state = InternalState::Accepted;
                        }
                        SubmitResult::Accepted(_)
                        | SubmitResult::AlreadyKnown
                        | SubmitResult::InputsSpent
                        | SubmitResult::Indeterminate => {}
                    }
                    self.put(operation_id, record)?;
                }
                Ok(0)
            }
        }
    }

    pub fn pending_ids(&self) -> Result<Vec<[u8; 16]>, ApiError> {
        let tx = self.db.begin_read().map_err(|_| ApiError::unavailable())?;
        let table = tx
            .open_table(OPERATIONS)
            .map_err(|_| ApiError::unavailable())?;
        let mut ids = Vec::new();
        for entry in table.iter().map_err(|_| ApiError::unavailable())? {
            let (key, value) = entry.map_err(|_| ApiError::unavailable())?;
            let record: Record =
                serde_json::from_slice(value.value()).map_err(|_| ApiError::unexpected())?;
            if record.state.is_pending() || record.state == InternalState::Confirmed {
                let mut id = [0u8; 16];
                id.copy_from_slice(key.value());
                ids.push(id);
            }
        }
        Ok(ids)
    }

    fn count_pending(&self, operations: &redb::Table<'_, &[u8], &[u8]>) -> Result<usize, ApiError> {
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

    fn file_len(&self) -> Result<u64, ApiError> {
        Ok(0)
        // ponytail: redb 4 has no cheap size API here; admission uses pending count. Add file metadata if disk pressure shows up.
    }
}

pub struct InflightGuard<'a> {
    store: &'a OpsStore,
}

impl Drop for InflightGuard<'_> {
    fn drop(&mut self) {
        self.store.inflight.fetch_sub(1, Ordering::Relaxed);
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
    use super::InternalState;
    use super::finality;

    #[test]
    fn depth_boundaries() {
        assert_eq!(finality(4), InternalState::Accepted);
        assert_eq!(finality(5), InternalState::Confirmed);
        assert_eq!(finality(2159), InternalState::Confirmed);
        assert_eq!(finality(2160), InternalState::Settled);
    }
}

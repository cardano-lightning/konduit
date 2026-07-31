use std::collections::HashMap;

use kupo_client::{Match, MatchBound, MatchFilters};

use crate::{
    store::{self, IndexedTransactions, IndexedTransactionsError, Store, StoreQueries},
    transaction::{Block, SlotNo, Transaction, TransactionId},
};

pub struct BlockDepth(pub u32);

pub struct Config {
    /// How many times to retry the sync pass if we encounter
    /// an ETag mismatch before the end of the procedure.
    pub max_sync_retries: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum IndexerError {
    #[error("Failed to fetch ETag from Kupo")]
    EtagFetchError,
    #[error("Unable to finalize sync pass with consistent ETag")]
    EtagMismatch,
    #[error("Internal error: {0}")]
    InternalError(&'static str),
    #[error(transparent)]
    KupoError(#[from] kupo_client::Error),
    #[error(transparent)]
    StoreError(#[from] store::StoreError),
    #[error("Failed to fetch tip from Kupo")]
    TipFetchError,
    #[error(transparent)]
    TransactionDecodingError(#[from] crate::transaction::TransactionDecodingError),
    #[error(transparent)]
    IndexedTransactionsError(#[from] IndexedTransactionsError),
}

pub type Result<T> = std::result::Result<T, IndexerError>;

pub struct Indexer<S> {
    config: Config,
    kupo: kupo_client::blocking::Client,
    store: S,
}

impl<S: Store> Indexer<S> {
    pub fn new(store: S, kupo: kupo_client::blocking::Client, config: Config) -> Self {
        Self {
            store,
            kupo,
            config,
        }
    }
    pub fn sync(&mut self) -> Result<()> {
        self.store.with_queries(|queries| {
            let mut sync =
                IndexerSyncLoop::new(self.kupo.clone(), queries, self.config.max_sync_retries);
            sync.run()
        })
    }
}

// Kupo returns an ETag header on every response indicating
// the current state of the chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Etag(pub [u8; 32]);

impl From<&kupo_client::Blake2b256> for Etag {
    fn from(value: &kupo_client::Blake2b256) -> Self {
        Self(value.0)
    }
}

pub struct LocalTip(pub Block);

pub struct RemoteTip(pub Block);

pub enum SyncStatus {
    InSync,
    OutOfSync {
        local_tip: Option<LocalTip>,
        remote_tip: RemoteTip,
    },
}

impl SyncStatus {
    pub fn new(local_tip: Option<LocalTip>, remote_tip: RemoteTip) -> Self {
        if local_tip.as_ref().map(|t| t.0.slot_no) == Some(remote_tip.0.slot_no) {
            SyncStatus::InSync
        } else {
            SyncStatus::OutOfSync {
                local_tip,
                remote_tip,
            }
        }
    }
}

// We consider the indexing pass to be successful
// if all the Kupo requests return the same ETag.
pub struct SyncState {
    pub kupo_etag: Option<Etag>,
    pub attempt: u32,
}

struct IndexerSyncLoop<'a, Q: ?Sized> {
    kupo: kupo_client::blocking::Client,
    max_retries: u32,
    queries: &'a mut Q,
}

impl<'a, Q: ?Sized + StoreQueries> IndexerSyncLoop<'a, Q> {
    fn new(kupo: kupo_client::blocking::Client, queries: &'a mut Q, max_retries: u32) -> Self {
        Self {
            kupo,
            max_retries,
            queries,
        }
    }

    fn extract_response_etag<T>(&self, response: &kupo_client::KupoResponse<T>) -> Result<Etag> {
        let etag = response
            .etag
            .as_ref()
            .ok_or(IndexerError::EtagFetchError)
            .map(Etag::from)?;
        Ok(etag)
    }

    fn extract_response_body<T>(
        &mut self,
        expected_etag: &Etag,
        response: kupo_client::KupoResponse<T>,
    ) -> Result<T> {
        let etag = self.extract_response_etag(&response)?;
        if etag == *expected_etag {
            Ok(response.body)
        } else {
            Err(IndexerError::EtagMismatch)
        }
    }

    fn fetch_remote_tip(&mut self, expected_etag: &Etag) -> Result<RemoteTip> {
        let response = self.kupo.checkpoints()?;
        let body = self.extract_response_body(expected_etag, response)?;
        match body.into_iter().next() {
            Some(tip) => Ok(RemoteTip(tip.into())),
            None => Err(IndexerError::EtagFetchError),
        }
    }

    /// Reconcile our local tip with Kupo's tip.
    /// - If we have no local blocks, the common tip is `0`.
    /// - Otherwise, query Kupo for the checkpoint at our local tip's slot
    ///   and verify the header hash matches. If it does, our tip slot is
    ///   the common one. If it doesn't (or the checkpoint is missing),
    ///   drop the local block and retry, up to `max_rollback_retries`.
    pub fn reconcile_local_tip(&mut self) -> Result<(Etag, SyncStatus)> {
        let (etag, local_tip) = loop {
            let response = self.queries.get_tip()?;
            match response {
                Some(local_tip) => {
                    let response = self
                        .kupo
                        .checkpoint(local_tip.slot_no.0, kupo_client::Strict(true))?;
                    let etag = self.extract_response_etag(&response)?;
                    let missing = match response.body {
                        None => true,
                        Some(checkpoint) => checkpoint.header_hash.0 != local_tip.header_hash.0,
                    };
                    if missing {
                        eprintln!(
                            "indexer: local tip {} has no Kupo checkpoint or hash mismatch; rolling back",
                            local_tip.slot_no
                        );
                        self.queries.rollback_block(local_tip.slot_no)?;
                    } else {
                        break (etag, Some(LocalTip(local_tip)));
                    }
                }
                None => {
                    let response = self.kupo.health()?;
                    let etag = self.extract_response_etag(&response)?;
                    break (etag, None);
                }
            }
        };
        let remote_tip = self.fetch_remote_tip(&etag)?;
        Ok((etag, SyncStatus::new(local_tip, remote_tip)))
    }

    fn sync_attempt(&mut self) -> Result<()> {
        let (etag, sync_status) = self.reconcile_local_tip()?;
        match sync_status {
            SyncStatus::InSync => {
                eprintln!("indexer: local tip matches remote tip; nothing to do");
                Ok(())
            }
            SyncStatus::OutOfSync {
                local_tip,
                remote_tip,
            } => {
                let (indexed_slot, not_indexed_slot) = match local_tip {
                    Some(LocalTip(local_tip)) => {
                        (Some(local_tip.slot_no), local_tip.slot_no.succ())
                    }
                    None => (None, SlotNo(0)),
                };

                let response = self.kupo.all_matches(
                    &MatchFilters::new()
                        .with_created_after(MatchBound::Slot(not_indexed_slot.0))
                        .with_resolve_hashes(true),
                )?;
                let new_outputs = self.extract_response_body(&etag, response)?;

                let old_outputs_spends = match indexed_slot {
                    None => Ok(Vec::new()),
                    Some(indexed_slot) => {
                        let response = self.kupo.all_matches(
                            &MatchFilters::new()
                                .with_created_before(MatchBound::Slot(indexed_slot.0))
                                .with_spent_after(MatchBound::Slot(not_indexed_slot.0))
                                .with_resolve_hashes(true),
                        )?;
                        self.extract_response_body(&etag, response)
                    }
                }?;

                // All spends that happend since the last indexed slot including existing
                // outputs and newly created ones.
                let mut all_new_spends: HashMap<TransactionId, Vec<Match>> = HashMap::new();
                for spent in old_outputs_spends {
                    match spent.spent_at.as_ref() {
                        Some(spent_at) => {
                            let transaction_id = spent_at.transaction_id.into();
                            all_new_spends
                                .entry(transaction_id)
                                .or_default()
                                .push(spent);
                        }
                        None => {
                            return Err(IndexerError::InternalError(
                                "indexer: spent output has no spent_at",
                            ));
                        }
                    }
                }
                for output in &new_outputs {
                    let Some(spent_at) = output.spent_at.as_ref() else {
                        continue;
                    };
                    let transaction_id = spent_at.transaction_id.into();
                    all_new_spends
                        .entry(transaction_id)
                        .or_default()
                        .push(output.clone());
                }

                let mut transactions: Vec<(Vec<Match>, Vec<Match>)> = Vec::new();
                let mut current_tx_id: Option<kupo_client::Blake2b256> = None;
                let mut current_tx_outputs: Vec<Match> = Vec::new();

                // Kupo returns outputs ordered by creation time and the transaction
                // output index so they are ready to process in a single pass.
                for output in new_outputs {
                    match current_tx_id {
                        Some(tx_id) => {
                            if output.transaction_id == tx_id {
                                current_tx_outputs.push(output);
                            } else {
                                let inputs =
                                    all_new_spends.remove(&tx_id.into()).unwrap_or_default();
                                transactions.push((inputs, current_tx_outputs));
                                current_tx_id = Some(output.transaction_id);
                                current_tx_outputs = vec![output];
                            }
                        }
                        None => {
                            current_tx_id = Some(output.transaction_id);
                            current_tx_outputs.push(output);
                        }
                    }
                }

                if let Some(tx_id) = current_tx_id {
                    let inputs = all_new_spends.remove(&tx_id.into()).unwrap_or_default();
                    transactions.push((inputs, current_tx_outputs));
                }

                for (_transaction_id, inputs) in all_new_spends {
                    transactions.push((inputs, Vec::new()));
                }

                let transactions = transactions
                    .into_iter()
                    .map(|(inputs, outputs)| Transaction::from_kupo_matches(inputs, outputs))
                    .collect::<std::result::Result<Vec<_>, _>>()?;

                let indexed_transactions =
                    IndexedTransactions::new(remote_tip.0.clone(), transactions)?;
                self.queries.insert_transactions(&indexed_transactions)?;
                Ok(())
            }
        }
    }

    pub fn run(&mut self) -> std::result::Result<(), IndexerError> {
        let mut attempt = 1;
        loop {
            match self.sync_attempt() {
                Ok(_) => break Ok(()),
                Err(err @ IndexerError::EtagMismatch) => {
                    if attempt >= self.max_retries {
                        break Err(err);
                    }
                    attempt += 1;
                }
                Err(e) => break Err(e),
            }
        }
    }
}

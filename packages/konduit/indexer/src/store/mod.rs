//! Introduces the [`Store`] trait which abstracts over the underlying database implementation.
//!
//! It is a rough abstraction layer suited for the SQL like backend as the types exposed are flat.

use std::collections::{HashMap, HashSet};

use cardano_sdk::VerificationKey;
use itertools::{Either, Itertools};
use konduit_data::{Datum, Redeemer, Tag};
use thiserror::Error;

use crate::transaction::{
    Block, BlockHeaderHash, BlockNo, Input, Lovelace, NonMutual, OutputIndex, ScriptHash, SlotNo,
    Steps, Transaction, TransactionId, TransactionIndex, TxOutRef,
};

pub mod sqlite;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChannelId {
    pub transaction_id: TransactionId,
    pub output_index: OutputIndex,
}

#[derive(Debug, Clone)]
pub struct NewChannel<'a> {
    pub add_vkey: &'a VerificationKey,
    pub block_slot_no: SlotNo,
    pub datum: &'a Datum,
    pub lovelace: Lovelace,
    pub output_index: OutputIndex,
    pub script_hash: &'a ScriptHash,
    pub sub_vkey: &'a VerificationKey,
    pub tag: &'a Tag,
    pub transaction_id: &'a TransactionId,
    pub transaction_index: TransactionIndex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewFocus<'a> {
    pub add_vkey: &'a VerificationKey,
    pub channel_transaction_id: &'a TransactionId,
    pub channel_transaction_index: TransactionIndex,
    pub tag: &'a Tag,
}

#[derive(Debug, Clone)]
pub enum NewStep<'a> {
    NewCloseStep {
        block_slot_no: SlotNo,
        channel_output_index: OutputIndex,
        channel_transaction_id: &'a TransactionId,
        redeemer: &'a Redeemer,
        transaction_id: &'a TransactionId,
        transaction_index: TransactionIndex,
    },
    NewTransitionStep {
        block_slot_no: SlotNo,
        channel_output_index: OutputIndex,
        channel_transaction_id: &'a TransactionId,
        datum: &'a Datum,
        lovelace: Lovelace,
        output_index: OutputIndex,
        redeemer: &'a Redeemer,
        transaction_id: &'a TransactionId,
        transaction_index: TransactionIndex,
    },
}

#[derive(Debug, Clone)]
pub struct BlockRow {
    pub block_no: BlockNo,
    pub header_hash: BlockHeaderHash,
    pub slot_no: SlotNo,
}

#[derive(Debug, Clone)]
pub struct ChannelRow {
    pub add_vkey: VerificationKey,
    pub block_slot_no: SlotNo,
    pub datum: Datum,
    pub lovelace: Lovelace,
    pub output_index: OutputIndex,
    pub script_hash: ScriptHash,
    pub sub_vkey: VerificationKey,
    pub tag: Tag,
    pub transaction_id: TransactionId,
}

#[derive(Debug, Clone)]
pub struct ThreadOutput {
    pub block_no: BlockNo,
    pub block_slot_no: SlotNo,
    pub transaction_id: TransactionId,
    pub output_index: OutputIndex,
    pub datum: Datum,
    pub step: Option<(Redeemer, Option<Box<ThreadOutput>>)>,
}

impl ThreadOutput {
    pub fn is_thread_closed(&self) -> bool {
        let mut current = self;
        loop {
            match &current.step {
                Some((_, Some(step))) => current = step,
                Some((_, None)) => return true,
                None => return false,
            }
        }
    }

    pub fn curr_thread_state(&self) -> Option<&konduit_data::Datum> {
        let mut current = self;
        loop {
            match &current.step {
                Some((_, Some(step))) => current = step,
                Some((_, None)) => return None,
                None => return Some(&current.datum),
            }
        }
    }
}

pub type Tip = Block;

// Non empty set of transactions, ordered by block slot number and transaction index.
pub struct IndexedTransactions {
    indexed_against: Tip,
    transactions: Vec<Transaction>,
}

#[derive(Debug, Error)]
pub enum IndexedTransactionsError {
    #[error("Transactions are indexed against an older tip")]
    TransactionsIndexedAgainstOlderTip,
}

impl IndexedTransactions {
    pub fn new(
        indexed_against: Tip,
        transactions: Vec<Transaction>,
    ) -> std::result::Result<Self, IndexedTransactionsError> {
        let mut transactions = transactions;
        transactions.sort_by_key(|tx| (tx.block.slot_no, tx.transaction_index));
        if transactions
            .last()
            .is_some_and(|tx| tx.block.slot_no > indexed_against.slot_no)
        {
            return Err(IndexedTransactionsError::TransactionsIndexedAgainstOlderTip);
        }
        Ok(Self {
            indexed_against,
            transactions,
        })
    }

    pub fn iter_blocks(&self) -> impl Iterator<Item = &Block> {
        self.transactions
            .iter()
            .map(|tx| &tx.block)
            .chain(std::iter::once(&self.indexed_against))
            .dedup_by(|a, b| a.slot_no == b.slot_no)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Transaction> {
        self.transactions.iter()
    }

    pub fn blocks_tip(&self) -> Option<&Block> {
        let last_tx = self.transactions.last();
        last_tx.map(|tx| &tx.block)
    }

    pub fn tip(&self) -> &Block {
        &self.indexed_against
    }
}

#[derive(Debug, Clone)]
pub struct Thread(ThreadOutput);

impl Thread {
    pub fn channel_id(&self) -> ChannelId {
        ChannelId {
            transaction_id: self.0.transaction_id,
            output_index: self.0.output_index,
        }
    }

    pub fn initial_state(&self) -> &konduit_data::Datum {
        &self.0.datum
    }

    pub fn is_closed(&self) -> bool {
        self.0.is_thread_closed()
    }

    pub fn curr_state(&self) -> Option<&konduit_data::Datum> {
        self.0.curr_thread_state()
    }
}

pub enum ThreadItem<'a> {
    InitialOutput(&'a ThreadOutput),
    ContStep(&'a Redeemer, &'a ThreadOutput),
    EolStep(&'a Redeemer),
}

pub type Continuation = (Redeemer, Option<Box<ThreadOutput>>);

impl<'a> TryFrom<Either<&'a ThreadOutput, Option<&'a Continuation>>> for ThreadItem<'a> {
    type Error = ();

    fn try_from(
        value: Either<&'a ThreadOutput, Option<&'a Continuation>>,
    ) -> Result<Self, Self::Error> {
        match value {
            Either::Left(thread_output) => Ok(ThreadItem::InitialOutput(thread_output)),
            Either::Right(Some((redeemer, Some(next_thread_output)))) => {
                Ok(ThreadItem::ContStep(redeemer, next_thread_output))
            }
            Either::Right(Some((redeemer, None))) => Ok(ThreadItem::EolStep(redeemer)),
            Either::Right(None) => Err(()),
        }
    }
}

pub struct ThreadIterator<'a> {
    next: Either<&'a ThreadOutput, Option<&'a Continuation>>,
}

impl<'a> Iterator for ThreadIterator<'a> {
    type Item = ThreadItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let item = ThreadItem::try_from(self.next).ok()?;
        self.next = match item {
            ThreadItem::InitialOutput(thread_output) => Either::Right(thread_output.step.as_ref()),
            ThreadItem::ContStep(_redeemer, thread_output) => {
                Either::Right(thread_output.step.as_ref())
            }
            ThreadItem::EolStep(_redeemer) => Either::Right(None),
        };
        Some(item)
    }
}

impl<'a> IntoIterator for &'a Thread {
    type Item = ThreadItem<'a>;
    type IntoIter = ThreadIterator<'a>;
    fn into_iter(self) -> Self::IntoIter {
        ThreadIterator {
            next: Either::Left(&self.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Threads {
    pub focus: Option<Thread>,
    pub others: Vec<Thread>,
}

#[derive(Debug, Error)]
pub enum StoreError {
    // When `NonMutual` is missing it means that the input was consumed by a mutual transaction.
    #[error("Missing channel for input {input:?} and non-mutual step: {step:?}")]
    ChannelNotFound {
        input: Box<Input>,
        step: Option<Box<NonMutual>>,
    },
    #[error("Store error: {0}")]
    StoreError(String), // Errors like connection issues, query errors, etc.
    #[error("Store is corrupted with some invalid data: {0}")]
    StoreCorruped(String), // Errors indicating that the store contains invalid or unexpected data.
}

/// We separate [`StoreQueries`] from the [`Store`] trait to leave
/// room for automatic db transaction management around batch of the queries.
/// We borrow exclusive reference to the store for the duration
/// of the batch processing which locks the object. By separating
/// those two we gain some flexibility around usage of [`StoreQueries`] methods etc.
pub trait Store {
    // There should be conversion from StoreError to the desired E
    fn with_queries<F, T, E: From<StoreError>>(&mut self, f: F) -> std::result::Result<T, E>
    where
        F: FnMut(&mut dyn StoreQueries) -> std::result::Result<T, E>;
}

pub type Result<T, E = StoreError> = std::result::Result<T, E>;

pub trait StoreQueries {
    fn get_block(&self, slot_no: SlotNo) -> Result<Option<Block>>;
    fn get_channel(&self, id: &ChannelId) -> Result<Option<ChannelRow>>;
    fn get_channel_ids_by_keytag(
        &self,
        add_vkey: &VerificationKey,
        tag: &Tag,
    ) -> Result<Vec<ChannelId>>;
    fn get_channel_ids_by_outputs(
        &self,
        output_refs: &[TxOutRef],
    ) -> Result<HashMap<TxOutRef, ChannelId>>;
    fn get_threads_by_keytag(&self, add_vkey: &VerificationKey, tag: &Tag) -> Result<Threads>;

    fn get_tip(&self) -> Result<Option<Block>>;

    // Given a coherent batch of konduit transactions (i.e. for every block the full set of
    // transactions is present), insert corresponding blocks, channels and steps into the store.
    // The set should only contain transactions which are not already present in the store
    // (collected after the last tip).
    fn insert_transactions(&self, transactions: &IndexedTransactions) -> Result<()> {
        // Print the number of blocks
        for block in transactions.iter_blocks() {
            println!("Inserting block: {}", block);
        }
        for block in transactions.iter_blocks() {
            self.insert_block(block)?;
        }
        // FIXME: We can keep this fetch here or do an aggregate
        // fetch at the beginning and mutate as we go.
        let mut input2channel: HashMap<TxOutRef, ChannelId> = {
            let transaction_ids = transactions
                .iter()
                .map(|transaction| transaction.transaction_id)
                .collect::<HashSet<_>>();

            // Initial set of UTxOs which should be already
            // present in the store.
            let old_inputs = transactions
                .iter()
                .flat_map(|transaction| {
                    match transaction.steps() {
                        Some(Steps::Mutual(input)) => vec![input.tx_out_ref],
                        Some(Steps::Batch(batch)) => {
                            batch.iter().map(|(input, _)| input.tx_out_ref).collect()
                        }
                        None => vec![],
                    }
                    .into_iter()
                    .filter(|tx_out_ref| !transaction_ids.contains(&tx_out_ref.0))
                })
                .collect::<Vec<_>>();
            self.get_channel_ids_by_outputs(old_inputs.as_slice())?
        };

        for transaction in transactions.iter() {
            for output in transaction.opens().iter() {
                let add_vkey = &output.datum.constants.add_vkey.into();
                let sub_vkey = &output.datum.constants.sub_vkey.into();
                let tag = &output.datum.constants.tag;
                let channel = NewChannel {
                    add_vkey,
                    block_slot_no: transaction.block.slot_no,
                    datum: &output.datum,
                    lovelace: output.lovelace,
                    output_index: output.output_index,
                    script_hash: &output.script_hash,
                    sub_vkey,
                    tag,
                    transaction_id: &transaction.transaction_id,
                    transaction_index: transaction.transaction_index,
                };
                self.insert_channel(channel)?;
                input2channel.insert(
                    TxOutRef(transaction.transaction_id, output.output_index),
                    ChannelId {
                        transaction_id: transaction.transaction_id,
                        output_index: output.output_index,
                    },
                );
            }

            match transaction.steps() {
                None => {}
                // This closes the channel
                Some(Steps::Mutual(input)) => {
                    let channel_id = input2channel.get(&input.tx_out_ref).ok_or_else(|| {
                        StoreError::ChannelNotFound {
                            input: input.clone(),
                            step: None,
                        }
                    })?;
                    let step = NewStep::NewCloseStep {
                        block_slot_no: transaction.block.slot_no,
                        channel_output_index: channel_id.output_index,
                        channel_transaction_id: &channel_id.transaction_id,
                        redeemer: &input.redeemer,
                        transaction_id: &transaction.transaction_id,
                        transaction_index: transaction.transaction_index,
                    };
                    self.insert_step(step)?;
                }
                Some(Steps::Batch(batch)) => {
                    for (input, non_mutual) in batch {
                        let channel_id = input2channel.get(&input.tx_out_ref).ok_or_else(|| {
                            StoreError::ChannelNotFound {
                                input: Box::new(input.clone()),
                                step: Some(Box::new(non_mutual.clone())),
                            }
                        })?;
                        match non_mutual {
                            NonMutual::Cont(boxed) => {
                                let (_cont, output) = boxed.as_ref();
                                let step = NewStep::NewTransitionStep {
                                    block_slot_no: transaction.block.slot_no,
                                    channel_output_index: channel_id.output_index,
                                    channel_transaction_id: &channel_id.transaction_id,
                                    datum: &output.datum,
                                    lovelace: output.lovelace,
                                    output_index: output.output_index,
                                    redeemer: &input.redeemer,
                                    transaction_id: &transaction.transaction_id,
                                    transaction_index: transaction.transaction_index,
                                };
                                self.insert_step(step)?;
                                input2channel.insert(
                                    TxOutRef(transaction.transaction_id, output.output_index),
                                    *channel_id,
                                );
                            }
                            NonMutual::Eol(_eol) => {
                                let step = NewStep::NewCloseStep {
                                    block_slot_no: transaction.block.slot_no,
                                    channel_output_index: channel_id.output_index,
                                    channel_transaction_id: &channel_id.transaction_id,
                                    redeemer: &input.redeemer,
                                    transaction_id: &transaction.transaction_id,
                                    transaction_index: transaction.transaction_index,
                                };
                                self.insert_step(step)?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    fn insert_block(&self, block: &Block) -> Result<()>;
    fn insert_channel(&self, channel: NewChannel) -> Result<()>;
    fn insert_step(&self, step: NewStep) -> Result<()>;
    fn rollback_block(&self, slot_no: SlotNo) -> Result<()>;
    fn set_focus(&self, focus: NewFocus) -> Result<()>;
}

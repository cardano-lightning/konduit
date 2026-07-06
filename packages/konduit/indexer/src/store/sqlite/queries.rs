use crate::error::Result;
use rusqlite::{Connection, OptionalExtension, Row};

/// On-chain reference that identifies a single channel instance:
/// This is distinct from a `keytag` (`add_vkey + tag`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChannelId {
    pub transaction_id: [u8; 32],
    pub output_index: u64,
}

/// Input shape for `INSERT INTO block`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewBlock {
    pub block_no: u64,
    pub header_hash: [u8; 32],
    pub slot_no: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewChannel {
    pub add_vkey: [u8; 32],
    pub block_slot_no: u64,
    pub datum: Vec<u8>,
    pub lovelace: u64,
    pub output_index: u64,
    pub script_hash: [u8; 28],
    pub sub_vkey: [u8; 32],
    pub tag: Vec<u8>,
    pub transaction_id: [u8; 32],
    pub transaction_index: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewStepRecord {
    NewCloseStep {
        block_slot_no: u64,
        channel_transaction_id: [u8; 32],
        channel_output_index: u64,
        redeemer: Vec<u8>,
        transaction_id: [u8; 32],
        transaction_index: u64,
    },
    NewTransitionStep {
        block_slot_no: u64,
        channel_transaction_id: [u8; 32],
        channel_output_index: u64,
        datum: Vec<u8>,
        lovelace: u64,
        output_index: u64,
        redeemer: Vec<u8>,
        transaction_id: [u8; 32],
        transaction_index: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockRecord {
    pub header_hash: [u8; 32],
    pub block_no: u64,
    pub slot_no: u64,
}

/// Output shape for `SELECT FROM channel`.
///
/// `keytag` is computed on the Rust side as `add_vkey || tag` rather than
/// read from the SQL `keytag` virtual column. SQLite's `||` operator on
/// two BLOBs is reported as `TEXT` by `sqlite3_column_type`, so reading it
/// back through rusqlite would force a lossy UTF-8 round-trip. Computing
/// it in Rust preserves the binary identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelRecord {
    pub add_vkey: [u8; 32],
    pub block_slot_no: u64,
    pub datum: Vec<u8>,
    pub lovelace: u64,
    pub output_index: u64,
    pub script_hash: [u8; 28],
    pub sub_vkey: [u8; 32],
    pub tag: Vec<u8>,
    pub transaction_id: [u8; 32],
}

impl ChannelRecord {
    /// Compute the `keytag` (concatenation of `add_vkey` and `tag`) for this
    /// record. Mirrors the SQL `keytag` virtual column.
    pub fn keytag(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.add_vkey.len() + self.tag.len());
        out.extend_from_slice(&self.add_vkey);
        out.extend_from_slice(&self.tag);
        out
    }
}

/// Output shape for `SELECT FROM step`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepRecord {
    TransitionStep {
        block_slot_no: u64,
        channel_slot_no: u64,
        channel_tx_index: u64,
        transaction_id: [u8; 32],
        datum: Vec<u8>,
        lovelace: u64,
        output_index: u64,
        redeemer: Vec<u8>,
    },
    CloseStep {
        block_slot_no: u64,
        channel_slot_no: u64,
        channel_tx_index: u64,
        transaction_id: [u8; 32],
        redeemer: Vec<u8>,
    },
}

/// Cross-table read: a channel together with all of its steps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadRecord {
    pub channel: ChannelRecord,
    pub steps: Vec<StepRecord>,
}

// ----------------------------------------------------------------- Queries

/// Close-to-DB handle: borrows the connection and exposes typed
/// insertion and lookup methods.
pub struct Queries<'a> {
    conn: &'a Connection,
}

impl<'a> Queries<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Insert a new block. The block's `slot_no` must not already exist
    /// (the table's `PRIMARY KEY (slot_no)` enforces this).
    pub fn insert_block(&self, block: &NewBlock) -> Result<()> {
        self.conn.execute(
            "INSERT INTO block (header_hash, block_no, slot_no) VALUES (?, ?, ?)",
            rusqlite::params![block.header_hash, block.block_no, block.slot_no],
        )?;
        Ok(())
    }

    /// Insert a new channel. `block_slot_no` must already exist in `block` (FK).
    pub fn insert_channel(&self, channel: &NewChannel) -> Result<()> {
        self.conn.execute(
            "INSERT INTO channel (\
                add_vkey,\
                block_slot_no,\
                datum,\
                lovelace,\
                output_index,\
                script_hash,\
                sub_vkey,\
                tag,\
                transaction_id,\
                transaction_index\
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                channel.add_vkey,
                channel.block_slot_no,
                channel.datum,
                channel.lovelace,
                channel.output_index,
                channel.script_hash,
                channel.sub_vkey,
                channel.tag,
                channel.transaction_id,
                channel.transaction_index
            ],
        )?;
        Ok(())
    }

    pub fn insert_step(&self, step: &NewStepRecord) -> Result<()> {
        match step {
            NewStepRecord::NewCloseStep {
                block_slot_no,
                channel_transaction_id,
                channel_output_index,
                redeemer,
                transaction_id,
                transaction_index,
            } => self.conn.execute(
                "INSERT INTO step (\
                        block_slot_no,\
                        channel_transaction_id,\
                        channel_output_index,\
                        redeemer,\
                        transaction_id,\
                        transaction_index\
                     ) VALUES (?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    block_slot_no,
                    channel_transaction_id,
                    channel_output_index,
                    redeemer,
                    transaction_id,
                    transaction_index
                ],
            ),
            NewStepRecord::NewTransitionStep {
                block_slot_no,
                channel_transaction_id,
                channel_output_index,
                datum,
                lovelace,
                output_index,
                redeemer,
                transaction_id,
                transaction_index,
            } => self.conn.execute(
                "INSERT INTO step (\
                        block_slot_no,\
                        channel_transaction_id,\
                        channel_output_index,\
                        datum,\
                        lovelace,\
                        output_index,\
                        redeemer,\
                        transaction_id,\
                        transaction_index\
                     ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    block_slot_no,
                    channel_transaction_id,
                    channel_output_index,
                    datum,
                    lovelace,
                    output_index,
                    redeemer,
                    transaction_id,
                    transaction_index
                ],
            ),
        }?;
        Ok(())
    }

    pub fn block(&self, slot_no: u64) -> Result<Option<BlockRecord>> {
        let mut stmt = self
            .conn
            .prepare("SELECT header_hash, block_no, slot_no FROM block WHERE slot_no = ?")?;
        fn row_to_block_record(row: &Row<'_>) -> rusqlite::Result<BlockRecord> {
            Ok(BlockRecord {
                header_hash: row.get(0)?,
                block_no: row.get(1)?,
                slot_no: row.get(2)?,
            })
        }
        let record = stmt.query_row([slot_no], row_to_block_record).optional()?;
        Ok(record)
    }

    pub fn channel(&self, id: &ChannelId) -> Result<Option<ChannelRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT \
                add_vkey,\
                block_slot_no,\
                datum,\
                lovelace,\
                output_index,\
                script_hash,\
                sub_vkey,\
                tag,\
                transaction_id \
            FROM channel \
            WHERE transaction_id = ? AND output_index = ?",
        )?;

        fn row_to_channel_record(row: &Row<'_>) -> rusqlite::Result<ChannelRecord> {
            Ok(ChannelRecord {
                add_vkey: row.get(0)?,
                block_slot_no: row.get(1)?,
                datum: row.get(2)?,
                lovelace: row.get(3)?,
                output_index: row.get(4)?,
                script_hash: row.get(5)?,
                sub_vkey: row.get(6)?,
                tag: row.get(7)?,
                transaction_id: row.get(8)?,
            })
        }

        let record = stmt
            .query_row(
                rusqlite::params![id.transaction_id, id.output_index],
                row_to_channel_record,
            )
            .optional()?;
        Ok(record)
    }

    /// Look up a channel and all of its steps, joined under one
    /// [`ThreadRecord`]. Returns `None` if no channel exists for the id.
    // pub fn thread(&self, id: &ChannelId) -> Result<Option<ThreadRecord>> {
    //     let Some(channel) = self.channel(id)? else {
    //         return Ok(None);
    //     };

    //     let mut stmt = self.conn.prepare(
    //         "SELECT \
    //             block_slot_no, channel_slot_no, channel_tx_index, \
    //             transaction_id, datum, lovelace, output_index, redeemer \
    //          FROM step\
    //          WHERE channel_slot_no = ? AND channel_tx_index = ? \
    //          ORDER BY rowid",
    //     )?;
    //     let steps: Vec<StepRecord> = stmt
    //         .query_map(
    //             rusqlite::params![channel.block_slot_no, channel.transaction_index],
    //             row_to_step_record,
    //         )?
    //         .collect::<rusqlite::Result<_>>()?;
    // fn row_to_step_record(row: &Row<'_>) -> rusqlite::Result<StepRecord> {
    //     Ok(StepRecord::TransitionStep {
    //         block_slot_no: row.get(0)?,
    //         channel_slot_no: row.get(1)?,
    //         channel_tx_index: row.get(2)?,
    //         datum: row.get(4)?,
    //         lovelace: row.get(5)?,
    //         output_index: row.get(6)?,
    //         redeemer: row.get(7)?,
    //         transaction_id: row.get(3)?,
    //     })
    // }
    //     Ok(Some(ThreadRecord { channel, steps }))
    // }

    /// Return every `ChannelId` currently stored. Order is whatever SQLite
    /// chooses; callers that need stability should sort the result.
    pub fn all_channel_ids(&self) -> Result<Vec<ChannelId>> {
        let mut stmt = self
            .conn
            .prepare("SELECT transaction_id, output_index FROM channel")?;
        let ids = stmt
            .query_map([], row_to_channel_id)?
            .collect::<rusqlite::Result<_>>()?;
        Ok(ids)
    }

    pub fn channel_ids_by_keytag(&self, keytag: &[u8]) -> Result<Vec<ChannelId>> {
        if keytag.len() < 32 {
            return Ok(Vec::new());
        }
        let (add_vkey, tag) = keytag.split_at(32);

        let mut stmt = self.conn.prepare(
            "SELECT transaction_id, output_index FROM channel \
             WHERE add_vkey = ? AND tag = ?",
        )?;
        let ids = stmt
            .query_map(rusqlite::params![add_vkey, tag], row_to_channel_id)?
            .collect::<rusqlite::Result<_>>()?;
        Ok(ids)
    }
}

// ------------------------------------------------------------ Row decoders

fn row_to_channel_id(row: &Row<'_>) -> rusqlite::Result<ChannelId> {
    Ok(ChannelId {
        transaction_id: row.get(0)?,
        output_index: row.get(1)?,
    })
}

// ------------------------------------------------------------------- Tests

#[cfg(test)]
mod queries_test_suite {
    use super::*;
    use crate::api::Store;
    use crate::store::sqlite::SqliteStore;

    fn store() -> SqliteStore {
        SqliteStore::open_in_memory().expect("open in-memory store")
    }

    #[derive(Clone, Copy)]
    struct SlotNo(u64);

    fn mk_new_block(SlotNo(slot): SlotNo) -> NewBlock {
        // Header hash is unique per slot so tests inserting multiple blocks
        // don't trip the UNIQUE(header_hash) constraint.
        let mut header_hash = [0u8; 32];
        header_hash[0] = 0xAB;
        header_hash[1..9].copy_from_slice(&slot.to_be_bytes());
        NewBlock {
            header_hash,
            block_no: slot,
            slot_no: slot,
        }
    }
    #[derive(Clone, Copy)]
    struct TransactionIndex(u64);

    #[derive(Clone, Copy)]
    struct OutputIndex(u64);

    fn mk_new_channel(
        SlotNo(block_slot): SlotNo,
        TransactionIndex(transaction_index): TransactionIndex,
        OutputIndex(output_index): OutputIndex,
    ) -> (ChannelId, NewChannel) {
        let transaction_id = {
            let mut id = [0u8; 32];
            id[0] = block_slot as u8;
            id[1] = transaction_index as u8;
            id
        };
        let id = ChannelId {
            transaction_id,
            output_index,
        };
        let new = NewChannel {
            add_vkey: [0x11; 32],
            block_slot_no: block_slot,
            datum: vec![0xD0, 0xD1],
            lovelace: 1_500_000,
            output_index,
            script_hash: [0x33; 28],
            sub_vkey: [0x22; 32],
            tag: b"deadbeef".to_vec(),
            transaction_id,
            transaction_index,
        };
        (id, new)
    }

    fn new_transition_step(
        SlotNo(block_slot_no): SlotNo,
        channel_id: &ChannelId,
        TransactionIndex(transaction_index): TransactionIndex,
    ) -> NewStepRecord {
        NewStepRecord::NewTransitionStep {
            block_slot_no,
            channel_output_index: channel_id.output_index,
            channel_transaction_id: channel_id.transaction_id,
            datum: b"normal-datum".to_vec(),
            lovelace: 1_000_000,
            output_index: 0,
            redeemer: b"normal-redeemer".to_vec(),
            transaction_id: [0xCA; 32],
            transaction_index,
        }
    }

    fn new_close_step(
        SlotNo(block_slot_no): SlotNo,
        channel_id: &ChannelId,
        TransactionIndex(transaction_index): TransactionIndex,
    ) -> NewStepRecord {
        NewStepRecord::NewCloseStep {
            block_slot_no,
            channel_output_index: channel_id.output_index,
            channel_transaction_id: channel_id.transaction_id,
            redeemer: b"close-redeemer".to_vec(),
            transaction_id: [0xCC; 32],
            transaction_index,
        }
    }

    #[test]
    fn insert_and_lookup_block() {
        let store = store();
        let q = Queries::new(store.connection());

        let inserted = mk_new_block(SlotNo(42));
        q.insert_block(&inserted).unwrap();

        let got = q.block(42).unwrap().expect("block exists");
        assert_eq!(got.slot_no, 42);
        assert_eq!(got.block_no, 42);
        assert_eq!(got.header_hash, inserted.header_hash);
    }

    #[test]
    fn block_lookup_misses_when_absent() {
        let store = store();
        let q = Queries::new(store.connection());
        assert!(q.block(999).unwrap().is_none());
    }

    #[test]
    fn insert_and_lookup_channel_by_id() {
        let store = store();
        let q = Queries::new(store.connection());
        let slot_1 = SlotNo(1);
        q.insert_block(&mk_new_block(slot_1)).unwrap();
        let (id, ch) = mk_new_channel(slot_1, TransactionIndex(0), OutputIndex(0));
        q.insert_channel(&ch).unwrap();

        let got = q.channel(&id).unwrap().expect("channel exists");
        assert_eq!(got.transaction_id, id.transaction_id);
        assert_eq!(got.output_index, id.output_index);
        assert_eq!(got.lovelace, 1_500_000);
        assert_eq!(got.tag, b"deadbeef");
    }

    #[test]
    fn channel_lookup_misses_when_absent() {
        let store = store();
        let q = Queries::new(store.connection());
        let id = ChannelId {
            transaction_id: [0u8; 32],
            output_index: 0,
        };
        assert!(q.channel(&id).unwrap().is_none());
    }

    #[test]
    fn channel_requires_existing_block() {
        let store = store();
        let q = Queries::new(store.connection());
        let (_, ch) = mk_new_channel(SlotNo(999), TransactionIndex(0), OutputIndex(0));
        // No block inserted for slot 999 -> FK violation.
        assert!(q.insert_channel(&ch).is_err());
    }

    // #[test]
    // fn thread_returns_channel_with_steps_in_order() {
    //     let store = store();
    //     let q = Queries::new(store.connection());
    //     q.insert_block(&mk_new_block(10)).unwrap();
    //     q.insert_block(&mk_new_block(11)).unwrap();

    //     let (id, ch) = mk_new_channel(10, 0, 0);
    //     q.insert_channel(&ch).unwrap();

    //     q.insert_step(&new_transition_step(10, &id, 0)).unwrap();
    //     q.insert_step(&new_transition_step(11, &id, 0)).unwrap();
    //     q.insert_step(&new_close_step(11, &id, 1)).unwrap();

    //     let thread = q.thread(&id).unwrap().expect("thread exists");
    //     assert_eq!(thread.channel.block_slot_no, 10);
    //     assert_eq!(thread.channel.transaction_index, 0);
    //     assert_eq!(thread.steps.len(), 3);

    //     // Ordered by rowid.
    //     assert!(thread.steps[0].datum.is_some());
    //     assert!(thread.steps[1].datum.is_some());
    //     assert!(thread.steps[2].datum.is_none(), "close step has NULL datum");
    //     assert!(thread.steps[2].lovelace.is_none());
    //     assert!(thread.steps[2].output_index.is_none());
    // }

    #[test]
    fn step_before_channel_is_rejected_by_the_trigger() {
        let store = store();
        let q = Queries::new(store.connection());
        let slot_20 = SlotNo(20);
        q.insert_block(&mk_new_block(slot_20)).unwrap();

        let channel_tx_index = TransactionIndex(1);
        let invalid_step_tx_index = TransactionIndex(0);

        let (id, ch) = mk_new_channel(slot_20, channel_tx_index, OutputIndex(0));
        q.insert_channel(&ch).unwrap();
        let err = q
            .insert_step(&new_close_step(slot_20, &id, invalid_step_tx_index))
            .unwrap_err();

        assert!(
            format!("{err}")
                .contains("Any step before an initial `channel` output is not possible"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn step_after_close_is_rejected_by_trigger() {
        let store = store();
        let q = Queries::new(store.connection());
        let slot_20 = SlotNo(20);
        q.insert_block(&mk_new_block(slot_20)).unwrap();

        let (id, ch) = mk_new_channel(slot_20, TransactionIndex(0), OutputIndex(0));
        q.insert_channel(&ch).unwrap();

        let close_transaction_index = TransactionIndex(1);
        q.insert_step(&new_close_step(slot_20, &id, close_transaction_index))
            .unwrap();

        let step_transaction_index = TransactionIndex(2);
        // Any further step (normal or close) on the same channel must fail.
        let err = q
            .insert_step(&new_transition_step(slot_20, &id, step_transaction_index))
            .unwrap_err();
        assert!(
            format!("{err}").contains("Any step after a `close` is not possible"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn close_before_step_is_rejected_by_trigger() {
        let store = store();
        let q = Queries::new(store.connection());
        let slot_20 = SlotNo(20);
        q.insert_block(&mk_new_block(slot_20)).unwrap();

        let (id, ch) = mk_new_channel(slot_20, TransactionIndex(0), OutputIndex(0));
        q.insert_channel(&ch).unwrap();

        let step_transaction_index = TransactionIndex(2);
        // Any further step (normal or close) on the same channel must fail.
        q.insert_step(&new_transition_step(slot_20, &id, step_transaction_index))
            .unwrap();

        let close_transaction_index = TransactionIndex(1);
        let err = q
            .insert_step(&new_close_step(slot_20, &id, close_transaction_index))
            .unwrap_err();

        assert!(
            format!("{err}").contains("Any step after a `close` is not possible"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn step_before_close_is_possible() {
        let store = store();
        let q = Queries::new(store.connection());
        let slot_20 = SlotNo(20);
        q.insert_block(&mk_new_block(slot_20)).unwrap();

        let (id, ch) = mk_new_channel(slot_20, TransactionIndex(0), OutputIndex(0));
        q.insert_channel(&ch).unwrap();

        let close_transaction_index = TransactionIndex(2);
        q.insert_step(&new_close_step(slot_20, &id, close_transaction_index))
            .unwrap();

        let step_transaction_index = TransactionIndex(1);
        // Any further step (normal or close) on the same channel must fail.
        q.insert_step(&new_transition_step(slot_20, &id, step_transaction_index))
            .unwrap();
    }

    #[test]
    fn all_channel_ids_and_by_keytag() {
        let store = store();
        let q = Queries::new(store.connection());
        let slot_1 = SlotNo(1);
        q.insert_block(&mk_new_block(slot_1)).unwrap();

        // Two channels sharing the same keytag (same add_vkey, same tag).
        let (id_a, ch_a) = mk_new_channel(slot_1, TransactionIndex(0), OutputIndex(0));
        let (id_b, ch_b) = mk_new_channel(slot_1, TransactionIndex(1), OutputIndex(1));
        q.insert_channel(&ch_a).unwrap();
        q.insert_channel(&ch_b).unwrap();

        // A third channel with a different tag.
        let mut ch_c = ch_a.clone();
        ch_c.output_index = 0;
        ch_c.transaction_id = {
            let mut id = [0u8; 32];
            id[0] = 0xDE;
            id[1] = 0xFF;
            id
        };
        ch_c.transaction_index = 2;
        let id_c = ChannelId {
            transaction_id: ch_c.transaction_id,
            output_index: ch_c.output_index,
        };
        ch_c.tag = b"different".to_vec();
        q.insert_channel(&ch_c).unwrap();

        let all = q.all_channel_ids().unwrap();
        assert_eq!(all.len(), 3);

        let same_keytag: Vec<_> = q
            .channel_ids_by_keytag(
                {
                    let mut k = Vec::new();
                    k.extend_from_slice(&[0x11; 32]);
                    k.extend_from_slice(b"deadbeef");
                    k
                }
                .as_slice(),
            )
            .unwrap()
            .into_iter()
            .map(|c| (c.transaction_id, c.output_index))
            .collect();
        assert_eq!(same_keytag.len(), 2);
        assert!(same_keytag.contains(&(id_a.transaction_id, id_a.output_index)));
        assert!(same_keytag.contains(&(id_b.transaction_id, id_b.output_index)));
        assert!(!same_keytag.contains(&(id_c.transaction_id, id_c.output_index)));
    }
}

//! Implementation of the [`Store`] trait for sqlite.

use std::collections::HashMap;

use crate::{
    store::{
        StoreQueries, Thread, ThreadOutput, TransactionId,
        sqlite::codecs::{
            SqlDatumRef, SqlTag, SqlTagRef, SqlVerificationKey, SqlVerificationKeyRef,
        },
    },
    transaction::{Block, BlockHeaderHash, BlockNo, OutputIndex, SlotNo, TxOutRef},
};

use super::{
    ChannelId, ChannelRow, NewChannel, NewFocus, NewStep, Result, Store, StoreError, Threads,
};
use cardano_sdk::VerificationKey;
use konduit_data::{Redeemer, Tag};
use rusqlite::{OptionalExtension, Row};

mod codecs;
mod migrations;
use codecs::{SqlDatum, SqlRedeemer, SqlRedeemerRef};

impl From<rusqlite::Error> for StoreError {
    fn from(err: rusqlite::Error) -> Self {
        match err {
            rusqlite::Error::FromSqlConversionFailure(_, _, e) => {
                StoreError::StoreCorruped(format!("Failed to convert from SQL: {}", e))
            }
            _ => StoreError::StoreError(format!("SQLite error: {}", err)),
        }
    }
}

pub struct SqliteStore {
    conn: rusqlite::Connection,
}

impl SqliteStore {
    pub fn new(mut conn: rusqlite::Connection) -> Result<Self> {
        let version = migrations::database_version(&conn)?;
        migrations::run_migrations(&mut conn, version)?;
        Ok(SqliteStore { conn })
    }
}

pub struct SqliteQueries<'a> {
    conn: rusqlite::Transaction<'a>,
}

impl Store for SqliteStore {
    fn with_queries<F, T, E: From<StoreError>>(&mut self, f: F) -> std::result::Result<T, E>
    where
        F: FnOnce(&mut dyn StoreQueries) -> std::result::Result<T, E>,
    {
        let tx = self.conn.transaction().map_err(StoreError::from)?;
        let mut queries = SqliteQueries { conn: tx };
        let result = f(&mut queries);
        match result {
            Ok(val) => {
                queries.conn.commit().map_err(StoreError::from)?;
                Ok(val)
            }
            Err(e) => {
                // transaction is rolled back on drop
                Err(e)
            }
        }
    }
}

impl<'a> SqliteQueries<'a> {
    fn row_to_block_row(row: &Row<'_>) -> std::result::Result<Block, rusqlite::Error> {
        let header_hash: BlockHeaderHash = row.get(0)?;
        let block_no: BlockNo = row.get(1)?;
        let slot_no: SlotNo = row.get(2)?;
        Ok(Block {
            header_hash,
            block_no,
            slot_no,
        })
    }
}

struct TempTableGuard<'a> {
    conn: &'a rusqlite::Transaction<'a>,
    name: &'static str,
}

impl Drop for TempTableGuard<'_> {
    fn drop(&mut self) {
        // Ignore any error – Drop must not panic
        let _ = self
            .conn
            .execute(&format!("DROP TABLE IF EXISTS {}", self.name), []);
    }
}

impl<'a> StoreQueries for SqliteQueries<'a> {
    fn get_block(&self, slot_no: SlotNo) -> Result<Option<Block>> {
        let mut stmt = self
            .conn
            .prepare("SELECT header_hash, block_no, slot_no FROM block WHERE slot_no = ?")?;
        stmt.query_row(rusqlite::params![slot_no], |row| {
            Self::row_to_block_row(row)
        })
        .optional()
        .map_err(|e| e.into())
    }

    fn get_channel(&self, id: &ChannelId) -> Result<Option<ChannelRow>> {
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

        fn row_to_channel_record(
            row: &Row<'_>,
        ) -> std::result::Result<ChannelRow, rusqlite::Error> {
            Ok(ChannelRow {
                add_vkey: row.get(0).map(|SqlVerificationKey(vk)| vk)?,
                block_slot_no: row.get(1)?,
                datum: row.get(2).map(|SqlDatum(d)| d)?,
                lovelace: row.get(3)?,
                output_index: row.get(4)?,
                script_hash: row.get(5)?,
                sub_vkey: row.get(6).map(|SqlVerificationKey(vk)| vk)?,
                tag: row.get(7).map(|SqlTag(tag)| tag)?,
                transaction_id: row.get(8)?,
            })
        }
        stmt.query_row(
            rusqlite::params![id.transaction_id, id.output_index],
            row_to_channel_record,
        )
        .optional()
        .map_err(|e| e.into())
    }

    fn get_channel_ids_by_keytag(
        &self,
        add_vkey: &VerificationKey,
        tag: &Tag,
    ) -> Result<Vec<ChannelId>> {
        let mut stmt = self.conn.prepare(
            "SELECT transaction_id, output_index FROM channel \
             WHERE add_vkey = ? AND tag = ?",
        )?;
        fn row_to_channel_id(row: &Row<'_>) -> rusqlite::Result<ChannelId> {
            Ok(ChannelId {
                transaction_id: row.get(0)?,
                output_index: row.get(1)?,
            })
        }
        let ids = stmt
            .query_map(
                rusqlite::params![SqlVerificationKeyRef(add_vkey), SqlTagRef(tag)],
                row_to_channel_id,
            )?
            .collect::<rusqlite::Result<_>>()?;
        Ok(ids)
    }

    fn get_channel_ids_by_outputs(
        &self,
        output_refs: &[TxOutRef],
    ) -> Result<HashMap<TxOutRef, ChannelId>> {
        if output_refs.is_empty() {
            return Ok(HashMap::new());
        }
        let temp_table_name = "tmp_out_refs";
        self.conn.execute_batch(
            "\
            CREATE TEMP TABLE IF NOT EXISTS tmp_out_refs ( \
                transaction_id  BLOB NOT NULL, \
                output_index    INTEGER NOT NULL \
            ); \
            CREATE INDEX IF NOT EXISTS idx_tmp_out_refs \
            ON tmp_out_refs (transaction_id, output_index); \
            ",
        )?;
        let _temp_table_guard = TempTableGuard {
            conn: &self.conn,
            name: temp_table_name,
        };
        for output_ref in output_refs {
            self.conn.execute(
                &format!(
                    "INSERT INTO {} (transaction_id, output_index) VALUES (?1, ?2)",
                    temp_table_name
                ),
                rusqlite::params![output_ref.0, output_ref.1],
            )?;
        }
        let mut stmt = self.conn.prepare(
            "\
            SELECT DISTINCT \
               s.transaction_id, s.output_index, \
               c.transaction_id, c.output_index \
            FROM channel c \
            LEFT JOIN step s \
               ON s.channel_transaction_id = c.transaction_id \
               AND s.channel_output_index = c.output_index \
            INNER JOIN tmp_out_refs t \
                   ON (c.transaction_id = t.transaction_id AND c.output_index = t.output_index) \
                   OR (s.transaction_id = t.transaction_id AND s.output_index = t.output_index) \
            ",
        )?;
        fn row_to_the_final_pair(row: &Row<'_>) -> rusqlite::Result<(TxOutRef, ChannelId)> {
            let step_tx_id: Option<TransactionId> = row.get(0)?;
            let step_output_index: Option<OutputIndex> = row.get(1)?;
            let channel_tx_id: TransactionId = row.get(2)?;
            let channel_output_index: OutputIndex = row.get(3)?;
            let tx_out_ref =
                if let (Some(tx_id), Some(output_index)) = (step_tx_id, step_output_index) {
                    TxOutRef(tx_id, output_index)
                } else {
                    TxOutRef(channel_tx_id, channel_output_index)
                };
            let channel_id = ChannelId {
                transaction_id: channel_tx_id,
                output_index: channel_output_index,
            };
            Ok((tx_out_ref, channel_id))
        }
        let res = stmt
            .query_map([], row_to_the_final_pair)?
            .collect::<rusqlite::Result<HashMap<_, _>>>()?;
        Ok(res)
    }

    fn get_threads_by_keytag(&self, add_vkey: &VerificationKey, tag: &Tag) -> Result<Threads> {
        // One row per (channel, step). Channels with no steps still appear.
        // `is_focused` are `true` for at most one unique channel.
        // The ordering groups rows by channel id and then by
        // the reverse order of steps so we can build the threads
        // linked lists in a one pass.
        let mut stmt = self.conn.prepare(
            "SELECT \
                c.datum \
                c.output_index \
                c.transaction_id \
                cb.block_no \
                cb.slot_no \
                f.channel_transaction_id IS NOT NULL \
                s.datum \
                s.output_index \
                s.redeemer \
                s.transaction_id \
                s.transaction_index \
                sb.block_no \
                sb.slot_no \
             FROM channel c \
             INNER JOIN block cb ON cb.slot_no = c.block_slot_no \
             LEFT JOIN step s \
                ON s.channel_transaction_id = c.transaction_id \
               AND s.channel_output_index   = c.output_index \
             LEFT JOIN block sb ON sb.slot_no = s.block_slot_no \
             LEFT JOIN focus f \
                ON f.channel_transaction_id    = c.transaction_id \
               AND f.channel_transaction_index = c.transaction_index \
             WHERE c.add_vkey = ? AND c.tag = ? \
             ORDER BY \
                c.block_no, c.transaction_index, c.output_index,
                sb.block_no, s.transaction_index, s.output_index,
             DESC",
        )?;
        let mut threads = Threads {
            focus: None,
            others: Vec::new(),
        };
        let mut current_channel_id: Option<ChannelId> = None;
        let mut current_thread_initial_output: Option<ThreadOutput> = None;
        let mut current_thread_steps: Option<(Redeemer, Option<Box<ThreadOutput>>)> = None;
        let mut current_thread_is_focused = false;
        fn finalize_current_thread(
            threads: &mut Threads,
            initial_output: &mut Option<ThreadOutput>,
            steps: &mut Option<(Redeemer, Option<Box<ThreadOutput>>)>,
            is_focused: bool,
        ) {
            if let Some(mut head) = initial_output.take() {
                head.step = steps.take();
                let thread = Thread(head);
                if is_focused {
                    threads.focus = Some(thread);
                } else {
                    threads.others.push(thread);
                }
            }
        }
        let _ = stmt.query_and_then(
            rusqlite::params![SqlVerificationKeyRef(add_vkey), SqlTagRef(tag)],
            |row| {
                // Let's unpack all the fields keeping the names and order from the SQL query.
                let c_datum = row.get(0).map(|SqlDatum(d)| d)?;
                let c_output_index = row.get(1)?;
                let c_tx_id = row.get(2)?;
                let cb_block_no = row.get(3)?;
                let cb_slot_no = row.get(4)?;
                let f_is_focused = row.get(5)?;
                let s_datum = row.get::<_, Option<_>>(6)?.map(|SqlDatum(d)| d);
                let s_output_index = row.get::<_, Option<OutputIndex>>(7)?;
                let s_redeemer = row.get::<_, Option<_>>(8)?.map(|SqlRedeemer(r)| r);
                let s_tx_id = row.get::<_, Option<TransactionId>>(9)?;
                let sb_block_no = row.get::<_, Option<_>>(10)?;
                let sb_slot_no = row.get::<_, Option<SlotNo>>(11)?;
                let channel_id = ChannelId {
                    transaction_id: c_tx_id,
                    output_index: c_output_index,
                };
                if current_channel_id != Some(channel_id) {
                    finalize_current_thread(
                        &mut threads,
                        &mut current_thread_initial_output,
                        &mut current_thread_steps,
                        current_thread_is_focused,
                    );
                    current_thread_steps = None;
                    current_channel_id = Some(channel_id);
                    current_thread_is_focused = f_is_focused;
                    current_thread_initial_output = Some(ThreadOutput {
                        block_no: cb_block_no,
                        block_slot_no: cb_slot_no,
                        transaction_id: c_tx_id,
                        output_index: c_output_index,
                        datum: c_datum,
                        step: None,
                    });
                }
                current_thread_steps = s_redeemer.map(|redeemer| {
                    (
                        redeemer,
                        (|| {
                            let block_no = sb_block_no?;
                            let block_slot_no = sb_slot_no?;
                            let transaction_id = s_tx_id?;
                            let output_index = s_output_index?;
                            let datum = s_datum?;

                            Some(Box::new(ThreadOutput {
                                block_no,
                                block_slot_no,
                                transaction_id,
                                output_index,
                                datum,
                                step: current_thread_steps.take(),
                            }))
                        })(),
                    )
                });
                Ok::<(), rusqlite::Error>(())
            },
        )?;
        finalize_current_thread(
            &mut threads,
            &mut current_thread_initial_output,
            &mut current_thread_steps,
            current_thread_is_focused,
        );
        Ok(threads)
    }

    fn get_tip(&self) -> Result<Option<Block>> {
        let mut stmt = self.conn.prepare(
            "SELECT header_hash, block_no, slot_no FROM block \
             ORDER BY block_no DESC LIMIT 1",
        )?;
        stmt.query_row([], Self::row_to_block_row)
            .optional()
            .map_err(|e| e.into())
    }

    fn insert_block(&self, block: &Block) -> Result<()> {
        let mut stmt = self
            .conn
            .prepare("INSERT INTO block (block_no, header_hash, slot_no) VALUES (?, ?, ?)")?;
        stmt.execute(rusqlite::params![
            block.block_no,
            block.header_hash,
            block.slot_no
        ])?;
        Ok(())
    }

    fn insert_channel(&self, channel: NewChannel) -> Result<()> {
        let mut stmt = self.conn.prepare(
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
                transaction_index \
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )?;

        stmt.execute(rusqlite::params![
            SqlVerificationKeyRef(channel.add_vkey),
            channel.block_slot_no,
            SqlDatumRef(channel.datum),
            channel.lovelace,
            channel.output_index,
            channel.script_hash,
            SqlVerificationKeyRef(channel.sub_vkey),
            SqlTagRef(channel.tag),
            channel.transaction_id,
            channel.transaction_index,
        ])?;
        Ok(())
    }

    fn insert_step(&self, step: NewStep) -> Result<()> {
        match step {
            NewStep::NewCloseStep {
                block_slot_no,
                channel_output_index,
                channel_transaction_id,
                redeemer,
                transaction_id,
                transaction_index,
            } => self.conn.execute(
                "INSERT INTO step (\
                        block_slot_no, \
                        channel_output_index,\
                        channel_transaction_id,\
                        redeemer,\
                        transaction_id,\
                        transaction_index
                     ) VALUES (?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    block_slot_no,
                    channel_output_index,
                    channel_transaction_id,
                    SqlRedeemerRef(redeemer),
                    transaction_id,
                    transaction_index
                ],
            ),
            NewStep::NewTransitionStep {
                block_slot_no,
                channel_output_index,
                channel_transaction_id,
                datum,
                lovelace,
                output_index,
                redeemer,
                transaction_id,
                transaction_index,
            } => self.conn.execute(
                "INSERT INTO step (\
                        block_slot_no,\
                        channel_output_index,\
                        channel_transaction_id,\
                        datum,\
                        lovelace,\
                        output_index,\
                        redeemer,\
                        transaction_id,\
                        transaction_index\
                     ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                rusqlite::params![
                    block_slot_no,
                    channel_output_index,
                    channel_transaction_id,
                    SqlDatumRef(datum),
                    lovelace,
                    output_index,
                    SqlRedeemerRef(redeemer),
                    transaction_id,
                    transaction_index
                ],
            ),
        }?;
        Ok(())
    }

    fn rollback_block(&self, slot_no: SlotNo) -> Result<()> {
        self.conn.execute(
            "DELETE FROM block WHERE slot_no = ?",
            rusqlite::params![slot_no],
        )?;
        Ok(())
    }

    /// Set the focused channel for a keytag.
    ///
    /// The caller provides the `add_vkey` and `tag` directly (they
    /// already know which channel they're focusing) so we don't need a
    /// extra `SELECT` against `channel`. The `focus_check_consistency`
    /// trigger validates that the provided values match the channel row,
    /// so an inconsistency surfaces as an `SqliteFailure` here.
    ///
    /// If a focus already exists for the same `(add_vkey, tag)` it is
    /// *replaced* — the schema's `UNIQUE (add_vkey, tag)` only allows one
    /// focused channel per keytag, so we have to remove the previous row
    /// first.
    fn set_focus(&self, focus: NewFocus) -> Result<()> {
        self.conn.execute(
            "DELETE FROM focus WHERE add_vkey = ? AND tag = ?",
            rusqlite::params![SqlVerificationKeyRef(focus.add_vkey), SqlTagRef(focus.tag)],
        )?;
        self.conn.execute(
            "INSERT INTO focus (add_vkey, tag, channel_transaction_id, channel_transaction_index) \
             VALUES (?, ?, ?, ?)",
            rusqlite::params![
                SqlVerificationKeyRef(focus.add_vkey),
                SqlTagRef(focus.tag),
                focus.channel_transaction_id,
                focus.channel_transaction_index
            ],
        )?;
        Ok(())
    }
}

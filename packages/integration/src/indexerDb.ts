import * as fs from "node:fs";
import type { SQLInputValue } from "node:sqlite";
import { DatabaseSync } from "node:sqlite";
import { err, ok, type Result } from "neverthrow";
import { unwrapOrPanicWith } from "@konduit/konduit-consumer/neverthrow";
import * as hexString from "@konduit/codec/hexString";
import * as uint8Array from "@konduit/codec/uint8Array";
import { Datum } from "./konduit/datum.js";
import { Redeemer } from "./konduit/redeemer.js";

/**
 * @file Read-only access to the `konduit-indexer` SQLite database.
 *
 * This is the SQLite counterpart of what `marlowe-integration` does against
 * Postgres in `marlowe-integration/app/src/marloweIndexer/db.ts`: a thin,
 * typed layer over "the indexer's own view of the chain", plus domain types
 * that mirror the tables in `packages/konduit/indexer/sqlite/v1.0.0/001.sql`.
 *
 * Two things are deliberate:
 *
 *  1. **Read-only, always.** The db belongs to the long-running
 *     `konduit-indexer` process-compose process. Tests open it with
 *     `readOnly: true` so a stray write cannot corrupt the state the rest of
 *     the suite (and the running indexer) depends on.
 *  2. **Reads are retried, not fatal.** The indexer syncs on a timer
 *     (`schedule = { interval = "10s" }` in `flake/process-compose/dev-env.nix`)
 *     and the store uses SQLite's default rollback journal, so a writer can
 *     hold an exclusive lock for a moment. `SQLITE_BUSY` here is a transient
 *     condition, not a failure, so {@link IndexerDb.withRetry} swallows it and
 *     the caller's polling loop simply tries again.
 *
 * Everything is option-driven — the db location comes from
 * {@link IndexerDbConfig}, which `testEnv.loadTestEnv` builds from the
 * environment. This module never reads `process.env`.
 */

export type IndexerDbConfig = {
  /** Absolute path to the SQLite file the running `konduit-indexer` writes to. */
  dbPath: string;
  /**
   * Directory holding the versioned migration folders
   * (`packages/konduit/indexer/sqlite`). Used to cross-check that the schema
   * baked into the indexer binary matches the one the tests expect.
   */
  schemaDir?: string;
  /** SQLite busy timeout in milliseconds — how long a read waits for a writer. */
  busyTimeoutMs: number;
  /** Echo every statement being run. */
  debug?: boolean;
};

// ---------------------------------------------------------------------------
// Row shapes — a 1:1 mirror of the tables in 001.sql.
// ---------------------------------------------------------------------------

export type BlockRow = {
  blockNo: number;
  /** 32-byte header hash, hex encoded. */
  headerHashHex: string;
  slotNo: number;
};

/** `channel` table. A channel's initial output is its thread's root. */
export type ChannelRow = {
  addVkeyHex: string;
  blockSlotNo: number;
  datumBlob: Uint8Array;
  /** Decoded `Datum`, or `null` when the blob could not be decoded. */
  datum: Datum | null;
  datumError: string | null;
  lovelace: number;
  outputIndex: number;
  scriptHashHex: string;
  subVkeyHex: string;
  tagHex: string;
  /** Transaction that created the channel output, hex encoded. */
  transactionIdHex: string;
  /** Index of that transaction inside its block. */
  transactionIndex: number;
  blockNo: number | null;
};

/**
 * `step` table. One row per channel transition.
 *
 * A *continuation* step (`add`, `sub`, `respond`, `unlock`, `expire`) has a
 * datum, lovelace and output index; a *closing* step has all three `NULL`
 * — that is the schema's `CHECK` constraint, and it is the cheapest way to tell
 * the two apart without decoding the redeemer.
 */
export type StepRow = {
  blockSlotNo: number;
  /** The channel this step belongs to (its initial output). */
  channelTransactionIdHex: string;
  channelOutputIndex: number;
  /** `null` only for a closing step. */
  datumBlob: Uint8Array | null;
  datum: Datum | null;
  datumError: string | null;
  /** `null` only for a closing step. */
  lovelace: number | null;
  /** `null` only for a closing step. */
  outputIndex: number | null;
  redeemerBlob: Uint8Array | null;
  redeemer: Redeemer | null;
  redeemerError: string | null;
  /** Convenience projection of the redeemer: e.g. `["Cont:Add"]`. */
  redeemerSteps: string[];
  transactionIdHex: string;
  transactionIndex: number;
  blockNo: number | null;
};

export type FocusRow = {
  addVkeyHex: string;
  tagHex: string;
  channelTransactionIdHex: string;
  channelTransactionIndex: number;
};

export type TableCounts = {
  block: number;
  channel: number;
  step: number;
  focus: number;
};

/** A channel's full thread: the initial output plus every step, in order. */
export type ChannelThread = {
  channel: ChannelRow;
  steps: StepRow[];
  /** `channel.datum` — the state at the root of the thread. */
  initialStage: Datum["stage"] | null;
  /**
   * The live state: the *last* step's datum, or the channel's own datum when
   * the channel has no steps yet, or `null` when the thread reached an end-of-
   * life step.
   *
   * Mirrors `konduit_indexer::store::ThreadOutput::curr_thread_state`.
   */
  currentStage: Datum["stage"] | null;
  /**
   * `true` when the thread ended in a step that produced *no* continuation —
   * i.e. a step row with `output_index IS NULL`.
   *
   * Careful with the wording: this is `ThreadOutput::is_thread_closed`, which is
   * **not** the same as "the konduit channel is closed". `consumer tx --close`
   * submits `Redeemer::Main([Step::Cont(Cont::Close)])`, which the indexer
   * records as a *transition* (it has a continuation output whose datum stage is
   * `Closed`). Only a mutual close (`Redeemer::Mutual`) or an end-of-life step
   * (`Eol::End` / `Eol::Elapse`) yields a row with a NULL output index.
   * See `insert_transactions` in `packages/konduit/indexer/src/store/mod.rs`.
   */
  terminated: boolean;
};

/** The `(add_vkey, tag)` pair that identifies a channel thread on-chain. */
export type Keytag = { addVkeyHex: string; tagHex: string };

// ---------------------------------------------------------------------------
// Handle
// ---------------------------------------------------------------------------

/** Anything `node:sqlite` can throw that means "come back later". */
const isBusyError = (error: unknown): boolean => {
  const message = error instanceof Error ? error.message : String(error);
  return (
    message.includes("SQLITE_BUSY") ||
    message.includes("database is locked") ||
    message.includes("database table is locked")
  );
};

const isMissingFileError = (error: unknown): boolean => {
  const message = error instanceof Error ? error.message : String(error);
  return (
    message.includes("no such file") || message.includes("SQLITE_CANTOPEN")
  );
};

export class IndexerDb {
  private handle: DatabaseSync | null = null;
  private closed = false;

  constructor(readonly config: IndexerDbConfig) {}

  /**
   * Open the db. Returns an error rather than throwing so callers can skip the
   * db checks when no indexer is running (the same way the e2e tests skip when
   * `process-compose` is down).
   */
  open(): Result<this, string> {
    if (this.handle) return ok(this);
    if (!fs.existsSync(this.config.dbPath)) {
      return err(
        `indexer db not found at ${this.config.dbPath} — ` +
          `is the konduit-indexer process-compose process running?`,
      );
    }
    try {
      this.handle = new DatabaseSync(this.config.dbPath, {
        readOnly: true,
        // Foreign keys are declared ON DELETE CASCADE in 001.sql but SQLite
        // defaults them *off*; the store never relies on enforcement for
        // correctness, and we are not writing, so leave the default alone.
        enableForeignKeyConstraints: false,
        timeout: this.config.busyTimeoutMs,
      });
      return ok(this);
    } catch (error) {
      if (isMissingFileError(error)) {
        return err(`cannot open ${this.config.dbPath}: ${describe(error)}`);
      }
      return err(`cannot open ${this.config.dbPath}: ${describe(error)}`);
    }
  }

  close(): void {
    if (this.closed) return;
    this.closed = true;
    try {
      this.handle?.close();
    } catch {
      // Closing is best effort; the handle is dropped either way.
    }
    this.handle = null;
  }

  /**
   * Run `fn` against the connection, converting a transient `SQLITE_BUSY` into
   * an error the polling loops can retry.
   */
  private withRetry<T>(
    label: string,
    fn: (db: DatabaseSync) => T,
  ): Result<T, string> {
    const db = this.handle;
    if (!db) return err(`indexer db is not open (while ${label})`);
    const trace = (what: string) => {
      try {
        require("node:fs").appendFileSync(
          new URL("../../../.run/e2e/dbg.log", import.meta.url).pathname,
          `${new Date().toISOString()} ${label} ${what}\n`,
        );
      } catch {
        /* ignore */
      }
    };
    trace("enter");
    try {
      const value = fn(db);
      trace("ok");
      if (this.config.debug) {
        console.debug(`[indexerDb] ${label} -> ok`);
      }
      return ok(value);
    } catch (error) {
      const message = describe(error);
      trace(`err ${message}`);
      return err(
        `${label}: ${message}${isBusyError(error) ? " (busy — retry)" : ""}`,
      );
    }
  }

  /** Every statement is logged when `debug` is on. */
  private all<T>(
    db: DatabaseSync,
    sql: string,
    params: SQLInputValue[] = [],
  ): T[] {
    if (this.config.debug)
      console.debug(`[indexerDb] ${sql} ${JSON.stringify(params)}`);
    return db.prepare(sql).all(...params) as T[];
  }

  private one<T>(
    db: DatabaseSync,
    sql: string,
    params: SQLInputValue[] = [],
  ): T | undefined {
    if (this.config.debug)
      console.debug(`[indexerDb] ${sql} ${JSON.stringify(params)}`);
    return db.prepare(sql).get(...params) as T | undefined;
  }

  // -------------------------------------------------------------------------
  // Schema / liveness
  // -------------------------------------------------------------------------

  /** `PRAGMA user_version` — the migration revision the store last applied. */
  schemaVersion(): Result<number, string> {
    return this.withRetry("schemaVersion", (db) => {
      const row = this.one<Record<string, number>>(db, "PRAGMA user_version");
      return row ? (Object.values(row)[0] ?? 0) : 0;
    });
  }

  /** Tables the tests need. A miss here means the migration has not run. */
  tables(): Result<string[], string> {
    return this.withRetry("tables", (db) =>
      this.all<{ name: string }>(
        db,
        "SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name",
      ).map((r) => r.name),
    );
  }

  /**
   * The indexed tip — the highest `block_no` the store knows about.
   *
   * A live indexer moves this forward every sync pass, so it doubles as a
   * liveness probe: if the tip is stuck while the chain advances, the indexer
   * is wedged and any channel/step assertions would time out for the wrong
   * reason.
   */
  tip(): Result<BlockRow | null, string> {
    return this.withRetry("tip", (db) => {
      const row = this.one<RawBlockRow>(
        db,
        "SELECT block_no, header_hash, slot_no FROM block ORDER BY block_no DESC LIMIT 1",
      );
      return row ? toBlockRow(row) : null;
    });
  }

  counts(): Result<TableCounts, string> {
    return this.withRetry("counts", (db) => ({
      block:
        this.one<{ c: number }>(db, "SELECT COUNT(*) c FROM block")?.c ?? 0,
      channel:
        this.one<{ c: number }>(db, "SELECT COUNT(*) c FROM channel")?.c ?? 0,
      step: this.one<{ c: number }>(db, "SELECT COUNT(*) c FROM step")?.c ?? 0,
      focus:
        this.one<{ c: number }>(db, "SELECT COUNT(*) c FROM focus")?.c ?? 0,
    }));
  }

  // -------------------------------------------------------------------------
  // Channels
  // -------------------------------------------------------------------------

  /**
   * Every channel row for `(add_vkey, tag)`.
   *
   * The schema's comment is explicit that this pair is *not* enforced unique on
   * chain (mimics are possible), so this returns a list and callers assert on
   * its length rather than assuming a single row.
   */
  channelsByKeytag(keytag: Keytag): Result<ChannelRow[], string> {
    return this.withRetry("channelsByKeytag", (db) => {
      const rows = this.all<RawChannelRow>(
        db,
        `SELECT c.add_vkey, c.block_slot_no, c.datum, c.lovelace, c.output_index,
                c.script_hash, c.sub_vkey, c.tag, c.transaction_id,
                c.transaction_index, b.block_no
           FROM channel c
           LEFT JOIN block b ON b.slot_no = c.block_slot_no
          WHERE c.add_vkey = ? AND c.tag = ?`,
        [bytesOrThrow(keytag.addVkeyHex), bytesOrThrow(keytag.tagHex)],
      );
      return rows.map(toChannelRow);
    });
  }

  channelById(channel: {
    transactionIdHex: string;
    outputIndex: number;
  }): Result<ChannelRow | null, string> {
    return this.withRetry("channelById", (db) => {
      const row = this.one<RawChannelRow>(
        db,
        `SELECT c.add_vkey, c.block_slot_no, c.datum, c.lovelace, c.output_index,
                c.script_hash, c.sub_vkey, c.tag, c.transaction_id,
                c.transaction_index, b.block_no
           FROM channel c
           LEFT JOIN block b ON b.slot_no = c.block_slot_no
          WHERE c.transaction_id = ? AND c.output_index = ?`,
        [bytesOrThrow(channel.transactionIdHex), channel.outputIndex],
      );
      return row ? toChannelRow(row) : null;
    });
  }

  /** All channels, newest block first. Used by the "did anything change" checks. */
  channels(limit = 50): Result<ChannelRow[], string> {
    return this.withRetry("channels", (db) => {
      const rows = this.all<RawChannelRow>(
        db,
        `SELECT c.add_vkey, c.block_slot_no, c.datum, c.lovelace, c.output_index,
                c.script_hash, c.sub_vkey, c.tag, c.transaction_id,
                c.transaction_index, b.block_no
           FROM channel c
           LEFT JOIN block b ON b.slot_no = c.block_slot_no
          ORDER BY c.block_slot_no DESC, c.transaction_index DESC, c.output_index DESC
          LIMIT ?`,
        [limit],
      );
      return rows.map(toChannelRow);
    });
  }

  // -------------------------------------------------------------------------
  // Steps
  // -------------------------------------------------------------------------

  /**
   * Every step belonging to a channel, in chain order.
   *
   * The `ORDER BY` mirrors the indexer that supports (`step_ordering` on
   * `(block_slot_no, transaction_index, output_index)`), which is also the
   * order `get_threads_by_keytag` walks when it rebuilds a thread.
   */
  stepsByChannel(channel: {
    transactionIdHex: string;
    outputIndex: number;
  }): Result<StepRow[], string> {
    return this.withRetry("stepsByChannel", (db) => {
      const rows = this.all<RawStepRow>(
        db,
        `SELECT s.block_slot_no, s.channel_transaction_id, s.channel_output_index,
                s.datum, s.lovelace, s.output_index, s.redeemer,
                s.transaction_id, s.transaction_index, b.block_no
           FROM step s
           LEFT JOIN block b ON b.slot_no = s.block_slot_no
          WHERE s.channel_transaction_id = ? AND s.channel_output_index = ?
          ORDER BY s.block_slot_no ASC, s.transaction_index ASC, s.output_index ASC`,
        [bytesOrThrow(channel.transactionIdHex), channel.outputIndex],
      );
      return rows.map(toStepRow);
    });
  }

  /** Every step in the store, newest first. */
  steps(limit = 100): Result<StepRow[], string> {
    return this.withRetry("steps", (db) => {
      const rows = this.all<RawStepRow>(
        db,
        `SELECT s.block_slot_no, s.channel_transaction_id, s.channel_output_index,
                s.datum, s.lovelace, s.output_index, s.redeemer,
                s.transaction_id, s.transaction_index, b.block_no
           FROM step s
           LEFT JOIN block b ON b.slot_no = s.block_slot_no
          ORDER BY s.block_slot_no DESC, s.transaction_index DESC, s.output_index DESC
          LIMIT ?`,
        [limit],
      );
      return rows.map(toStepRow);
    });
  }

  /** Is `transactionId` a step of this channel? The cheapest "did it land?" probe. */
  hasStepInTransaction(
    channel: { transactionIdHex: string; outputIndex: number },
    transactionIdHex: string,
  ): Result<boolean, string> {
    return this.withRetry("hasStepInTransaction", (db) => {
      const row = this.one<{ c: number }>(
        db,
        `SELECT COUNT(*) c FROM step
          WHERE channel_transaction_id = ? AND channel_output_index = ?
            AND transaction_id = ?`,
        [
          bytesOrThrow(channel.transactionIdHex),
          channel.outputIndex,
          bytesOrThrow(transactionIdHex),
        ],
      );
      return (row?.c ?? 0) > 0;
    });
  }

  focusRows(): Result<FocusRow[], string> {
    return this.withRetry("focusRows", (db) =>
      this.all<{
        add_vkey: Uint8Array;
        tag: Uint8Array;
        channel_transaction_id: Uint8Array;
        channel_transaction_index: number;
      }>(
        db,
        `SELECT add_vkey, tag, channel_transaction_id, channel_transaction_index
           FROM focus ORDER BY add_vkey, tag`,
      ).map((r) => ({
        addVkeyHex: hexString.fromUint8Array(r.add_vkey),
        tagHex: hexString.fromUint8Array(r.tag),
        channelTransactionIdHex: hexString.fromUint8Array(
          r.channel_transaction_id,
        ),
        channelTransactionIndex: r.channel_transaction_index,
      })),
    );
  }

  // -------------------------------------------------------------------------
  // Thread assembly
  // -------------------------------------------------------------------------

  /**
   * Load a channel and its whole thread in one round trip.
   *
   * Returns `null` (not an error) when the channel is not indexed *yet* — that
   * is the normal state while the indexer is still catching up, and the
   * polling helpers distinguish it from a genuine failure.
   */
  threadByKeytag(keytag: Keytag): Result<ChannelThread | null, string> {
    const channels = this.channelsByKeytag(keytag);
    if (channels.isErr()) return err(channels.error);
    const channel = channels.value[0];
    if (!channel) return ok(null);
    const steps = this.stepsByChannel({
      transactionIdHex: channel.transactionIdHex,
      outputIndex: channel.outputIndex,
    });
    if (steps.isErr()) return err(steps.error);
    return ok(assembleThread(channel, steps.value));
  }
}

// ---------------------------------------------------------------------------
// Thread assembly (mirrors `konduit_indexer::store::Thread`)
// ---------------------------------------------------------------------------

/**
 * A thread is *terminated* when its last step produced no continuation, i.e. a
 * step row with `output_index IS NULL`. Its current state is then `None`,
 * exactly like `ThreadOutput::curr_thread_state`.
 */
export const assembleThread = (
  channel: ChannelRow,
  steps: StepRow[],
): ChannelThread => {
  const initialStage = channel.datum?.stage ?? null;
  const last = steps[steps.length - 1] ?? null;
  const terminated = last !== null && last.outputIndex === null;
  const currentStage = terminated ? null : (last?.datum?.stage ?? initialStage);
  return { channel, steps, initialStage, currentStage, terminated };
};

/**
 * Is the konduit *channel* closed, as opposed to the thread being terminated?
 *
 * A `Cont::Close` step moves the stage to `Closed` while the UTxO still exists
 * (the close period has to elapse before it can be unlocked), so "closed" and
 * "terminated" are genuinely different questions.
 */
export const isChannelClosed = (thread: ChannelThread): boolean =>
  thread.terminated || thread.currentStage?.label === "Closed";

// ---------------------------------------------------------------------------
// Raw row -> domain row
// ---------------------------------------------------------------------------

type RawBlockRow = {
  block_no: number;
  header_hash: Uint8Array;
  slot_no: number;
};

type RawChannelRow = {
  add_vkey: Uint8Array;
  block_slot_no: number;
  datum: Uint8Array;
  lovelace: number;
  output_index: number;
  script_hash: Uint8Array;
  sub_vkey: Uint8Array;
  tag: Uint8Array;
  transaction_id: Uint8Array;
  transaction_index: number;
  block_no: number | null;
};

type RawStepRow = {
  block_slot_no: number;
  channel_transaction_id: Uint8Array;
  channel_output_index: number;
  datum: Uint8Array | null;
  lovelace: number | null;
  output_index: number | null;
  redeemer: Uint8Array | null;
  transaction_id: Uint8Array;
  transaction_index: number;
  block_no: number | null;
};

const toBlockRow = (raw: RawBlockRow): BlockRow => ({
  blockNo: raw.block_no,
  headerHashHex: hexString.fromUint8Array(raw.header_hash),
  slotNo: raw.slot_no,
});

const toChannelRow = (raw: RawChannelRow): ChannelRow => {
  const decoded = decodeSafely(Datum.uint8ArrayCodec.deserialise(raw.datum));
  return {
    addVkeyHex: hexString.fromUint8Array(raw.add_vkey),
    blockSlotNo: raw.block_slot_no,
    datumBlob: raw.datum,
    datum: decoded.value ?? null,
    datumError: decoded.error,
    lovelace: raw.lovelace,
    outputIndex: raw.output_index,
    scriptHashHex: hexString.fromUint8Array(raw.script_hash),
    subVkeyHex: hexString.fromUint8Array(raw.sub_vkey),
    tagHex: hexString.fromUint8Array(raw.tag),
    transactionIdHex: hexString.fromUint8Array(raw.transaction_id),
    transactionIndex: raw.transaction_index,
    blockNo: raw.block_no,
  };
};

const toStepRow = (raw: RawStepRow): StepRow => {
  const datum = raw.datum
    ? decodeSafely(Datum.uint8ArrayCodec.deserialise(raw.datum))
    : null;
  const redeemer = raw.redeemer
    ? decodeSafely(Redeemer.uint8ArrayCodec.deserialise(raw.redeemer))
    : null;
  return {
    blockSlotNo: raw.block_slot_no,
    channelTransactionIdHex: hexString.fromUint8Array(
      raw.channel_transaction_id,
    ),
    channelOutputIndex: raw.channel_output_index,
    datumBlob: raw.datum,
    datum: datum?.value ?? null,
    datumError: datum?.error ?? null,
    lovelace: raw.lovelace,
    outputIndex: raw.output_index,
    redeemerBlob: raw.redeemer,
    redeemer: redeemer?.value ?? null,
    redeemerError: redeemer?.error ?? null,
    redeemerSteps: redeemer?.value ? Redeemer.labels(redeemer.value) : [],
    transactionIdHex: hexString.fromUint8Array(raw.transaction_id),
    transactionIndex: raw.transaction_index,
    blockNo: raw.block_no,
  };
};

/**
 * Decode without failing the read.
 *
 * A blob we cannot decode is reported *next to* the row rather than aborting
 * the whole query — a decoder bug must not be able to hide the very rows it is
 * meant to be checked against. Codecs return a `Result`, so this only has to
 * flatten one into an optional value.
 */
const decodeSafely = <T>(
  result: Result<T, unknown>,
): { value?: T; error: string | null } =>
  result.isOk()
    ? { value: result.value, error: null }
    : { error: describe(result.error) };

const describe = (error: unknown): string =>
  error instanceof Error ? `${error.name}: ${error.message}` : String(error);

/**
 * Hex → bytes for a SQL parameter.
 *
 * Every hex string reaching this point came out of our own hex conversions or
 * off the CLI, so a parse failure is a bug in *this* file rather than bad data —
 * throw so it is loud instead of silently binding `null` into the query.
 */
const bytesOrThrow = (hex: string): Uint8Array =>
  unwrapOrPanicWith(
    uint8Array.stringCodec.deserialise(hex),
    (error) => `indexerDb: ${hex} is not valid hex: ${describe(error)}`,
  );

// ---------------------------------------------------------------------------
// Wiring
// ---------------------------------------------------------------------------

/**
 * Open (or reuse) a handle for `config`.
 *
 * Tests create one handle per suite and close it in `afterAll`; the value is
 * cached per `dbPath` so a scenario and its assertions share the same
 * connection — and therefore the same snapshot-free read view.
 */
const openDbs = new Map<string, IndexerDb>();

export const openIndexerDb = (
  config: IndexerDbConfig,
): Result<IndexerDb, string> => {
  const cached = openDbs.get(config.dbPath);
  if (cached) return ok(cached);
  const db = new IndexerDb(config);
  const opened = db.open();
  if (opened.isErr()) return err(opened.error);
  openDbs.set(config.dbPath, db);
  return ok(db);
};

/** Drop every cached handle. Only used by tests that create throwaway dbs. */
export const closeIndexerDbs = (): void => {
  for (const db of openDbs.values()) db.close();
  openDbs.clear();
};

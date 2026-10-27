import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { DatabaseSync } from "node:sqlite";
import * as uint8Array from "@konduit/codec/uint8Array";
import { unwrapOrPanicWith } from "@konduit/konduit-consumer/neverthrow";

/**
 * @file Builds a throw-away SQLite database that looks exactly like the one the
 * `konduit-indexer` writes.
 *
 * The schema is **not** duplicated here: we apply the real
 * `packages/konduit/indexer/sqlite/v1.0.0/001.sql`, triggers and all. That
 * matters for two reasons:
 *
 *  1. If the schema and this harness ever drift, the tests keep passing against
 *     a fiction — so the file is read at test time, not baked in.
 *  2. The triggers (`channel_no_updates`, `no_step_after_close`,
 *     `no_step_before_channel`, `focus_check_consistency`) become part of the
 *     test surface: we can assert that the *store itself* rejects bad data, not
 *     only that our query layer notices it afterwards.
 */

/**
 * Hex → bytes for a SQL parameter.
 *
 * Fixture values are literals in this file, so a parse failure is a typo in a
 * test — throw rather than binding `null` and failing somewhere less obvious.
 */
const hexToBytesOrThrow = (hex: string): Uint8Array =>
  unwrapOrPanicWith(
    uint8Array.stringCodec.deserialise(hex),
    (error) => `fixtureDb: ${hex} is not valid hex: ${error}`,
  );

export type FixtureBlock = { slotNo: number; blockNo: number };

export type FixtureChannel = {
  addVkeyHex: string;
  subVkeyHex: string;
  tagHex: string;
  scriptHashHex: string;
  transactionIdHex: string;
  transactionIndex: number;
  outputIndex: number;
  blockSlotNo: number;
  lovelace: number;
  datumHex: string;
};

export type FixtureStep = {
  channelTransactionIdHex: string;
  channelOutputIndex: number;
  transactionIdHex: string;
  transactionIndex: number;
  blockSlotNo: number;
  /** `null` renders the closing step (NULL datum/lovelace/output_index). */
  outputIndex: number | null;
  lovelace?: number | null;
  datumHex?: string | null;
  redeemerHex?: string | null;
};

export type FixtureDb = {
  path: string;
  db: DatabaseSync;
  /** The schema revision we recorded — mirrors `migrations::run_migrations`. */
  schemaVersion: number;
  close: () => void;
};

export type FixtureDbOptions = {
  /** Defaults to every `v<major>.<minor>.<patch>/*.sql` under `schemaDir`, in order. */
  migrations?: Array<{ version: number; sql: string }>;
};

/** Locate `packages/konduit/indexer/sqlite` relative to this repo. */
export const findSchemaDir = (start = process.cwd()): string => {
  let dir = start;
  for (let i = 0; i < 12; i++) {
    const candidate = path.join(
      dir,
      "packages",
      "konduit",
      "indexer",
      "sqlite",
    );
    if (fs.existsSync(candidate)) return candidate;
    const parent = path.dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  throw new Error(
    `findSchemaDir: could not find packages/konduit/indexer/sqlite above ${start}`,
  );
};

/**
 * Read the migration folders exactly the way the Rust side does:
 * `sqlite/v<major>.<minor>.<patch>/<nnn>.sql`, one revision per folder.
 */
export const readMigrations = (
  schemaDir: string,
): Array<{ version: number; sql: string }> =>
  fs
    .readdirSync(schemaDir, { withFileTypes: true })
    .filter(
      (entry) => entry.isDirectory() && /^v\d+\.\d+\.\d+$/.test(entry.name),
    )
    .map((entry) => {
      const version = Number(entry.name.slice(1).split(".")[0]);
      const dir = path.join(schemaDir, entry.name);
      const sql = fs
        .readdirSync(dir)
        .filter((file) => file.endsWith(".sql"))
        .sort()
        .map((file) => fs.readFileSync(path.join(dir, file), "utf-8"))
        .join("\n");
      return { version, sql };
    })
    .sort((a, b) => a.version - b.version);

/**
 * Create a fresh database in a temp directory with the real schema applied and
 * `PRAGMA user_version` set, mirroring
 * `konduit_indexer::store::sqlite::migrations::execute_migration`.
 */
export const createFixtureDb = (options: FixtureDbOptions = {}): FixtureDb => {
  const migrations = options.migrations ?? readMigrations(findSchemaDir());
  if (migrations.length === 0) {
    throw new Error("createFixtureDb: no migrations found");
  }
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "konduit-indexer-db-"));
  const dbPath = path.join(dir, "konduit-indexer.db");
  const db = new DatabaseSync(dbPath);
  let schemaVersion = 0;
  for (const migration of migrations) {
    db.exec("BEGIN");
    try {
      db.exec(migration.sql);
      db.exec(`PRAGMA user_version = ${migration.version};`);
      db.exec("COMMIT");
      schemaVersion = migration.version;
    } catch (error) {
      db.exec("ROLLBACK");
      throw new Error(
        `createFixtureDb: migration ${migration.version} failed: ${(error as Error).message}`,
      );
    }
  }
  return {
    path: dbPath,
    db,
    schemaVersion,
    close: () => {
      try {
        db.close();
      } catch {
        // best effort
      }
      fs.rmSync(dir, { recursive: true, force: true });
    },
  };
};

export const insertBlocks = (
  db: DatabaseSync,
  blocks: FixtureBlock[],
): void => {
  for (const block of blocks) {
    db.prepare(
      "INSERT INTO block (block_no, header_hash, slot_no) VALUES (?, ?, ?)",
    ).run(block.blockNo, blockHash(block.slotNo), block.slotNo);
  }
};

/** A stable, deterministic 32-byte "header hash" per slot. */
export const blockHash = (slotNo: number): Uint8Array => {
  const out = new Uint8Array(32);
  new DataView(out.buffer).setUint32(0, slotNo, false);
  return out;
};

export const insertChannel = (
  db: DatabaseSync,
  channel: FixtureChannel,
): void => {
  db.prepare(
    `INSERT INTO channel (
        add_vkey, block_slot_no, datum, lovelace, output_index,
        script_hash, sub_vkey, tag, transaction_id, transaction_index
     ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
  ).run(
    hexToBytesOrThrow(channel.addVkeyHex),
    channel.blockSlotNo,
    hexToBytesOrThrow(channel.datumHex),
    channel.lovelace,
    channel.outputIndex,
    hexToBytesOrThrow(channel.scriptHashHex),
    hexToBytesOrThrow(channel.subVkeyHex),
    hexToBytesOrThrow(channel.tagHex),
    hexToBytesOrThrow(channel.transactionIdHex),
    channel.transactionIndex,
  );
};

export const insertStep = (db: DatabaseSync, step: FixtureStep): void => {
  const closing = step.outputIndex === null;
  const datum = step.datumHex ?? null;
  db.prepare(
    `INSERT INTO step (
        block_slot_no, channel_transaction_id, channel_output_index,
        datum, lovelace, output_index, redeemer,
        transaction_id, transaction_index
     ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
  ).run(
    step.blockSlotNo,
    hexToBytesOrThrow(step.channelTransactionIdHex),
    step.channelOutputIndex,
    datum === null ? null : hexToBytesOrThrow(datum),
    closing ? null : (step.lovelace ?? null),
    step.outputIndex,
    step.redeemerHex === undefined || step.redeemerHex === null
      ? null
      : hexToBytesOrThrow(step.redeemerHex),
    hexToBytesOrThrow(step.transactionIdHex),
    step.transactionIndex,
  );
};

export const insertFocus = (
  db: DatabaseSync,
  focus: {
    addVkeyHex: string;
    tagHex: string;
    channelTransactionIdHex: string;
    channelTransactionIndex: number;
  },
): void => {
  db.prepare(
    `INSERT INTO focus (add_vkey, tag, channel_transaction_id, channel_transaction_index)
     VALUES (?, ?, ?, ?)`,
  ).run(
    hexToBytesOrThrow(focus.addVkeyHex),
    hexToBytesOrThrow(focus.tagHex),
    hexToBytesOrThrow(focus.channelTransactionIdHex),
    focus.channelTransactionIndex,
  );
};

// ---------------------------------------------------------------------------
// Deterministic identifiers for the fixtures
// ---------------------------------------------------------------------------

/** `0x..01`-style 32-byte ids so a failure message names the right row. */
export const idHex = (n: number): string =>
  n.toString(16).padStart(2, "0").repeat(32).slice(0, 64);

export const txId = (n: number): string => idHex(n);
export const vkeyHex = (n: number): string => idHex(n);
/** The validator script hash is 28 bytes, per 001.sql's `length()` CHECK. */
export const scriptHashHex = (n: number): string => idHex(n).slice(0, 56);

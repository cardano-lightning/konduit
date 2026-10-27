import { afterEach, describe, expect, it } from "vitest";
import { unwrapOrPanic } from "@konduit/konduit-consumer/neverthrow";
import type { ChannelThread } from "../src/indexerDb.js";
import { IndexerDb, isChannelClosed } from "../src/indexerDb.js";
import { Step } from "../src/konduit/redeemer.js";
import {
  assertChannelMatchesChain,
  assertClosingStepShape,
  assertDbMatchesCli,
  assertThreadInvariants,
  formatViolations,
  indexedAmount,
  indexedSubbed,
  indexedUseds,
  openIndexerDbFor,
  stageLabel,
  verifyIndexerDbSchema,
  waitForExactlyOneChannel,
  waitForIndexedChannel,
  waitForIndexedSteps,
  waitForIndexedTipAfter,
  waitForStepInTransaction,
  EXPECTED_SCHEMA_VERSION,
  REQUIRED_TABLES,
} from "../src/scenarios/indexing.js";
import { Datum } from "../src/konduit/datum.js";
import {
  createFixtureDb,
  findSchemaDir,
  insertBlocks,
  insertChannel,
  insertFocus,
  insertStep,
  readMigrations,
  scriptHashHex,
  txId,
  vkeyHex,
  type FixtureDb,
} from "./support/fixtureDb.js";
import {
  DATUM_ADDED,
  DATUM_CLOSED,
  DATUM_OPENED,
  DATUM_SUBBED,
  ADDED_LOVELACE,
  FIXTURE_ADD_VKEY_HEX,
  FIXTURE_SUB_VKEY_HEX,
  FIXTURE_TAG_HEX,
  REDEEMER_ADD,
  REDEEMER_CLOSE,
  REDEEMER_MUTUAL,
  REDEEMER_SUB,
  OPENED_LOVELACE,
} from "./support/konduitCbor.js";

// ---------------------------------------------------------------------------
// Fixture world
//
// One thread, built step by step, mirroring what `open -> add -> sub -> close`
// leaves in the store:
//
//   slot 100  channel  tx…01#0  lovelace 20 ada   datum DATUM_OPENED
//   slot 110  step     tx…02#1  lovelace 100 ada  datum DATUM_ADDED     redeemer Cont:Add
//   slot 120  step     tx…03#2  lovelace 91 ada   datum DATUM_SUBBED   redeemer Cont:Sub
//   slot 130  step     tx…04    (closing)                            redeemer Mutual
// ---------------------------------------------------------------------------

const SLOTS = { channel: 100, add: 110, sub: 120, close: 130 } as const;

/**
 * The vkeys must be the ones baked into the Rust-generated fixture datums —
 * `assertThreadInvariants` cross-checks the `channel` columns against the
 * datum's own copy of them, so any other value is (correctly) a violation.
 */
const ADD_VKEY = FIXTURE_ADD_VKEY_HEX;
const SUB_VKEY = FIXTURE_SUB_VKEY_HEX;
const SCRIPT_HASH = scriptHashHex(0xc3);

const CHANNEL_TX = txId(0x01);
const ADD_TX = txId(0x02);
const SUB_TX = txId(0x03);
const CLOSE_TX = txId(0x04);

const KEYTAG = { addVkeyHex: ADD_VKEY, tagHex: FIXTURE_TAG_HEX };

/** A tiny env stub so the polling helpers can be exercised without a dev-env. */
const fakeEnv = (
  overrides: Partial<{ timeoutMs: number; pollMs: number }> = {},
) => ({
  indexerDb: {
    dbPath: "",
    enabled: true,
    timeoutMs: overrides.timeoutMs ?? 1_000,
    pollMs: overrides.pollMs ?? 10,
    busyTimeoutMs: 0,
  },
});

const openFixtures: FixtureDb[] = [];

const newFixtureDb = (): FixtureDb => {
  const fixture = createFixtureDb();
  openFixtures.push(fixture);
  insertBlocks(fixture.db, [
    { slotNo: 99, blockNo: 8 },
    { slotNo: SLOTS.channel, blockNo: 9 },
    { slotNo: SLOTS.add, blockNo: 10 },
    { slotNo: SLOTS.sub, blockNo: 11 },
    { slotNo: SLOTS.close, blockNo: 12 },
    { slotNo: 200, blockNo: 13 },
  ]);
  return fixture;
};

/** The full open → add → sub → close thread. */
const seedFullThread = (fixture: FixtureDb): void => {
  insertChannel(fixture.db, {
    addVkeyHex: ADD_VKEY,
    subVkeyHex: SUB_VKEY,
    tagHex: FIXTURE_TAG_HEX,
    scriptHashHex: SCRIPT_HASH,
    transactionIdHex: CHANNEL_TX,
    transactionIndex: 0,
    outputIndex: 0,
    blockSlotNo: SLOTS.channel,
    lovelace: 20_000_000,
    datumHex: DATUM_OPENED,
  });
  insertStep(fixture.db, {
    channelTransactionIdHex: CHANNEL_TX,
    channelOutputIndex: 0,
    transactionIdHex: ADD_TX,
    transactionIndex: 0,
    blockSlotNo: SLOTS.add,
    outputIndex: 1,
    lovelace: 100_000_000,
    datumHex: DATUM_ADDED,
    redeemerHex: REDEEMER_ADD,
  });
  insertStep(fixture.db, {
    channelTransactionIdHex: CHANNEL_TX,
    channelOutputIndex: 0,
    transactionIdHex: SUB_TX,
    transactionIndex: 0,
    blockSlotNo: SLOTS.sub,
    outputIndex: 2,
    lovelace: 91_000_000,
    datumHex: DATUM_SUBBED,
    redeemerHex: REDEEMER_SUB,
  });
  insertStep(fixture.db, {
    channelTransactionIdHex: CHANNEL_TX,
    channelOutputIndex: 0,
    transactionIdHex: CLOSE_TX,
    transactionIndex: 0,
    blockSlotNo: SLOTS.close,
    outputIndex: null,
    redeemerHex: REDEEMER_MUTUAL,
  });
};

const openReadonly = (fixture: FixtureDb): IndexerDb => {
  // `fixture.db` holds an open write handle; SQLite allows a second read-only
  // connection to the same file, which is exactly the shape the tests need.
  const db = new IndexerDb({
    dbPath: fixture.path,
    busyTimeoutMs: 0,
  });
  return unwrapOrPanic(db.open(), "fixture open failed");
};

afterEach(() => {
  while (openFixtures.length > 0) openFixtures.pop()!.close();
});

// ---------------------------------------------------------------------------

describe("konduit-indexer db :: schema", () => {
  it("finds the real migration folder the Rust store embeds", () => {
    const schemaDir = findSchemaDir();
    const migrations = readMigrations(schemaDir);
    expect(migrations.length).toBeGreaterThan(0);
    expect(migrations[0]!.version).toBe(1);
    // The tables and triggers the tests depend on must come from the real file.
    expect(migrations[0]!.sql).toContain("CREATE TABLE channel");
    expect(migrations[0]!.sql).toContain("CREATE TABLE step");
    expect(migrations[0]!.sql).toContain("CREATE TRIGGER no_step_after_close");
  });

  it("records the schema revision the indexer binary would record", () => {
    const fixture = createFixtureDb();
    openFixtures.push(fixture);
    expect(fixture.schemaVersion).toBe(EXPECTED_SCHEMA_VERSION);
  });

  it("passes the schema gate against a correctly migrated db", () => {
    const fixture = newFixtureDb();
    const db = openReadonly(fixture);
    const verified = verifyIndexerDbSchema(db);
    expect(verified.isOk(), verified.isErr() ? verified.error : "").toBe(true);
    db.close();
  });

  it("fails the schema gate on an unmigrated db, with the missing tables named", () => {
    const fixture = createFixtureDb();
    openFixtures.push(fixture);
    // Roll the recorded revision back the way a half-applied migration would.
    fixture.db.exec("PRAGMA user_version = 0");
    const db = openReadonly(fixture);
    const verified = verifyIndexerDbSchema(db);
    expect(verified.isErr()).toBe(true);
    if (verified.isErr()) expect(verified.error).toContain("schema revision 0");
    db.close();
  });

  it("names every table 001.sql must create", () => {
    const fixture = newFixtureDb();
    const db = openReadonly(fixture);
    const tables = db.tables();
    expect(tables.isOk()).toBe(true);
    if (tables.isOk()) {
      for (const required of REQUIRED_TABLES) {
        expect(tables.value).toContain(required);
      }
    }
    db.close();
  });

  it("rejects a db path that does not exist instead of creating one", () => {
    const db = new IndexerDb({
      dbPath: "/nonexistent/konduit-indexer.db",
      busyTimeoutMs: 0,
    });
    const opened = db.open();
    expect(opened.isErr()).toBe(true);
    if (opened.isErr()) expect(opened.error).toContain("indexer db not found");
  });
});

// ---------------------------------------------------------------------------

describe("konduit-indexer db :: blocks", () => {
  it("reports the highest indexed block as the tip", () => {
    const fixture = newFixtureDb();
    const db = openReadonly(fixture);
    const tip = db.tip();
    expect(tip.isOk()).toBe(true);
    if (tip.isOk()) {
      expect(tip.value?.blockNo).toBe(13);
      expect(tip.value?.slotNo).toBe(200);
      // 32-byte header hash, hex encoded.
      expect(tip.value?.headerHashHex).toMatch(/^[0-9a-f]{64}$/);
    }
    db.close();
  });

  it("reports an empty block table as no tip", () => {
    const fixture = createFixtureDb();
    openFixtures.push(fixture);
    const db = openReadonly(fixture);
    const tip = db.tip();
    expect(tip.isOk() && tip.value === null).toBe(true);
    db.close();
  });

  it("counts every table", () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    insertFocus(fixture.db, {
      addVkeyHex: ADD_VKEY,
      tagHex: FIXTURE_TAG_HEX,
      channelTransactionIdHex: CHANNEL_TX,
      channelTransactionIndex: 0,
    });
    const db = openReadonly(fixture);
    const counts = db.counts();
    expect(counts.isOk()).toBe(true);
    if (counts.isOk()) {
      expect(counts.value).toEqual({ block: 6, channel: 1, step: 3, focus: 1 });
    }
    db.close();
  });

  it("waits for the tip to pass a slot", async () => {
    const fixture = newFixtureDb();
    const db = openReadonly(fixture);
    const reached = await waitForIndexedTipAfter(db, 200, 1_000, 10);
    expect(reached.isOk()).toBe(true);
    if (reached.isOk()) expect(reached.value.slotNo).toBe(200);
    db.close();
  });

  it("explains a tip that never moves instead of hanging", async () => {
    const fixture = newFixtureDb();
    const db = openReadonly(fixture);
    const stuck = await waitForIndexedTipAfter(db, 9_000, 120, 10);
    expect(stuck.isErr()).toBe(true);
    if (stuck.isErr()) {
      expect(stuck.error.reason).toContain("tip is at slot 200");
      expect(stuck.error.elapsedMs).toBeGreaterThan(0);
    }
    db.close();
  });
});

// ---------------------------------------------------------------------------

describe("konduit-indexer db :: channels", () => {
  it("decodes the channel datum, keys and lovelace", () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    const db = openReadonly(fixture);
    const found = db.channelsByKeytag(KEYTAG);
    expect(found.isOk()).toBe(true);
    const channel = found.isOk() ? found.value[0] : undefined;
    expect(channel).toBeDefined();
    if (!channel) return;
    expect(channel.transactionIdHex).toBe(CHANNEL_TX);
    expect(channel.outputIndex).toBe(0);
    expect(channel.blockSlotNo).toBe(SLOTS.channel);
    expect(channel.blockNo).toBe(9);
    expect(channel.lovelace).toBe(20_000_000);
    expect(channel.addVkeyHex).toBe(ADD_VKEY);
    expect(channel.subVkeyHex).toBe(SUB_VKEY);
    expect(channel.scriptHashHex).toBe(SCRIPT_HASH);
    expect(channel.tagHex).toBe(FIXTURE_TAG_HEX);
    expect(channel.datumError).toBeNull();
    expect(channel.datum?.stage).toEqual({
      label: "Opened",
      subbed: 0n,
      useds: [],
    });
    // The channel's value lives in the `lovelace` column, not the datum.
    expect(channel.lovelace).toBe(Number(OPENED_LOVELACE));
    db.close();
  });

  it("agrees with the datum's own copy of the keytag and vkeys", () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    const db = openReadonly(fixture);
    const thread = db.threadByKeytag(KEYTAG);
    expect(thread.isOk() && thread.value !== null).toBe(true);
    const value = thread.isOk() ? thread.value : null;
    if (!value) return;
    const violations = assertThreadInvariants(value);
    expect(
      violations.isOk(),
      violations.isErr() ? formatViolations(violations.error) : "",
    ).toBe(true);
    db.close();
  });

  it("finds nothing for an unknown keytag", () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    const db = openReadonly(fixture);
    const found = db.channelsByKeytag({ addVkeyHex: ADD_VKEY, tagHex: "ffff" });
    expect(found.isOk() && found.value.length).toBe(0);
    db.close();
  });

  it("returns every channel sharing a keytag (the schema allows mimics)", () => {
    const fixture = newFixtureDb();
    // Two distinct channels under the same (add_vkey, tag) — exactly the
    // situation 001.sql's comment warns about.
    for (const [n, slot] of [
      [0x11, 100],
      [0x12, 110],
    ] as const) {
      insertChannel(fixture.db, {
        addVkeyHex: ADD_VKEY,
        subVkeyHex: SUB_VKEY,
        tagHex: FIXTURE_TAG_HEX,
        scriptHashHex: SCRIPT_HASH,
        transactionIdHex: txId(n),
        transactionIndex: 0,
        outputIndex: 0,
        blockSlotNo: slot,
        lovelace: 20_000_000,
        datumHex: DATUM_OPENED,
      });
    }
    const db = openReadonly(fixture);
    const found = db.channelsByKeytag(KEYTAG);
    expect(found.isOk() && found.value.length).toBe(2);
    db.close();
  });

  it("refuses to attach a datum to a mismatched keytag (focus trigger)", () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    expect(() =>
      insertFocus(fixture.db, {
        addVkeyHex: ADD_VKEY,
        tagHex: "deadbeef",
        channelTransactionIdHex: CHANNEL_TX,
        channelTransactionIndex: 0,
      }),
    ).toThrow(/focus\.add_vkey and focus\.tag must match the channel/);
  });

  it("stores a focus row and reads it back", () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    insertFocus(fixture.db, {
      addVkeyHex: ADD_VKEY,
      tagHex: FIXTURE_TAG_HEX,
      channelTransactionIdHex: CHANNEL_TX,
      channelTransactionIndex: 0,
    });
    const db = openReadonly(fixture);
    const rows = db.focusRows();
    expect(rows.isOk() && rows.value).toHaveLength(1);
    if (rows.isOk() && rows.value[0]) {
      expect(rows.value[0].channelTransactionIdHex).toBe(CHANNEL_TX);
      expect(rows.value[0].tagHex).toBe(FIXTURE_TAG_HEX);
    }
    db.close();
  });
});

// ---------------------------------------------------------------------------

describe("konduit-indexer db :: steps", () => {
  it("returns steps in chain order with their redeemers decoded", () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    const db = openReadonly(fixture);
    const steps = db.stepsByChannel({
      transactionIdHex: CHANNEL_TX,
      outputIndex: 0,
    });
    expect(steps.isOk()).toBe(true);
    const rows = steps.isOk() ? steps.value : [];
    expect(rows.map((s) => s.blockSlotNo)).toEqual([110, 120, 130]);
    expect(rows.map((s) => s.redeemerSteps)).toEqual([
      ["Cont:Add"],
      ["Cont:Sub"],
      ["Mutual"],
    ]);
    expect(rows[0]!.lovelace).toBe(100_000_000);
    expect(rows[0]!.outputIndex).toBe(1);
    // An `add` leaves the stage untouched — the datum is identical to the
    // channel's and only the lovelace column moves.
    expect(rows[0]!.datum?.stage).toEqual({
      label: "Opened",
      subbed: 0n,
      useds: [],
    });
    expect(rows[0]!.lovelace).toBe(Number(ADDED_LOVELACE));
    // The closing step carries no continuation output — 001.sql's CHECK.
    expect(rows[2]!.outputIndex).toBeNull();
    expect(rows[2]!.lovelace).toBeNull();
    expect(rows[2]!.datumBlob).toBeNull();
    db.close();
  });

  it("summarises a sub redeemer down to its squash and cheque counts", () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    const db = openReadonly(fixture);
    const steps = db.stepsByChannel({
      transactionIdHex: CHANNEL_TX,
      outputIndex: 0,
    });
    const sub = steps.isOk() ? steps.value[1] : undefined;
    expect(sub?.redeemer?.kind).toBe("Main");
    if (sub?.redeemer?.kind === "Main") {
      const step = sub.redeemer.steps[0]!;
      expect(Step.label(step)).toBe("Cont:Sub");
      if ("cont" in step && step.cont.kind === "Sub") {
        expect(step.cont.items).toBe(1);
      }
    }
    db.close();
  });

  it("reports a corrupt blob next to the row instead of aborting the query", () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    // Plant a step whose redeemer is not valid CBOR. The store happily accepts
    // it — there is no CHECK on blob content — which is exactly why the read
    // path has to surface the decode failure rather than trust the column.
    insertStep(fixture.db, {
      channelTransactionIdHex: CHANNEL_TX,
      channelOutputIndex: 0,
      transactionIdHex: txId(0x09),
      transactionIndex: 0,
      blockSlotNo: SLOTS.sub,
      outputIndex: 3,
      lovelace: 91_000_000,
      datumHex: DATUM_SUBBED,
      redeemerHex: "ffff",
    });
    const db = openReadonly(fixture);
    const steps = db.stepsByChannel({
      transactionIdHex: CHANNEL_TX,
      outputIndex: 0,
    });
    expect(steps.isOk()).toBe(true);
    const rows = steps.isOk() ? steps.value : [];
    // The other steps are still returned — one bad blob must not hide the rows
    // we are being checked against.
    expect(rows).toHaveLength(4);
    const corrupt = rows.find((s) => s.transactionIdHex === txId(0x09))!;
    expect(corrupt.redeemer).toBeNull();
    // The message comes from `@konduit/codec`'s reader — "Unexpected break
    // byte" for a bare `ff`, which is exactly the blob we planted.
    expect(corrupt.redeemerError).toBeTruthy();
    expect(corrupt.redeemerError).toMatch(
      /break byte|array|Datum|constructor/i,
    );
    expect(corrupt.redeemerSteps).toEqual([]);
    expect(rows[0]!.redeemerSteps).toEqual(["Cont:Add"]);
    db.close();
  });

  it("finds a step by its transaction id", () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    const db = openReadonly(fixture);
    const has = db.hasStepInTransaction(
      { transactionIdHex: CHANNEL_TX, outputIndex: 0 },
      SUB_TX,
    );
    expect(has.isOk() && has.value).toBe(true);
    const missing = db.hasStepInTransaction(
      { transactionIdHex: CHANNEL_TX, outputIndex: 0 },
      txId(0x99),
    );
    expect(missing.isOk() && missing.value).toBe(false);
    db.close();
  });
});

// ---------------------------------------------------------------------------

describe("konduit-indexer db :: thread assembly", () => {
  it("reports the live state of an open thread", () => {
    const fixture = newFixtureDb();
    insertChannel(fixture.db, {
      addVkeyHex: ADD_VKEY,
      subVkeyHex: SUB_VKEY,
      tagHex: FIXTURE_TAG_HEX,
      scriptHashHex: SCRIPT_HASH,
      transactionIdHex: CHANNEL_TX,
      transactionIndex: 0,
      outputIndex: 0,
      blockSlotNo: SLOTS.channel,
      lovelace: 20_000_000,
      datumHex: DATUM_OPENED,
    });
    insertStep(fixture.db, {
      channelTransactionIdHex: CHANNEL_TX,
      channelOutputIndex: 0,
      transactionIdHex: ADD_TX,
      transactionIndex: 0,
      blockSlotNo: SLOTS.add,
      outputIndex: 1,
      lovelace: 100_000_000,
      datumHex: DATUM_ADDED,
      redeemerHex: REDEEMER_ADD,
    });
    const db = openReadonly(fixture);
    const thread = db.threadByKeytag(KEYTAG);
    const value = thread.isOk() ? thread.value : null;
    expect(value).not.toBeNull();
    if (!value) return;
    expect(stageLabel(value.initialStage)).toBe("Opened");
    expect(stageLabel(value.currentStage)).toBe("Opened");
    expect(indexedAmount(value)).toBe(100_000_000);
    expect(value.terminated).toBe(false);
    expect(isChannelClosed(value)).toBe(false);
    db.close();
  });

  it("reports a thread terminated by a mutual close as having no current state", () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    const db = openReadonly(fixture);
    const thread = db.threadByKeytag(KEYTAG);
    const value = thread.isOk() ? thread.value : null;
    expect(value).not.toBeNull();
    if (!value) return;
    expect(value.terminated).toBe(true);
    // `Mutual` is an end-of-life step, so the channel counts as closed
    // even though the store keeps no datum for it.
    expect(isChannelClosed(value)).toBe(true);
    expect(value.currentStage).toBeNull();
    expect(indexedAmount(value)).toBeNull();
    // The root datum is still readable even though the thread is over.
    expect(value.initialStage).toEqual({
      label: "Opened",
      subbed: 0n,
      useds: [],
    });
    db.close();
  });

  it("reads a thread whose channel has no steps yet", () => {
    const fixture = newFixtureDb();
    insertChannel(fixture.db, {
      addVkeyHex: ADD_VKEY,
      subVkeyHex: SUB_VKEY,
      tagHex: FIXTURE_TAG_HEX,
      scriptHashHex: SCRIPT_HASH,
      transactionIdHex: CHANNEL_TX,
      transactionIndex: 0,
      outputIndex: 0,
      blockSlotNo: SLOTS.channel,
      lovelace: 20_000_000,
      datumHex: DATUM_OPENED,
    });
    const db = openReadonly(fixture);
    const thread = db.threadByKeytag(KEYTAG);
    const value = thread.isOk() ? thread.value : null;
    if (!value) throw new Error("expected a thread");
    expect(value.steps).toHaveLength(0);
    expect(value.terminated).toBe(false);
    // With no steps the channel's own datum *is* the current state.
    expect(value.currentStage).toBe(value.initialStage);
    db.close();
  });
});

// ---------------------------------------------------------------------------

describe("konduit-indexer db :: datum projections", () => {
  it("reads the subbed total out of the live datum, not out of the UTxO value", () => {
    // The whole point of the `subbed` field: it is the only cumulative figure
    // the datum carries, and it must move on a sub while the UTxO value moves
    // in the *opposite* direction (value leaves the channel).
    const fixture = newFixtureDb();
    insertChannel(fixture.db, {
      addVkeyHex: ADD_VKEY,
      subVkeyHex: SUB_VKEY,
      tagHex: FIXTURE_TAG_HEX,
      scriptHashHex: SCRIPT_HASH,
      transactionIdHex: CHANNEL_TX,
      transactionIndex: 0,
      outputIndex: 0,
      blockSlotNo: SLOTS.channel,
      lovelace: 20_000_000,
      datumHex: DATUM_OPENED,
    });
    insertStep(fixture.db, {
      channelTransactionIdHex: CHANNEL_TX,
      channelOutputIndex: 0,
      transactionIdHex: SUB_TX,
      transactionIndex: 0,
      blockSlotNo: SLOTS.sub,
      outputIndex: 1,
      lovelace: 91_000_000,
      datumHex: DATUM_SUBBED,
      redeemerHex: REDEEMER_SUB,
    });
    const db = openReadonly(fixture);
    const thread = db.threadByKeytag(KEYTAG);
    if (!thread.isOk() || !thread.value) throw new Error("expected a thread");
    expect(indexedSubbed(thread.value)).toBe(7_000_000);
    expect(indexedUseds(thread.value)).toEqual([1, 2]);
    expect(indexedAmount(thread.value)).toBe(91_000_000);
    db.close();
  });

  it("falls back to the channel row when there are no steps", () => {
    const fixture = newFixtureDb();
    insertChannel(fixture.db, {
      addVkeyHex: ADD_VKEY,
      subVkeyHex: SUB_VKEY,
      tagHex: FIXTURE_TAG_HEX,
      scriptHashHex: SCRIPT_HASH,
      transactionIdHex: CHANNEL_TX,
      transactionIndex: 0,
      outputIndex: 0,
      blockSlotNo: SLOTS.channel,
      lovelace: 20_000_000,
      datumHex: DATUM_OPENED,
    });
    const db = openReadonly(fixture);
    const thread = db.threadByKeytag(KEYTAG);
    if (!thread.isOk() || !thread.value) throw new Error("expected a thread");
    expect(indexedAmount(thread.value)).toBe(20_000_000);
    expect(indexedSubbed(thread.value)).toBe(0);
    expect(indexedUseds(thread.value)).toEqual([]);
    db.close();
  });

  it("reports nothing once the thread is terminated", () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    const db = openReadonly(fixture);
    const thread = db.threadByKeytag(KEYTAG);
    if (!thread.isOk() || !thread.value) throw new Error("expected a thread");
    expect(indexedAmount(thread.value)).toBeNull();
    expect(indexedSubbed(thread.value)).toBeNull();
    expect(indexedUseds(thread.value)).toEqual([]);
    db.close();
  });
});

describe("konduit-indexer db :: invariants", () => {
  it("accepts a well-formed thread", () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    const db = openReadonly(fixture);
    const thread = db.threadByKeytag(KEYTAG);
    if (!thread.isOk() || !thread.value) throw new Error("expected a thread");
    const result = assertThreadInvariants(thread.value);
    expect(
      result.isOk(),
      result.isErr() ? formatViolations(result.error) : "",
    ).toBe(true);
    db.close();
  });

  it("catches a step pointing at a different channel", () => {
    // SQLite's foreign key + the `no_step_after_close` trigger make this state
    // unreachable through the store, so it is exercised on the invariant
    // function directly: a buggy indexer that mis-attributes a transition is
    // exactly the regression the assertion exists for.
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    const db = openReadonly(fixture);
    const loaded = db.threadByKeytag(KEYTAG);
    if (!loaded.isOk() || !loaded.value) throw new Error("expected a thread");
    db.close();

    const tampered: ChannelThread = {
      ...loaded.value,
      steps: [
        {
          ...loaded.value.steps[0]!,
          channelTransactionIdHex: txId(0xee),
          channelOutputIndex: 7,
        },
        ...loaded.value.steps.slice(1),
      ],
    };
    const result = assertThreadInvariants(tampered);
    expect(result.isErr()).toBe(true);
    if (result.isErr()) {
      expect(result.error.map((v) => v.invariant)).toContain(
        "step points at its channel",
      );
    }
  });

  it("catches steps that are not in chain order", () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    const db = openReadonly(fixture);
    const loaded = db.threadByKeytag(KEYTAG);
    if (!loaded.isOk() || !loaded.value) throw new Error("expected a thread");
    db.close();

    const [first, second] = loaded.value.steps;
    const swapped: ChannelThread = {
      ...loaded.value,
      steps: [
        { ...second!, blockSlotNo: SLOTS.sub },
        { ...first!, blockSlotNo: SLOTS.add },
      ],
    };
    const result = assertThreadInvariants(swapped);
    expect(result.isErr()).toBe(true);
    if (result.isErr()) {
      expect(result.error.map((v) => v.invariant)).toContain(
        "steps are in chain order",
      );
    }
  });

  it("breaks the sort on equal slots by transaction index", () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    const db = openReadonly(fixture);
    const loaded = db.threadByKeytag(KEYTAG);
    if (!loaded.isOk() || !loaded.value) throw new Error("expected a thread");
    db.close();

    const [first, second] = loaded.value.steps;
    const sameSlot: ChannelThread = {
      ...loaded.value,
      steps: [
        { ...first!, blockSlotNo: SLOTS.sub, transactionIndex: 1 },
        { ...second!, blockSlotNo: SLOTS.sub, transactionIndex: 0 },
      ],
    };
    const result = assertThreadInvariants(sameSlot);
    expect(result.isErr()).toBe(true);
    if (result.isErr()) {
      expect(result.error.map((v) => v.invariant)).toContain(
        "steps are in chain order",
      );
    }
  });

  it("the store itself forbids a step after a close", () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    expect(() =>
      insertStep(fixture.db, {
        channelTransactionIdHex: CHANNEL_TX,
        channelOutputIndex: 0,
        transactionIdHex: txId(0x07),
        transactionIndex: 0,
        blockSlotNo: SLOTS.close + 10,
        outputIndex: 5,
        lovelace: 1_000_000,
        datumHex: DATUM_OPENED,
        redeemerHex: REDEEMER_ADD,
      }),
    ).toThrow(/Any step after a `close` is not possible/);
  });

  it("the store itself forbids updating a channel", () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    expect(() =>
      fixture.db
        .prepare("UPDATE channel SET lovelace = ? WHERE transaction_id = ?")
        .run(1, Buffer.from(CHANNEL_TX, "hex")),
    ).toThrow(/channel table does not allow updates/);
  });

  it("catches a channel that claims value it never got", () => {
    // Nothing in the schema ties `channel.lovelace` to the datum, because the
    // datum does not carry the amount at all — so a store bug here would be
    // invisible from SQL alone. The scenario-level cross-check against the CLI
    // is what catches it; here we only pin that the read path reports both
    // numbers faithfully.
    const fixture = newFixtureDb();
    insertChannel(fixture.db, {
      addVkeyHex: ADD_VKEY,
      subVkeyHex: SUB_VKEY,
      tagHex: FIXTURE_TAG_HEX,
      scriptHashHex: SCRIPT_HASH,
      transactionIdHex: CHANNEL_TX,
      transactionIndex: 0,
      outputIndex: 0,
      blockSlotNo: SLOTS.channel,
      lovelace: 999,
      datumHex: DATUM_OPENED,
    });
    const db = openReadonly(fixture);
    const found = db.channelsByKeytag(KEYTAG);
    const channel = found.isOk() ? found.value[0] : undefined;
    if (!channel) throw new Error("expected a channel");
    expect(channel.lovelace).toBe(999);
    expect(channel.datum?.stage).toEqual({
      label: "Opened",
      subbed: 0n,
      useds: [],
    });
    db.close();
  });

  it("catches a channel whose keytag column disagrees with its datum", () => {
    const fixture = newFixtureDb();
    // The datum says tag=0102030405060708; the column says something else.
    insertChannel(fixture.db, {
      addVkeyHex: ADD_VKEY,
      subVkeyHex: SUB_VKEY,
      tagHex: "aabbccddeeff0011",
      scriptHashHex: SCRIPT_HASH,
      transactionIdHex: CHANNEL_TX,
      transactionIndex: 0,
      outputIndex: 0,
      blockSlotNo: SLOTS.channel,
      lovelace: 20_000_000,
      datumHex: DATUM_OPENED,
    });
    const db = openReadonly(fixture);
    const thread = db.threadByKeytag({
      addVkeyHex: ADD_VKEY,
      tagHex: "aabbccddeeff0011",
    });
    if (!thread.isOk() || !thread.value) throw new Error("expected a thread");
    const result = assertThreadInvariants(thread.value);
    expect(result.isErr()).toBe(true);
    if (result.isErr()) {
      expect(formatViolations(result.error)).toContain(
        "channel.tag matches its datum constants",
      );
    }
    db.close();
  });
});

// ---------------------------------------------------------------------------

describe("konduit-indexer db :: chain cross-checks", () => {
  it("accepts a channel row that matches what the chain reported", () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    const db = openReadonly(fixture);
    const found = db.channelsByKeytag(KEYTAG);
    const channel = found.isOk() ? found.value[0] : undefined;
    if (!channel) throw new Error("expected a channel");
    const result = assertChannelMatchesChain(channel, {
      openTxId: CHANNEL_TX,
      addVkeyHex: ADD_VKEY,
      subVkeyHex: SUB_VKEY,
      tagHex: FIXTURE_TAG_HEX,
      scriptHashHex: SCRIPT_HASH,
      lovelace: 20_000_000,
    });
    expect(
      result.isOk(),
      result.isErr() ? formatViolations(result.error) : "",
    ).toBe(true);
    db.close();
  });

  it("names every field that disagrees with the chain", () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    const db = openReadonly(fixture);
    const found = db.channelsByKeytag(KEYTAG);
    const channel = found.isOk() ? found.value[0] : undefined;
    if (!channel) throw new Error("expected a channel");
    const result = assertChannelMatchesChain(channel, {
      openTxId: txId(0xee),
      addVkeyHex: vkeyHex(0xff),
      subVkeyHex: SUB_VKEY,
      scriptHashHex: scriptHashHex(0xdd),
      lovelace: 999,
    });
    expect(result.isErr()).toBe(true);
    if (result.isErr()) {
      const names = result.error.map((v) => v.invariant);
      expect(names).toContain("channel.transaction_id is the open tx");
      expect(names).toContain("channel.add_vkey is the consumer vkey");
      expect(names).toContain("channel.script_hash is the konduit validator");
      expect(names).toContain("channel.lovelace is the on-chain UTxO value");
      // Fields we did not pin down must not be reported.
      expect(names).not.toContain("channel.tag is the scenario tag");
    }
    db.close();
  });

  it("skips unset expectations rather than asserting against undefined", () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    const db = openReadonly(fixture);
    const found = db.channelsByKeytag(KEYTAG);
    const channel = found.isOk() ? found.value[0] : undefined;
    if (!channel) throw new Error("expected a channel");
    expect(assertChannelMatchesChain(channel, {}).isOk()).toBe(true);
    db.close();
  });

  it("compares the store's live state against the CLI's", () => {
    const fixture = newFixtureDb();
    insertChannel(fixture.db, {
      addVkeyHex: ADD_VKEY,
      subVkeyHex: SUB_VKEY,
      tagHex: FIXTURE_TAG_HEX,
      scriptHashHex: SCRIPT_HASH,
      transactionIdHex: CHANNEL_TX,
      transactionIndex: 0,
      outputIndex: 0,
      blockSlotNo: SLOTS.channel,
      lovelace: 20_000_000,
      datumHex: DATUM_OPENED,
    });
    insertStep(fixture.db, {
      channelTransactionIdHex: CHANNEL_TX,
      channelOutputIndex: 0,
      transactionIdHex: ADD_TX,
      transactionIndex: 0,
      blockSlotNo: SLOTS.add,
      outputIndex: 1,
      lovelace: 100_000_000,
      datumHex: DATUM_ADDED,
      redeemerHex: REDEEMER_ADD,
    });
    const db = openReadonly(fixture);
    const thread = db.threadByKeytag(KEYTAG);
    if (!thread.isOk() || !thread.value) throw new Error("expected a thread");

    expect(
      assertDbMatchesCli(thread.value, {
        stageLabel: "opened",
        bufferedLovelace: 100_000_000,
      }).isOk(),
    ).toBe(true);

    const drifted = assertDbMatchesCli(thread.value, {
      stageLabel: "opened",
      bufferedLovelace: 100_000_001,
    });
    expect(drifted.isErr()).toBe(true);
    if (drifted.isErr()) {
      expect(drifted.error[0]!.invariant).toBe(
        "channel amount agrees with the CLI",
      );
    }
    db.close();
  });

  it("does not compare amounts once a thread is closed", () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    const db = openReadonly(fixture);
    const thread = db.threadByKeytag(KEYTAG);
    if (!thread.isOk() || !thread.value) throw new Error("expected a thread");
    // A closed thread has no continuation output, so the store legitimately
    // cannot report an amount; demanding one would be a false positive.
    expect(
      assertDbMatchesCli(thread.value, { bufferedLovelace: 91_000_000 }).isOk(),
    ).toBe(true);
    db.close();
  });
});

// ---------------------------------------------------------------------------

describe("konduit-indexer db :: waiting", () => {
  it("waits for a channel and returns the whole thread", async () => {
    const fixture = newFixtureDb();
    const db = openReadonly(fixture);
    const env = fakeEnv();

    // Not there yet.
    const before = await waitForIndexedChannel(db, env, KEYTAG, CHANNEL_TX);
    expect(before.isErr()).toBe(true);
    if (before.isErr()) expect(before.error.reason).toContain("no channel row");

    seedFullThread(fixture);
    const after = await waitForIndexedChannel(db, env, KEYTAG, CHANNEL_TX);
    expect(after.isOk()).toBe(true);
    if (after.isOk()) expect(after.value.steps).toHaveLength(3);
    db.close();
  });

  it("does not accept a channel created by a different transaction", async () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    const db = openReadonly(fixture);
    const wrong = await waitForIndexedChannel(
      db,
      fakeEnv({ timeoutMs: 100 }),
      KEYTAG,
      txId(0xee),
    );
    expect(wrong.isErr()).toBe(true);
    if (wrong.isErr()) {
      // The description names the channel we *did* find, so the failure is
      // actionable rather than just "timed out".
      expect(wrong.error.reason).toContain(CHANNEL_TX.slice(0, 12));
    }
    db.close();
  });

  it("waits for the expected number of steps with the expected redeemer", async () => {
    const fixture = newFixtureDb();
    const db = openReadonly(fixture);
    const env = fakeEnv();

    // A thread that does not exist yet cannot produce steps.
    const nothing = await waitForIndexedSteps(
      db,
      fakeEnv({ timeoutMs: 100 }),
      { channel: { transactionIdHex: CHANNEL_TX, outputIndex: 0 } } as never,
      { expectedStepCount: 1 },
    );
    expect(nothing.isErr()).toBe(true);

    seedFullThread(fixture);
    const thread = db.threadByKeytag(KEYTAG);
    if (!thread.isOk() || !thread.value) throw new Error("expected a thread");

    // Three steps: Add, Sub, Mutual(close).
    const steps = await waitForIndexedSteps(db, env, thread.value, {
      expectedStepCount: 3,
      expectedRedeemers: ["Mutual"],
    });
    expect(steps.isOk()).toBe(true);
    if (steps.isOk()) expect(steps.value).toHaveLength(3);

    // The n-th step is pinned, so asking for a `Sub` at position 3 fails: the
    // store's third step is the close, and a poll must not succeed on the
    // earlier `Sub`.
    const mismatched = await waitForIndexedSteps(
      db,
      fakeEnv({ timeoutMs: 100 }),
      thread.value,
      { expectedStepCount: 3, expectedRedeemers: ["Cont:Sub"] },
    );
    expect(mismatched.isErr()).toBe(true);

    db.close();
  });

  it("waits for a step identified by its transaction id", async () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    const db = openReadonly(fixture);
    const thread = db.threadByKeytag(KEYTAG);
    if (!thread.isOk() || !thread.value) throw new Error("expected a thread");
    const step = await waitForStepInTransaction(
      db,
      fakeEnv(),
      thread.value,
      ADD_TX,
      "Cont:Add",
    );
    expect(step.isOk()).toBe(true);
    if (step.isOk()) expect(step.value.outputIndex).toBe(1);
    db.close();
  });

  it("insists on exactly one channel per keytag", async () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    // A mimic: same keytag, different channel.
    insertChannel(fixture.db, {
      addVkeyHex: ADD_VKEY,
      subVkeyHex: SUB_VKEY,
      tagHex: FIXTURE_TAG_HEX,
      scriptHashHex: SCRIPT_HASH,
      transactionIdHex: txId(0x77),
      transactionIndex: 0,
      outputIndex: 0,
      blockSlotNo: SLOTS.add,
      lovelace: 20_000_000,
      datumHex: DATUM_OPENED,
    });
    const db = openReadonly(fixture);
    const one = await waitForExactlyOneChannel(
      db,
      fakeEnv(),
      KEYTAG,
      CHANNEL_TX,
    );
    expect(one.isErr()).toBe(true);
    if (one.isErr()) expect(one.error.reason).toContain("found 2");
    db.close();
  });
});

// ---------------------------------------------------------------------------

describe("konduit-indexer db :: config resolution", () => {
  it("skips the db layer cleanly when no indexer db is configured", async () => {
    const { db, skipReason } = await openIndexerDbFor({
      indexerDb: {
        dbPath: "/nonexistent/konduit-indexer.db",
        enabled: false,
        disabledReason: "KONDUIT_INDEXER_DB_CHECKS=0",
        timeoutMs: 1_000,
        pollMs: 10,
        busyTimeoutMs: 0,
      },
    });
    expect(db).toBeNull();
    expect(skipReason).toBe("KONDUIT_INDEXER_DB_CHECKS=0");
  });

  it("explains itself when the db is enabled but unreachable", async () => {
    const { db, skipReason } = await openIndexerDbFor({
      indexerDb: {
        dbPath: "/nonexistent/konduit-indexer.db",
        enabled: true,
        timeoutMs: 1_000,
        pollMs: 10,
        busyTimeoutMs: 0,
      },
    });
    expect(db).toBeNull();
    expect(skipReason).toContain("indexer db not found");
  });
});

// ---------------------------------------------------------------------------

describe("konduit-indexer db :: fixture hygiene", () => {
  it("reproduces the exact datum it was handed, byte for byte", () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    const db = openReadonly(fixture);
    const found = db.channelsByKeytag(KEYTAG);
    const channel = found.isOk() ? found.value[0] : undefined;
    if (!channel) throw new Error("expected a channel");
    // Guards the whole fixture suite: if this ever fails, the datums we insert
    // are not the fixtures we think they are.
    expect(Buffer.from(channel.datumBlob).toString("hex")).toBe(DATUM_OPENED);
    expect(
      Datum.uint8ArrayCodec.deserialise(channel.datumBlob)._unsafeUnwrap()
        .stage,
    ).toEqual({
      label: "Opened",
      subbed: 0n,
      useds: [],
    });
    db.close();
  });

  it("records a Cont::Close as a transition into the Closed stage", () => {
    // This is the shape `consumer tx --close` produces, and it is the one the
    // lifecycle scenario asserts on. `insert_step` turns a
    // `NonMutual::Cont(_)` into a *transition* (datum + output index), so a
    // NULL output index here would mean the indexer misread the redeemer.
    const fixture = newFixtureDb();
    insertChannel(fixture.db, {
      addVkeyHex: ADD_VKEY,
      subVkeyHex: SUB_VKEY,
      tagHex: FIXTURE_TAG_HEX,
      scriptHashHex: SCRIPT_HASH,
      transactionIdHex: CHANNEL_TX,
      transactionIndex: 0,
      outputIndex: 0,
      blockSlotNo: SLOTS.channel,
      lovelace: 20_000_000,
      datumHex: DATUM_OPENED,
    });
    insertStep(fixture.db, {
      channelTransactionIdHex: CHANNEL_TX,
      channelOutputIndex: 0,
      transactionIdHex: txId(0x08),
      transactionIndex: 0,
      blockSlotNo: SLOTS.add,
      outputIndex: 1,
      lovelace: 20_000_000,
      datumHex: DATUM_CLOSED,
      redeemerHex: REDEEMER_CLOSE,
    });
    const db = openReadonly(fixture);
    const thread = db.threadByKeytag(KEYTAG);
    if (!thread.isOk() || !thread.value) throw new Error("expected a thread");
    const step = thread.value.steps[0]!;
    expect(step.redeemerSteps).toEqual(["Cont:Close"]);
    expect(step.outputIndex).toBe(1);
    expect(step.datum?.stage.label).toBe("Closed");

    // The thread is *not* terminated (the UTxO still lives during the close
    // period) but the channel *is* closed.
    expect(thread.value.terminated).toBe(false);
    expect(isChannelClosed(thread.value)).toBe(true);
    expect(stageLabel(thread.value.currentStage)).toBe("Closed");

    const shape = assertClosingStepShape(step, "closed-stage");
    expect(
      shape.isOk(),
      shape.isErr() ? formatViolations(shape.error) : "",
    ).toBe(true);
    db.close();
  });

  it("rejects a Cont::Close row that wrongly looks like a terminating step", () => {
    const fixture = newFixtureDb();
    insertChannel(fixture.db, {
      addVkeyHex: ADD_VKEY,
      subVkeyHex: SUB_VKEY,
      tagHex: FIXTURE_TAG_HEX,
      scriptHashHex: SCRIPT_HASH,
      transactionIdHex: CHANNEL_TX,
      transactionIndex: 0,
      outputIndex: 0,
      blockSlotNo: SLOTS.channel,
      lovelace: 20_000_000,
      datumHex: DATUM_OPENED,
    });
    insertStep(fixture.db, {
      channelTransactionIdHex: CHANNEL_TX,
      channelOutputIndex: 0,
      transactionIdHex: txId(0x08),
      transactionIndex: 0,
      blockSlotNo: SLOTS.add,
      outputIndex: null,
      redeemerHex: REDEEMER_CLOSE,
    });
    const db = openReadonly(fixture);
    const steps = db.stepsByChannel({
      transactionIdHex: CHANNEL_TX,
      outputIndex: 0,
    });
    const step = steps.isOk() ? steps.value[0] : undefined;
    if (!step) throw new Error("expected a step");
    const shape = assertClosingStepShape(step, "closed-stage");
    expect(shape.isErr()).toBe(true);
    if (shape.isErr()) {
      expect(shape.error.map((v) => v.invariant)).toContain(
        "a Cont::Close keeps its continuation output",
      );
    }
    db.close();
  });

  it("accepts a mutual close as a terminating step", () => {
    const fixture = newFixtureDb();
    seedFullThread(fixture);
    const db = openReadonly(fixture);
    const steps = db.stepsByChannel({
      transactionIdHex: CHANNEL_TX,
      outputIndex: 0,
    });
    const step = steps.isOk() ? steps.value[2] : undefined;
    if (!step) throw new Error("expected a step");
    expect(assertClosingStepShape(step, "terminated").isOk()).toBe(true);
    // The same row does *not* satisfy the transition shape.
    expect(assertClosingStepShape(step, "closed-stage").isErr()).toBe(true);
    db.close();
  });
});

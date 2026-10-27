import { afterAll, beforeAll, describe, expect, it } from "vitest";
import {
  loadTestEnv,
  pingProcessCompose,
  type KonduitTestEnv,
} from "../../src/testEnv.js";
import {
  type PollFailure,
  describeSteps,
  describeThread,
  openIndexerDbFor,
  verifyIndexerDbSchema,
  waitForIndexedTipAfter,
} from "../../src/scenarios/indexing.js";
import { type IndexerDb, assembleThread } from "../../src/indexerDb.js";
import * as hexString from "@konduit/codec/hexString";

/**
 * @file Live checks against the `konduit-indexer` SQLite store.
 *
 * `test/indexer-db.test.ts` proves the db layer works against a fixture; this
 * file proves it works against the *real* store the dev-env is writing, and
 * that the store is actually being fed.
 *
 * It deliberately does **not** submit any transaction: the e2e scenarios in
 * `test/e2e/*.test.ts` already drive the chain, and re-using the same wallets
 * here would make the two fight over the same UTxOs (see the `fileParallelism:
 * false` note in `vitest.config.ts`). What we can check without submitting
 * anything is:
 *
 *  1. the db is reachable, migrated and non-empty,
 *  2. its tip advances — i.e. the scheduled sync pass really is running,
 *  3. every channel and step already in the store satisfies the structural
 *     invariants, so a bad row introduced earlier cannot hide until a scenario
 *     happens to trip over it,
 *  4. every blob in the store decodes.
 *
 * (4) and (3) are the checks that scale: they run over *every* channel the
 * indexer has ever seen, so their value grows with the store.
 *
 * Everything skips cleanly when the dev-env is not up.
 */

describe("konduit-indexer db :: live store", () => {
  const daemonAlive = pingProcessCompose();
  if (!daemonAlive) {
    it.skip("process-compose daemon is not reachable on port 8100", () => {});
    return;
  }

  let env: KonduitTestEnv;
  let db: IndexerDb | null = null;
  let skipReason: string | undefined;

  beforeAll(async () => {
    env = loadTestEnv();
    const opened = await openIndexerDbFor(env);
    db = opened.db;
    skipReason = opened.skipReason;
    if (db) {
      const verified = verifyIndexerDbSchema(db);
      expect(verified.isOk(), verified.isErr() ? verified.error : "").toBe(
        true,
      );
    }
  }, 30_000);

  afterAll(() => {
    // The handle is cached and reused across suites; leaving it open is fine
    // (the process is about to exit), but be explicit about it.
    void db;
  });

  it("has a migrated db with a non-empty block table", () => {
    if (!db) {
      expect(skipReason).toBeTruthy();
      return;
    }
    const tip = db.tip();
    expect(tip.isOk(), tip.isErr() ? tip.error : "").toBe(true);
    if (tip.isOk()) {
      expect(
        tip.value,
        "the konduit-indexer has not indexed a single block",
      ).not.toBeNull();
    }
  }, 30_000);

  it(
    "keeps advancing its tip",
    async () => {
      if (!db) return;
      const before = db.tip();
      expect(before.isOk()).toBe(true);
      if (!before.isOk() || !before.value) return;
      // The indexer is scheduled every 10s
      // (`schedule = { interval = "10s" }` in flake/process-compose/dev-env.nix),
      // so waiting a couple of sync intervals is enough to see movement.
      const advanced = await waitForIndexedTipAfter(
        db,
        before.value.slotNo + 1,
        env.indexerDb.timeoutMs,
        env.indexerDb.pollMs,
      );
      expect(
        advanced.isOk(),
        advanced.isErr()
          ? `the indexer tip never advanced past slot ${before.value.slotNo}: ${describeFailure(advanced.error)}`
          : "",
      ).toBe(true);
    },
    3 * 60 * 1000,
  );

  it("holds no structurally invalid channel", () => {
    if (!db) return;
    const channels = db.channels(500);
    expect(channels.isOk(), channels.isErr() ? channels.error : "").toBe(true);
    if (channels.isErr()) return;
    const invalid = channels.value.filter(
      (c) => c.datumError !== null || c.datum === null,
    );
    expect(
      invalid.map(
        (c) =>
          `${c.transactionIdHex}#${c.outputIndex}: ${c.datumError ?? "<null datum>"}`,
      ),
      "every indexed channel must carry a decodable datum",
    ).toEqual([]);
  }, 60_000);

  it("holds no structurally invalid step", () => {
    if (!db) return;
    const steps = db.steps(2000);
    expect(steps.isOk(), steps.isErr() ? steps.error : "").toBe(true);
    if (steps.isErr()) return;
    const brokenDatum = steps.value.filter(
      (s) => s.datumBlob !== null && s.datum === null,
    );
    const brokenRedeemer = steps.value.filter(
      (s) => s.redeemerBlob !== null && s.redeemer === null,
    );
    expect(
      brokenDatum.map((s) => `${s.transactionIdHex}: ${s.datumError}`),
      "every indexed step datum must decode",
    ).toEqual([]);
    expect(
      brokenRedeemer.map((s) => `${s.transactionIdHex}: ${s.redeemerError}`),
      "every indexed step redeemer must decode",
    ).toEqual([]);
  }, 60_000);

  it("agrees with its own datums about every keytag", () => {
    if (!db) return;
    const channels = db.channels(500);
    if (channels.isErr()) return;
    const mismatches = channels.value
      .filter((c) => c.datum !== null)
      .flatMap((c) => {
        const constants = c.datum!.constants;
        const problems: string[] = [];
        if (c.tagHex !== hexString.fromUint8Array(constants.tag)) {
          problems.push(
            `tag ${c.tagHex} vs datum ${hexString.fromUint8Array(constants.tag)}`,
          );
        }
        if (c.addVkeyHex !== hexString.fromUint8Array(constants.addVkey.key)) {
          problems.push(
            `add_vkey ${c.addVkeyHex} vs datum ${hexString.fromUint8Array(constants.addVkey.key)}`,
          );
        }
        if (c.subVkeyHex !== hexString.fromUint8Array(constants.subVkey.key)) {
          problems.push(
            `sub_vkey ${c.subVkeyHex} vs datum ${hexString.fromUint8Array(constants.subVkey.key)}`,
          );
        }
        return problems.map(
          (p) => `${c.transactionIdHex}#${c.outputIndex}: ${p}`,
        );
      });
    expect(mismatches).toEqual([]);
  }, 60_000);

  it("holds channels whose datum says nothing was subbed yet", () => {
    // A freshly opened channel's datum is `Stage::Opened(0, [])`. A non-zero
    // `subbed` on a channel that has no sub step would mean the indexer paired
    // a step with the wrong channel — the exact mis-attribution the
    // `step` -> `channel` foreign key cannot catch.
    if (!db) return;
    const channels = db.channels(500);
    if (channels.isErr()) return;
    const suspicious = channels.value
      .filter((c) => {
        const stage = c.datum?.stage;
        return (
          stage !== undefined && stage.label === "Opened" && stage.subbed > 0n
        );
      })
      .map(
        (c) =>
          `${c.transactionIdHex}#${c.outputIndex}@slot${c.blockSlotNo}: subbed=${c.datum!.stage.label === "Opened" ? c.datum!.stage.subbed : "?"}`,
      );
    expect(
      suspicious,
      "a channel datum with subbed > 0 but no step rows would be a mis-attributed thread",
    ).toEqual([]);
  }, 60_000);

  it("only holds channels that carry the konduit validator", () => {
    if (!db) return;
    if (!env.validatorHashHex) {
      console.log(
        "[db-live] KONDUIT_VALIDATOR_HASH is unset; skipping the script-hash check",
      );
      return;
    }
    const channels = db.channels(500);
    if (channels.isErr()) return;
    const wrong = channels.value
      .filter(
        (c) =>
          c.scriptHashHex.toLowerCase() !== env.validatorHashHex!.toLowerCase(),
      )
      .map((c) => `${c.transactionIdHex}: ${c.scriptHashHex}`);
    expect(
      wrong,
      'kupo is started with --match "$KONDUIT_VALIDATOR_HASH/*", so nothing else ' +
        "should ever reach the store",
    ).toEqual([]);
  }, 60_000);

  it("prints a summary a human can eyeball", () => {
    if (!db) return;
    const counts = db.counts();
    const tip = db.tip();
    const channels = db.channels(3);
    const steps = db.steps(3);
    console.log(`[db-live] db     ${env.indexerDb.dbPath}`);
    console.log(
      `[db-live] counts ${counts.isOk() ? JSON.stringify(counts.value) : counts.error}`,
    );
    console.log(
      `[db-live] tip    ${tip.isOk() ? JSON.stringify(tip.value) : tip.error}`,
    );
    for (const channel of channels.isOk() ? channels.value : []) {
      const threadSteps = db.stepsByChannel({
        transactionIdHex: channel.transactionIdHex,
        outputIndex: channel.outputIndex,
      });
      console.log(
        `[db-live] thread ${describeThread(assembleThread(channel, threadSteps.isOk() ? threadSteps.value : []))}`,
      );
    }
    console.log(
      `[db-live] steps  ${steps.isOk() ? describeSteps(steps.value) : steps.error}`,
    );
    expect(counts.isOk()).toBe(true);
  }, 60_000);
});

const describeFailure = (failure: PollFailure): string =>
  `${failure.reason} (${failure.attempts} polls / ${failure.elapsedMs}ms)`;

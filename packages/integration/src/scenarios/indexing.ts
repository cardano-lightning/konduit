import * as uint8Array from "@konduit/codec/uint8Array";
import { err, ok, Result } from "neverthrow";
import type {
  ChannelRow,
  ChannelThread,
  IndexerDb,
  Keytag,
  StepRow,
} from "../indexerDb.js";
import { openIndexerDb } from "../indexerDb.js";
import {
  type Stage,
  RespondedStage,
  Stage as StageCodec,
} from "../konduit/datum.js";
import * as hexString from "@konduit/codec/hexString";
import type { KonduitTestEnv } from "../testEnv.js";
import { sleep } from "./shared.js";

/**
 * @file The db-check layer.
 *
 * The on-chain assertions the scenarios already make (`consumer show tip`)
 * prove the *validator* accepted something. They say nothing about whether the
 * `konduit-indexer` — the process that actually persists channel state for
 * everybody downstream — picked it up. This module closes that gap.
 *
 * It is deliberately shaped like the Postgres counterpart in
 * `marlowe-integration/app/src/marloweIndexer.ts`: `waitFor*` helpers that poll
 * until a predicate holds, returning domain objects rather than booleans. The
 * e2e scenario then reads:
 *
 * ```ts
 * const { db, skipReason } = await openIndexerDbFor(env);
 * if (db) {
 *   const thread = await waitForIndexedChannel(db, env, keytag, txId);
 *   assertChannelMatchesChain(thread.channel, {...});
 *   const steps = await waitForIndexedStep(db, env, thread, addTxId, ["Cont:Add"]);
 *   assertStageAmount(steps[steps.length - 1]!, expectedAmount);
 * }
 * ```
 *
 * ## Why the polling is not racy
 *
 * The indexer runs a one-shot sync pass every 10s
 * (`schedule = { interval = "10s" }` in `flake/process-compose/dev-env.nix`) and
 * only then commits, so a row can appear *after* the tx is definitely on chain.
 * Every helper therefore takes a timeout comfortably larger than one sync
 * interval and reports what it last saw, so a timeout says "the indexer never
 * inserted X" instead of failing with a bare assertion error.
 */

/** The subset of {@link KonduitTestEnv} these helpers need. */
export type IndexerDbEnv = Pick<
  KonduitTestEnv,
  "indexerDb" | "validatorHashHex"
>;

/**
 * Open the indexer store, or explain why it is not available.
 *
 * Scenarios call this and branch on `null` rather than failing, so the e2e
 * suite still runs on a machine where the indexer process is not part of the
 * dev-env.
 */
export const openIndexerDbFor = async (
  env: IndexerDbEnv,
): Promise<{ db: IndexerDb | null; skipReason?: string }> => {
  if (!env.indexerDb.enabled) {
    return { db: null, skipReason: env.indexerDb.disabledReason ?? "disabled" };
  }
  const opened = openIndexerDb({
    dbPath: env.indexerDb.dbPath,
    schemaDir: env.indexerDb.schemaDir,
    busyTimeoutMs: env.indexerDb.busyTimeoutMs,
    debug: env.indexerDb.debug,
  });
  if (opened.isErr()) return { db: null, skipReason: opened.error };
  return { db: opened.value };
};

// ---------------------------------------------------------------------------
// Low-level polling
// ---------------------------------------------------------------------------

export type PollFailure = {
  reason: string;
  attempts: number;
  elapsedMs: number;
};

/**
 * Poll `probe` until `matches` holds.
 *
 * A failing probe (a `SQLITE_BUSY` blip, say) is treated as "not yet" and the
 * last description — error text *or* the rendered value — is what ends up in
 * the failure, so a timeout always explains itself.
 */
const pollUntil = async <T>(
  probe: () => Result<T, string>,
  matches: (value: T) => boolean,
  describe: (value: T) => string,
  timeoutMs: number,
  pollMs: number,
): Promise<Result<T, PollFailure>> => {
  const start = Date.now();
  let attempts = 0;
  let lastDescription = "no attempt completed";
  while (Date.now() - start < timeoutMs) {
    attempts += 1;
    const probeResult = probe();
    if (probeResult.isOk()) {
      lastDescription = describe(probeResult.value);
      if (matches(probeResult.value)) return ok(probeResult.value);
    } else {
      lastDescription = probeResult.error;
    }
    await sleep(pollMs);
  }
  return err({
    reason: lastDescription,
    attempts,
    elapsedMs: Date.now() - start,
  });
};

/**
 * {@link pollUntil} for probes that legitimately return `null` ("not indexed
 * yet"). A `null` is never a success on its own — the caller still has to say
 * what it was waiting for.
 */
const pollUntilDefined = async <T>(
  probe: () => Result<T | null, string>,
  matches: (value: T) => boolean,
  describe: (value: T | null) => string,
  timeoutMs: number,
  pollMs: number,
): Promise<Result<T, PollFailure>> => {
  const boxed = await pollUntil(
    () => probe().map((value) => ({ value })),
    (box) => box.value !== null && matches(box.value),
    (box) => describe(box.value),
    timeoutMs,
    pollMs,
  );
  return boxed.andThen(
    (box): Result<T, PollFailure> =>
      box.value === null
        ? err({ reason: "probe returned null", attempts: 1, elapsedMs: 0 })
        : ok(box.value as T),
  );
};

// ---------------------------------------------------------------------------
// Schema / liveness
// ---------------------------------------------------------------------------

/** Tables 001.sql must create. Missing any of them means the db is not ours. */
export const REQUIRED_TABLES = ["block", "channel", "focus", "step"] as const;

/**
 * The migration revision the store claims to be at.
 *
 * `konduit_indexer::store::sqlite::migrations::MIGRATIONS` currently holds a
 * single revision, so `user_version == 1` after a successful migration.
 */
export const EXPECTED_SCHEMA_VERSION = 1;

/**
 * Verify the store is present and migrated before trusting anything else.
 *
 * Worth running once per suite: every other assertion in this module assumes
 * these tables exist, and a schema drift would otherwise surface as a baffling
 * "no such column" deep inside a channel query.
 */
export const verifyIndexerDbSchema = (
  db: IndexerDb,
): Result<{ schemaVersion: number }, string> => {
  const version = db.schemaVersion();
  if (version.isErr()) return err(version.error);
  if (version.value !== EXPECTED_SCHEMA_VERSION) {
    return err(
      `konduit-indexer db is at schema revision ${version.value}, ` +
        `expected ${EXPECTED_SCHEMA_VERSION} — the running indexer binary and ` +
        `packages/konduit/indexer/sqlite are out of sync`,
    );
  }
  const tables = db.tables();
  if (tables.isErr()) return err(tables.error);
  const missing = REQUIRED_TABLES.filter((t) => !tables.value.includes(t));
  if (missing.length > 0) {
    return err(`konduit-indexer db is missing table(s): ${missing.join(", ")}`);
  }
  return ok({ schemaVersion: version.value });
};

/**
 * Wait for the store's tip to move past `slotNo`.
 *
 * This is the liveness gate for every other check: without it a timeout on a
 * channel wait is ambiguous between "the indexer is wedged" and "the channel
 * genuinely was not indexed", and the two need very different fixes.
 */
export const waitForIndexedTipAfter = async (
  db: IndexerDb,
  slotNo: number,
  timeoutMs: number,
  pollMs: number,
): Promise<Result<{ slotNo: number; blockNo: number }, PollFailure>> =>
  pollUntilDefined(
    () => db.tip(),
    (tip) => tip.slotNo >= slotNo,
    (tip) =>
      tip === null
        ? "the block table is empty"
        : `tip is at slot ${tip.slotNo} (block ${tip.blockNo})`,
    timeoutMs,
    pollMs,
  );

// ---------------------------------------------------------------------------
// Channels
// ---------------------------------------------------------------------------

const describeChannels = (rows: ChannelRow[]): string => {
  if (rows.length === 0) return "no channel row for this keytag yet";
  return rows
    .map(
      (c) =>
        `${c.transactionIdHex.slice(0, 12)}#${c.outputIndex}@slot${c.blockSlotNo} ` +
        `lovelace=${c.lovelace} datum=${describeDatum(c)}`,
    )
    .join(" | ");
};

export const describeDatum = (row: ChannelRow | StepRow): string => {
  if (row.datumError) return `<undecodable: ${row.datumError}>`;
  if (!row.datum) return "<null>";
  const stage = row.datum.stage;
  // `Responded` is the one variant without a `subbed`; it reports the total it
  // is holding in pendings instead.
  const detail =
    stage.label === RespondedStage.LABEL
      ? `pendings=${stage.pendsAmount}`
      : `subbed=${stage.subbed}`;
  return `${stage.label}(${detail})`;
};

/**
 * Wait until the indexer has inserted the channel created by `openTxId`.
 *
 * `openTxId` is required on purpose. `001.sql` says so itself: a channel is
 * "uniquely identified by the initial output on the chain", and the
 * `(add_vkey, tag)` pair is explicitly *not* enforced unique ("we can expect
 * for example malicious mimics"). Keying the wait on the opening transaction
 * is therefore the only way to be sure we latch onto *our* channel and not a
 * stale one left behind by a previous run on the same tag.
 */
export const waitForIndexedChannel = async (
  db: IndexerDb,
  env: IndexerDbEnv,
  keytag: Keytag,
  openTxId: string,
): Promise<Result<ChannelThread, PollFailure>> =>
  pollUntilDefined(
    () => db.threadByKeytag(keytag),
    (thread) => thread.channel.transactionIdHex === openTxId,
    (thread) =>
      thread === null
        ? `no channel row for add_vkey=${keytag.addVkeyHex.slice(0, 8)}… tag=${keytag.tagHex}` +
          ` (waiting for tx ${openTxId.slice(0, 12)}…)`
        : describeChannels([thread.channel]),
    env.indexerDb.timeoutMs,
    env.indexerDb.pollMs,
  );

/**
 * Every channel the indexer knows for a keytag.
 *
 * `001.sql` deliberately does not enforce `(add_vkey, tag)` uniqueness on chain
 * (a malicious "mimic" is possible), so this is a list and asserting on its
 * length is part of the point.
 */
export const waitForExactlyOneChannel = async (
  db: IndexerDb,
  env: IndexerDbEnv,
  keytag: Keytag,
  openTxId: string,
): Promise<Result<ChannelRow, PollFailure>> => {
  const found = await waitForIndexedChannel(db, env, keytag, openTxId);
  if (found.isErr()) return err(found.error);
  const all = db.channelsByKeytag(keytag);
  if (all.isErr()) return err({ reason: all.error, attempts: 1, elapsedMs: 0 });
  if (all.value.length !== 1) {
    return err({
      reason: `expected exactly 1 channel for the keytag, found ${all.value.length}: ${describeChannels(all.value)}`,
      attempts: 1,
      elapsedMs: 0,
    });
  }
  return ok(all.value[0]!);
};

// ---------------------------------------------------------------------------
// Steps
// ---------------------------------------------------------------------------

export const describeSteps = (steps: StepRow[]): string => {
  if (steps.length === 0) return "no step rows for this channel yet";
  return steps
    .map(
      (s) =>
        `${s.transactionIdHex.slice(0, 12)}@slot${s.blockSlotNo} ` +
        `${s.redeemerSteps.length > 0 ? s.redeemerSteps.join("+") : "<undecodable redeemer>"}` +
        `${s.outputIndex === null ? " [close]" : ` -> #${s.outputIndex} lovelace=${s.lovelace}`}`,
    )
    .join(" | ");
};

/**
 * Wait until the indexer has inserted at least `expectedStepCount` steps for
 * the channel, *and* the last of them carries one of `expectedRedeemers`.
 *
 * `expectedRedeemers` uses the compact `Redeemer.labels` projection
 * (`"Cont:Add"`, `"Cont:Sub"`, `"Eol:End"`, …), because that is the assertion
 * that actually has teeth: it proves the indexer read the *redeemer*, not just
 * that it saw a matching output.
 */
/**
 * Wait until the indexer has inserted at least `expectedStepCount` steps for
 * the channel, and the step at position `expectedStepCount - 1` carries one of
 * `expectedRedeemers`.
 *
 * `expectedRedeemers` uses the compact `Redeemer.labels` projection
 * (`"Cont:Add"`, `"Cont:Sub"`, `"Eol:End"`, …), because that is the assertion
 * with teeth: it proves the indexer read the *redeemer*, not merely that it saw
 * a matching output.
 *
 * Pinning the *n*-th step rather than "somewhere in the list" is what keeps a
 * poll from succeeding on a stale earlier step: after `open → add` the list has
 * two entries and step 1 is the `add`, so a run that already indexed an older
 * `add` cannot make a later wait succeed early.
 */
export const waitForIndexedSteps = async (
  db: IndexerDb,
  env: IndexerDbEnv,
  thread: ChannelThread,
  options: {
    expectedStepCount: number;
    expectedRedeemers?: string[];
  },
): Promise<Result<StepRow[], PollFailure>> => {
  if (options.expectedStepCount < 1) {
    return err({
      reason: `expectedStepCount must be >= 1, got ${options.expectedStepCount}`,
      attempts: 0,
      elapsedMs: 0,
    });
  }
  return pollUntil(
    () => db.stepsByChannel(channelIdOf(thread)),
    (steps) => {
      if (steps.length < options.expectedStepCount) return false;
      const expected = options.expectedRedeemers;
      if (!expected || expected.length === 0) return true;
      const target = steps[options.expectedStepCount - 1]!;
      return target.redeemerSteps.some((k) => expected.includes(k));
    },
    describeSteps,
    env.indexerDb.timeoutMs,
    env.indexerDb.pollMs,
  );
};

/** The `(transaction_id, output_index)` pair that identifies a channel row. */
export const channelIdOf = (
  thread: ChannelThread,
): { transactionIdHex: string; outputIndex: number } => ({
  transactionIdHex: thread.channel.transactionIdHex,
  outputIndex: thread.channel.outputIndex,
});

/**
 * Wait until the channel for `keytag` holds at least `expectedStepCount` steps.
 *
 * The by-keytag counterpart of {@link waitForIndexedSteps}: use it when you have
 * just been handed a step (so you already know the channel exists) and only
 * want the whole thread refreshed.
 */
export const waitForThreadSteps = async (
  db: IndexerDb,
  env: IndexerDbEnv,
  keytag: Keytag,
  expectedStepCount: number,
): Promise<Result<ChannelThread, PollFailure>> =>
  pollUntilDefined(
    () => db.threadByKeytag(keytag),
    (thread) => thread.steps.length >= expectedStepCount,
    (thread) =>
      thread === null
        ? `no channel row for add_vkey=${keytag.addVkeyHex.slice(0, 8)}… tag=${keytag.tagHex}`
        : `${describeThread(thread)} (${thread.steps.length} step(s), want ${expectedStepCount})`,
    env.indexerDb.timeoutMs,
    env.indexerDb.pollMs,
  );

/**
 * Wait until the store holds the step produced by `txId`.
 *
 * `expectedRedeemer`, when given, must match that step's own redeemer — so a
 * transaction that landed in the store under the wrong step would be reported
 * rather than accepted.
 */
export const waitForStepInTransaction = async (
  db: IndexerDb,
  env: IndexerDbEnv,
  thread: ChannelThread,
  txId: string,
  expectedRedeemer?: string,
): Promise<Result<StepRow, PollFailure>> => {
  const polled = await pollUntil(
    () => db.stepsByChannel(channelIdOf(thread)),
    (steps) => {
      const match = steps.find((s) => s.transactionIdHex === txId);
      if (!match) return false;
      if (expectedRedeemer === undefined) return true;
      return match.redeemerSteps.includes(expectedRedeemer);
    },
    (steps) => describeSteps(steps),
    env.indexerDb.timeoutMs,
    env.indexerDb.pollMs,
  );
  if (polled.isErr()) return err(polled.error);
  const match = polled.value.find((s) => s.transactionIdHex === txId);
  if (!match) {
    return err({
      reason: `step for tx ${txId} not indexed; have ${describeSteps(polled.value)}`,
      attempts: 1,
      elapsedMs: 0,
    });
  }
  return ok(match);
};

// ---------------------------------------------------------------------------
// Invariants
// ---------------------------------------------------------------------------

export type InvariantViolation = { invariant: string; detail: string };

export type InvariantResult = Result<void, InvariantViolation[]>;

/**
 * Structural invariants that must hold for *any* indexed thread, whatever the
 * steps are.
 *
 * These are all enforced by triggers and foreign keys inside SQLite, so on a
 * healthy store they cannot fail. They are worth asserting anyway: they pin
 * down the schema's guarantees from the outside, and a violation means either
 * the store is corrupt or a migration is not what we think it is — both of
 * which should fail loudly rather than quietly weaken every later assertion.
 */
export const assertThreadInvariants = (
  thread: ChannelThread,
): InvariantResult => {
  const violations: InvariantViolation[] = [];
  const { channel, steps } = thread;

  const push = (invariant: string, detail: string) =>
    violations.push({ invariant, detail });

  // 1. Every step must point back at *this* channel. A mismatch means the
  //    indexer attributed a transition to the wrong thread.
  for (const step of steps) {
    if (
      step.channelTransactionIdHex !== channel.transactionIdHex ||
      step.channelOutputIndex !== channel.outputIndex
    ) {
      push(
        "step points at its channel",
        `step ${step.transactionIdHex.slice(0, 12)} references ` +
          `${step.channelTransactionIdHex.slice(0, 12)}#${step.channelOutputIndex}, ` +
          `but belongs to ${channel.transactionIdHex.slice(0, 12)}#${channel.outputIndex}`,
      );
    }
  }

  // 2. Steps must be in chain order: (block_slot_no, transaction_index,
  //    output_index) is the indexer's own ordering, and a thread is a linked
  //    list so reversing it would silently reorder the history.
  for (let i = 1; i < steps.length; i++) {
    const previous = steps[i - 1]!;
    const current = steps[i]!;
    if (compareStepOrder(previous, current) >= 0) {
      push(
        "steps are in chain order",
        `step ${i} (${current.transactionIdHex.slice(0, 12)}@${current.blockSlotNo}) ` +
          `does not sort after step ${i - 1} (${previous.transactionIdHex.slice(0, 12)}@${previous.blockSlotNo})`,
      );
    }
  }

  // 3. `001.sql`'s `no_step_after_close` trigger forbids anything past a
  //    closing step. Reproduce it so a regression in the indexer's own
  //    transaction ordering cannot slip through unnoticed.
  const closeAt = steps.findIndex((s) => s.outputIndex === null);
  if (closeAt >= 0 && closeAt !== steps.length - 1) {
    push(
      "no step after a close",
      `the closing step is at position ${closeAt} of ${steps.length}`,
    );
  }
  if (steps.filter((s) => s.outputIndex === null).length > 1) {
    push("at most one close", "more than one step has a NULL output_index");
  }

  // 4. `001.sql`'s step CHECK: a closing step has NULL datum/lovelace/
  //    output_index; a continuation step has all three set.
  for (const step of steps) {
    const closing = step.outputIndex === null;
    const continuation = step.lovelace !== null && step.outputIndex !== null;
    if (closing) {
      if (step.lovelace !== null || step.datumBlob !== null) {
        push(
          "closing steps carry no continuation output",
          `step ${step.transactionIdHex.slice(0, 12)} has output_index NULL but ` +
            `lovelace=${step.lovelace} datum=${describeDatum(step)}`,
        );
      }
    } else if (!continuation) {
      push(
        "continuation steps carry lovelace and an output index",
        `step ${step.transactionIdHex.slice(0, 12)} has output_index=${step.outputIndex} ` +
          `but lovelace=${step.lovelace}`,
      );
    }
  }

  // 5. Every blob must decode. A blob we cannot read is a bug in our decoder
  //    *or* a corrupt store; either way it must not pass silently.
  if (channel.datumError) {
    push("channel datum decodes", channel.datumError);
  }
  for (const step of steps) {
    if (step.datumError) {
      push(
        "step datum decodes",
        `step ${step.transactionIdHex.slice(0, 12)}: ${step.datumError}`,
      );
    }
    if (step.redeemerError) {
      push(
        "step redeemer decodes",
        `step ${step.transactionIdHex.slice(0, 12)}: ${step.redeemerError}`,
      );
    }
  }

  // 6. The constants are immutable across the thread — they are baked into the
  //    validator and repeated in every datum. A change mid-thread would mean the
  //    validator accepted a different channel.
  const reference = channel.datum?.constants;
  for (const step of steps) {
    const constants = step.datum?.constants;
    if (!constants || !reference) continue;
    if (
      !uint8Array.equal(constants.tag, reference.tag) ||
      !uint8Array.equal(constants.addVkey.key, reference.addVkey.key) ||
      !uint8Array.equal(constants.subVkey.key, reference.subVkey.key) ||
      constants.closePeriod !== reference.closePeriod
    ) {
      push(
        "constants are immutable across a thread",
        `step ${step.transactionIdHex.slice(0, 12)} carries different constants than the channel`,
      );
    }
  }

  // 7. The thread's identity must match the keytag it was looked up by, and
  //    the columns must agree with the datum's own copy of the same facts. The
  //    columns are stored as hex text, the datum as bytes — hence the compare.
  if (reference) {
    const check = (
      invariant: string,
      columnHex: string,
      datumBytes: Uint8Array,
    ) => {
      const expected = hexString.fromUint8Array(datumBytes);
      if (columnHex !== expected) {
        push(invariant, `column ${columnHex} vs datum ${expected}`);
      }
    };
    check(
      "channel.tag matches its datum constants",
      channel.tagHex,
      reference.tag,
    );
    check(
      "channel.add_vkey matches its datum constants",
      channel.addVkeyHex,
      reference.addVkey.key,
    );
    check(
      "channel.sub_vkey matches its datum constants",
      channel.subVkeyHex,
      reference.subVkey.key,
    );
  }
  if (violations.length > 0) return err(violations);
  return ok(undefined);
};

const compareStepOrder = (a: StepRow, b: StepRow): number => {
  if (a.blockSlotNo !== b.blockSlotNo) return a.blockSlotNo - b.blockSlotNo;
  if (a.transactionIndex !== b.transactionIndex)
    return a.transactionIndex - b.transactionIndex;
  return (a.outputIndex ?? -1) - (b.outputIndex ?? -1);
};

/**
 * Cross-check one channel row against what the chain told us.
 *
 * `openTxId` / `expectedLovelace` come from `consumer tx --open`'s response and
 * the CLI's own `show tip`; comparing them against the store proves the
 * indexer persisted *this* channel rather than something else that happens to
 * share a keytag.
 */
export const assertChannelMatchesChain = (
  channel: ChannelRow,
  expected: {
    openTxId?: string;
    addVkeyHex?: string;
    subVkeyHex?: string;
    tagHex?: string;
    scriptHashHex?: string;
    /**
     * The channel UTxO's lovelace, buffer included — i.e. the CLI's
     * `buffered_lovelace`, *not* its `amount_lovelace`.
     */
    lovelace?: number;
  },
): InvariantResult => {
  const violations: InvariantViolation[] = [];
  const check = (invariant: string, actual: unknown, wanted: unknown) => {
    if (wanted === undefined) return;
    if (actual !== wanted) {
      violations.push({
        invariant,
        detail: `expected ${String(wanted)}, got ${String(actual)}`,
      });
    }
  };
  check(
    "channel.transaction_id is the open tx",
    channel.transactionIdHex,
    expected.openTxId,
  );
  check(
    "channel.add_vkey is the consumer vkey",
    channel.addVkeyHex,
    expected.addVkeyHex,
  );
  check(
    "channel.sub_vkey is the adaptor vkey",
    channel.subVkeyHex,
    expected.subVkeyHex,
  );
  check("channel.tag is the scenario tag", channel.tagHex, expected.tagHex);
  check(
    "channel.script_hash is the konduit validator",
    channel.scriptHashHex?.toLowerCase(),
    expected.scriptHashHex?.toLowerCase(),
  );
  // The channel's lovelace lives in the `lovelace` column — the datum's stage
  // only carries `subbed`. Note the CLI's `amount_lovelace` excludes the
  // validator's ADA buffer while the UTxO value includes it, so callers pin
  // `lovelace` with the *buffered* figure (`channel.lovelace` in
  // `consumer show tip`) rather than the amount.
  check(
    "channel.lovelace is the on-chain UTxO value",
    channel.lovelace,
    expected.lovelace,
  );
  if (violations.length > 0) return err(violations);
  return ok(undefined);
};

/**
 * Compare the store's current state for a channel against the CLI's view.
 *
 * The CLI and the indexer derive their numbers from different sources (direct
 * node queries vs Kupo matches), so a mismatch here is a real finding — it is
 * precisely the class of bug the db-check layer exists to surface.
 *
 * A thread that reached an end-of-life step legitimately has no amount to
 * compare, so the amount check is skipped rather than reported as a difference.
 */
export const assertDbMatchesCli = (
  thread: ChannelThread,
  cli: {
    /**
     * The CLI's `buffered_lovelace` — the value of the channel's UTxO.
     *
     * This is what the store's `lovelace` column holds: the UTxO value,
     * which includes the validator's ADA buffer. The CLI's `amount_lovelace`
     * is the *unbuffered* amount and must not be compared with it.
     */
    bufferedLovelace?: number;
    stageLabel?: string;
  },
): InvariantResult => {
  const violations: InvariantViolation[] = [];
  const current = thread.currentStage;
  const where =
    current === null
      ? "db stage=<terminated>"
      : current.label === "Responded"
        ? `db stage=Responded pendsAmount=${current.pendsAmount}`
        : `db stage=${current.label} subbed=${current.subbed}`;

  if (cli.stageLabel !== undefined && current) {
    if (current.label.toLowerCase() !== cli.stageLabel.toLowerCase()) {
      violations.push({
        invariant: "stage label agrees with the CLI",
        detail: `${where} vs cli ${cli.stageLabel}`,
      });
    }
  }
  if (cli.bufferedLovelace !== undefined && !thread.terminated) {
    const indexed = indexedAmount(thread);
    if (indexed !== cli.bufferedLovelace) {
      violations.push({
        invariant: "channel amount agrees with the CLI",
        detail: `${where} lovelace=${indexed} vs cli buffered_lovelace=${cli.bufferedLovelace}`,
      });
    }
  }
  if (violations.length > 0) return err(violations);
  return ok(undefined);
};

/**
 * Assert that a step really is the closing one.
 *
 * "Closing" is a claim about the *shape* of the row, not about the redeemer:
 * 001.sql's step `CHECK` forces a row with `output_index IS NULL` to have a NULL
 * datum and NULL lovelace, and `insert_step` only emits that shape for
 * `Redeemer::Mutual` and `Step::Eol`. So a `Cont::Close` row (which the
 * consumer's `tx --close` produces) must still carry a continuation output and a
 * `Closed` datum — seeing `output_index === null` there means the indexer
 * misread the redeemer.
 */
export const assertClosingStepShape = (
  step: StepRow,
  expected: "closed-stage" | "terminated",
): InvariantResult => {
  const violations: InvariantViolation[] = [];
  const isTerminated = step.outputIndex === null;

  if (expected === "terminated" && !isTerminated) {
    violations.push({
      invariant: "a mutual/EOL close is stored as a terminating step",
      detail: `step ${step.transactionIdHex.slice(0, 12)} has output_index=${step.outputIndex}`,
    });
  }
  if (expected === "closed-stage") {
    if (isTerminated) {
      violations.push({
        invariant: "a Cont::Close keeps its continuation output",
        detail:
          `step ${step.transactionIdHex.slice(0, 12)} has output_index=NULL, ` +
          `but Cont::Close transitions the datum to Closed while the UTxO survives`,
      });
    }
    if (step.datum?.stage.label !== "Closed") {
      violations.push({
        invariant: "a Cont::Close moves the datum to the Closed stage",
        detail: `datum stage is ${step.datum?.stage.label ?? "<undecodable>"}`,
      });
    }
  }
  if (violations.length > 0) return err(violations);
  return ok(undefined);
};

/**
 * The lovelace the store says is locked, in lovelace.
 *
 * This comes from the `lovelace` **column**, never from the datum: the datum's
 * stage carries `subbed`, not the channel amount (see {@link OpenedStage}).
 * For a thread the value is the one on its last continuation step, falling back
 * to the channel row when there are no steps yet.
 *
 * `null` when the thread reached an end-of-life step (there is no continuation
 * output to read) or when the row is missing.
 */
export const indexedAmount = (thread: ChannelThread): number | null => {
  const last = thread.steps[thread.steps.length - 1] ?? null;
  if (last === null) return thread.channel.lovelace;
  if (last.lovelace === null) return null;
  return last.lovelace;
};

/**
 * The running `subbed` total the *datum* records for the thread's live state.
 *
 * `null` once the stage is `Responded` (that variant folds everything into the
 * pendings) or after the thread terminated.
 */
export const indexedSubbed = (thread: ChannelThread): number | null => {
  const stage = thread.currentStage;
  if (!stage || stage.label === RespondedStage.LABEL) return null;
  return Number(stage.subbed);
};

/** The cheque indexes the live datum records as already redeemed. */
export const indexedUseds = (thread: ChannelThread): number[] => {
  const stage = thread.currentStage;
  if (!stage || stage.label === RespondedStage.LABEL) return [];
  return stage.useds.map((used) => Number(used.index));
};

/** A human-readable one-liner used in scenario logs and assertion messages. */
export const describeThread = (thread: ChannelThread): string => {
  const { channel, steps, terminated } = thread;
  const head = `${channel.transactionIdHex.slice(0, 12)}#${channel.outputIndex}@slot${channel.blockSlotNo}`;
  const tail = steps
    .map((s) => `${s.redeemerSteps.join("+") || "?"}`)
    .join(" -> ");
  const amount = indexedAmount(thread);
  const amountPart = amount === null ? "" : ` lovelace=${amount}`;
  return (
    `${head} ${describeDatum(channel)}${amountPart}` +
    `${tail ? ` -> ${tail}` : ""}${terminated ? " [terminated]" : ""}`
  );
};

/** Render violations the way a test failure should read. */
export const formatViolations = (violations: InvariantViolation[]): string =>
  violations.map((v) => `  ✗ ${v.invariant}: ${v.detail}`).join("\n");

/** `Datum` stage label, for assertions that only care about the label. */
export const stageLabel = (stage: Stage | null): string | null =>
  stage === null ? null : StageCodec.label(stage);

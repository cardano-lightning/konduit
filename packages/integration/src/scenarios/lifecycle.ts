import {
  type ConsumerTipOutput,
  consumerShowTipJson,
  consumerTxAdd,
  consumerTxClose,
  consumerTxOpen,
  describeError,
} from "../konduitCli.js";
import {
  unwrapOrPanic,
  unwrapOrPanicWith,
} from "@konduit/konduit-consumer/neverthrow";
import type { ChannelThread } from "../indexerDb.js";
import { assembleThread, isChannelClosed } from "../indexerDb.js";
import type { KonduitTestEnv } from "../testEnv.js";
import {
  type InvariantViolation,
  type PollFailure,
  assertChannelMatchesChain,
  assertClosingStepShape,
  assertDbMatchesCli,
  assertThreadInvariants,
  formatViolations,
  indexedAmount,
  openIndexerDbFor,
  verifyIndexerDbSchema,
  waitForIndexedChannel,
  waitForIndexedSteps,
  waitForIndexedTipAfter,
  waitForStepInTransaction,
  waitForThreadSteps,
} from "./indexing.js";
import * as hexString from "@konduit/codec/hexString";
import {
  ADA,
  EXEC_FEE_BUDGET_LOVELACE,
  MIN_ADA_BUFFER_LOVELACE,
  MIN_CHANGE_LOVELACE,
  bootstrap,
  ensureFunded,
  explainNodeRejection,
  findFreeTag,
  pollFailure as renderPollFailure,
  uniqueTagHex,
  waitFor,
} from "./shared.js";

/**
 * @file Scenario: the plain channel lifecycle, inspired by `./bin/e2e.sh`.
 *
 *   1. bootstrap (fund the wallets, deploy the reference script *only if* it
 *      is not already visible, resolve the adaptor constants),
 *   2. `consumer tx --open` on a unique tag,
 *   3. `consumer tx --add`,
 *   4. `consumer tx --close`.
 *
 * Every step is followed by an on-chain assertion made through
 * `consumer show tip`, **and** — when the `konduit-indexer` db is reachable —
 * by an assertion that the indexer actually persisted the corresponding row in
 * its SQLite store. The two answer different questions: `show tip` proves the
 * validator accepted the transaction, the db check proves the state is
 * *durable and queryable* for everything downstream of the indexer.
 *
 * Every step is resilient: funding and deployment are skipped when they are
 * not needed, a fresh tag is chosen per run so the scenario never collides with
 * channels left behind by earlier runs, and the db checks are skipped (with a
 * printed reason) when no indexer db is configured.
 */

const OPEN_AMOUNT_LOVELACE = 20n * ADA;
const ADD_AMOUNT_LOVELACE = 80n * ADA;

/**
 * What each transaction needs from the consumer's **largest** utxo.
 *
 * The transaction builder picks utxos (largest first) until it has covered
 * the amount it has to move — `konduit_tx`'s "fuel" — and the fee comes out of
 * the same pot, together with the change output. The channel's value does
 * *not* help: it pays for the channel, not for the transaction that spends
 * it.
 *
 *   * the open moves the amount *into* the channel, so the fuel is the amount
 *     plus the validator's ada buffer,
 *   * the add moves the amount *into* the channel as well,
 *   * the close only moves the fee.
 *
 * The fee budget is per transaction, and each transaction is checked
 * separately — the wallet shrinks with every one of them.
 */
const OPEN_REQUIRED_LOVELACE =
  OPEN_AMOUNT_LOVELACE +
  MIN_ADA_BUFFER_LOVELACE +
  EXEC_FEE_BUDGET_LOVELACE +
  MIN_CHANGE_LOVELACE;
const ADD_REQUIRED_LOVELACE =
  ADD_AMOUNT_LOVELACE + EXEC_FEE_BUDGET_LOVELACE + MIN_CHANGE_LOVELACE;
const CLOSE_REQUIRED_LOVELACE = EXEC_FEE_BUDGET_LOVELACE + MIN_CHANGE_LOVELACE;

/** A top-up has to cover the *largest* of the three, on its own. */
const CONSUMER_TOPUP_LOVELACE = 100n * ADA;

const ADMIN_REQUIRED_LOVELACE = 60n * ADA;
const ADMIN_TOPUP_LOVELACE = 200n * ADA;

export type LifecycleScenarioResult = {
  tag: string;
  openTxId: string;
  addTxId: string;
  closeTxId?: string;
  /**
   * What the indexer's SQLite store ended up holding, when the db layer ran.
   * `null` when it was skipped — the on-chain assertions still ran.
   */
  indexed: IndexedLifecycleResult | null;
};

/** What the indexer's SQLite store ended up holding, when the db layer ran. */
export type IndexedLifecycleResult = {
  dbPath: string;
  channelTransactionId: string;
  /** Redeemer kinds the store recorded, in chain order. */
  stepRedeemers: string[];
  /** `slotNo` the store had reached when the last check passed. */
  indexedTipSlot: number;
  /** Final live state as the indexer sees it. */
  currentStage: string | null;
  currentAmountLovelace: number | null;
  /** The datum stage reached `Closed` (or the thread hit an EOL). */
  channelClosed: boolean;
};

/**
 * Open the store, prove it is usable, and remember where its tip is.
 *
 * The captured `tipSlot` is what makes the following checks trustworthy: the
 * indexer only advances its tip at the end of a successful sync pass, so
 * waiting for it to move past this value proves a *new* pass committed after we
 * started looking. Without that, a healthy but merely-lagging indexer would
 * look exactly like a broken one and every row wait would time out for the
 * wrong reason.
 */
const openIndexerForChecks = async (env: KonduitTestEnv) => {
  const { db, skipReason } = await openIndexerDbFor(env);
  if (!db) {
    console.log(`[lifecycle]   indexer db checks skipped: ${skipReason}`);
    return null;
  }
  const schema = unwrapOrPanic(
    verifyIndexerDbSchema(db),
    "[lifecycle] konduit-indexer db is not usable",
  );
  const tip = unwrapOrPanic(
    db.tip(),
    "[lifecycle] cannot read the indexer tip",
  );
  if (tip === null) {
    throw new Error(
      "[lifecycle] the konduit-indexer db has no blocks — the indexer has not " +
        "completed a sync pass yet (see .run/konduit-indexer.log)",
    );
  }
  console.log(
    `[lifecycle]   indexer db ready (${env.indexerDb.dbPath}, revision ${schema.schemaVersion}, tip slot ${tip.slotNo})`,
  );
  return { db, tipSlot: tip.slotNo };
};

/** Wait for the store's tip to move strictly past `tipSlot`. */
const waitForIndexerToCatchUp = async (
  db: NonNullable<Awaited<ReturnType<typeof openIndexerForChecks>>>["db"],
  env: KonduitTestEnv,
  tipSlot: number,
): Promise<number> => {
  const caughtUp = unwrapOrPanicWith(
    await waitForIndexedTipAfter(
      db,
      tipSlot + 1,
      env.indexerDb.timeoutMs,
      env.indexerDb.pollMs,
    ),
    (error) =>
      `[lifecycle] the konduit-indexer never advanced past slot ${tipSlot}: ` +
      `${error.reason} (gave up after ${error.attempts} polls / ${error.elapsedMs}ms)`,
  );
  return caughtUp.slotNo;
};

/** Re-render the structured poll failure the db layer returns. */
const pollFailure = (label: string, failure: PollFailure): string =>
  renderPollFailure(`[lifecycle] ${label}`, failure);

/** Re-render the structured invariant violations. */
const violations = (label: string, found: InvariantViolation[]): string =>
  `[lifecycle] ${label} disagrees with the chain:\n${formatViolations(found)}`;

export const runLifecycle = async (
  env: KonduitTestEnv,
): Promise<LifecycleScenarioResult> => {
  const { opts, cardano, adaptorConstants, closePeriodMs } = await bootstrap(
    env,
    {
      adminRequiredLovelace: ADMIN_REQUIRED_LOVELACE,
      adminTopupLovelace: ADMIN_TOPUP_LOVELACE,
      // The consumer is *not* funded here: every transaction needs a different
      // amount, and the wallet shrinks with each of them. It is topped up
      // before each transaction instead — see `ensureFunded` below.
      fundAlso: [],
    },
  );

  // The indexer db layer is optional: without it the scenario still proves the
  // on-chain behaviour, it just cannot prove the state was indexed.
  const indexer = await openIndexerForChecks(env);
  const db = indexer?.db ?? null;

  // --- open --------------------------------------------------------------
  console.log("[lifecycle] step 1/5 :: open a fresh channel");
  const tag = await findFreeTag(async () =>
    consumerShowTipJson(env.wallets.consumer, opts),
  );
  console.log(`[lifecycle]   tag=0x${tag}`);

  await ensureFunded(
    env,
    env.wallets.consumer.address,
    cardano,
    OPEN_REQUIRED_LOVELACE,
    CONSUMER_TOPUP_LOVELACE,
    "consumer (open)",
  );

  const openResult = unwrapOrPanicWith(
    await consumerTxOpen(
      env.wallets.consumer,
      [
        {
          tag,
          subVkeyHex: adaptorConstants.sub_vkey,
          closePeriodMs,
          ada: Number(OPEN_AMOUNT_LOVELACE / ADA),
        },
      ],
      opts,
    ),
    (error) => `consumer tx open failed: ${describeError(error)}`,
  );
  console.log(`[lifecycle]   open tx=${openResult.tx_id}`);

  const openedTip = await waitFor(
    "channel-open",
    async () => consumerShowTipJson(env.wallets.consumer, opts),
    (tip) => tip.channels.some((c) => c.tag === tag),
    env.defaultTimeoutMs,
    env.defaultPollMs,
  );
  const openedChannel = requireChannel(openedTip, tag, "after open");
  console.log(
    `[lifecycle]   channel open (amount=${openedChannel.amount_lovelace} buffered=${openedChannel.buffered_lovelace})`,
  );

  // --- db: the channel row must show up ---------------------------------
  let thread: ChannelThread | null = null;
  if (db && indexer) {
    console.log("[lifecycle]   checking the indexer inserted the channel…");
    // Prove the indexer is still making progress before we blame it for a
    // missing row.
    await waitForIndexerToCatchUp(db, env, indexer.tipSlot);
    const keytag = {
      addVkeyHex: openedChannel.add_vkey,
      tagHex: openedChannel.tag,
    };
    thread = unwrapOrPanicWith(
      await waitForIndexedChannel(db, env, keytag, openResult.tx_id),
      (failure) => pollFailure("indexing the channel row", failure),
    );

    unwrapOrPanicWith(
      assertChannelMatchesChain(thread.channel, {
        openTxId: openResult.tx_id,
        addVkeyHex: openedChannel.add_vkey,
        subVkeyHex: adaptorConstants.sub_vkey,
        tagHex: openedChannel.tag,
        scriptHashHex: env.validatorHashHex,
        // The store records the UTxO value, which includes the validator's ADA
        // buffer — hence the CLI's `buffered_lovelace`, not `amount_lovelace`.
        lovelace: openedChannel.buffered_lovelace,
      }),
      (found) => violations("the indexed channel row", found),
    );
    unwrapOrPanicWith(assertThreadInvariants(thread), (found) =>
      violations("the indexed thread", found),
    );
    // A freshly opened channel has no steps at all — if the store already has
    // some, it is reading something that is not this channel.
    if (thread.steps.length !== 0) {
      throw new Error(
        `[lifecycle] expected no indexed steps right after open, found ${thread.steps.length}`,
      );
    }
    console.log(
      `[lifecycle]   ✓ channel indexed at slot ${thread.channel.blockSlotNo} ` +
        `(lovelace=${thread.channel.lovelace}, datum=${thread.channel.datum?.stage.label})`,
    );
  }

  // --- add ---------------------------------------------------------------
  console.log("[lifecycle] step 2/5 :: add more funds");
  await ensureFunded(
    env,
    env.wallets.consumer.address,
    cardano,
    ADD_REQUIRED_LOVELACE,
    CONSUMER_TOPUP_LOVELACE,
    "consumer (add)",
  );

  const addResult = unwrapOrPanicWith(
    await consumerTxAdd(
      env.wallets.consumer,
      [{ tag, ada: Number(ADD_AMOUNT_LOVELACE / ADA) }],
      opts,
    ),
    (error) => `consumer tx add failed: ${describeError(error)}`,
  );
  console.log(`[lifecycle]   add tx=${addResult.tx_id}`);

  const expectedAmount =
    openedChannel.amount_lovelace + Number(ADD_AMOUNT_LOVELACE);
  const addedTip = await waitFor(
    "channel-amount",
    async () => consumerShowTipJson(env.wallets.consumer, opts),
    (tip) => {
      const channel = tip.channels.find((c) => c.tag === tag);
      return channel !== undefined && channel.amount_lovelace >= expectedAmount;
    },
    env.defaultTimeoutMs,
    env.defaultPollMs,
  );
  const addedChannel = requireChannel(addedTip, tag, "after add");
  console.log(
    `[lifecycle]   channel amount ${openedChannel.amount_lovelace} -> ${addedChannel.amount_lovelace}`,
  );

  // --- db: the add step must show up ------------------------------------
  if (db && thread) {
    console.log("[lifecycle]   checking the indexer inserted the add step…");
    const reloadedSteps = unwrapOrPanicWith(
      await waitForIndexedSteps(db, env, thread, {
        expectedStepCount: 1,
        expectedRedeemers: ["Cont:Add"],
      }),
      (failure) => pollFailure("indexing the add step", failure),
    );
    const addStep = reloadedSteps[0]!;
    if (addStep.transactionIdHex !== addResult.tx_id) {
      throw new Error(
        `[lifecycle] the first indexed step belongs to tx ${addStep.transactionIdHex}, ` +
          `but the add was submitted as ${addResult.tx_id}`,
      );
    }
    // Re-assemble the thread from the freshly read rows so the close check
    // below sees the add step.
    thread = assembleThread(thread.channel, reloadedSteps);
    unwrapOrPanicWith(assertThreadInvariants(thread), (found) =>
      violations("the indexed thread after add", found),
    );

    // `add.ak` leaves the *stage* untouched and only moves value, so the add
    // step's datum must be byte-identical to the channel's, and its `lovelace`
    // column must be exactly what the CLI reports as buffered.
    if (
      addStep.datumBlob === null ||
      hexString.fromUint8Array(addStep.datumBlob) !==
        hexString.fromUint8Array(thread.channel.datumBlob)
    ) {
      throw new Error(
        "[lifecycle] an `add` must not change the datum, but the indexed step's " +
          "datum differs from the channel's (add.ak requires stage_in == stage_out)",
      );
    }
    if (addStep.lovelace !== addedChannel.buffered_lovelace) {
      throw new Error(
        `[lifecycle] the indexed step's lovelace (${addStep.lovelace}) disagrees with the ` +
          `channel's buffered lovelace (${addedChannel.buffered_lovelace})`,
      );
    }
    if (
      addStep.lovelace !== undefined &&
      addStep.lovelace - openedChannel.buffered_lovelace !==
        Number(ADD_AMOUNT_LOVELACE)
    ) {
      throw new Error(
        `[lifecycle] the indexed amount grew by ` +
          `${addStep.lovelace - openedChannel.buffered_lovelace}, expected ${ADD_AMOUNT_LOVELACE}`,
      );
    }
    console.log(
      `[lifecycle]   ✓ add step indexed at slot ${addStep.blockSlotNo} ` +
        `(redeemer=${addStep.redeemerSteps.join("+")}, lovelace=${addStep.lovelace}, datum unchanged)`,
    );
  }

  // --- close -------------------------------------------------------------
  console.log("[lifecycle] step 3/5 :: close the channel");

  await ensureFunded(
    env,
    env.wallets.consumer.address,
    cardano,
    CLOSE_REQUIRED_LOVELACE,
    CONSUMER_TOPUP_LOVELACE,
    "consumer (close)",
  );

  const closeResult = unwrapOrPanicWith(
    await consumerTxClose(env.wallets.consumer, [tag], opts),
    (error) =>
      `consumer tx close failed: ${describeError(error)}` +
      explainNodeRejection(error),
  );
  console.log(`[lifecycle]   close tx=${closeResult.tx_id}`);

  const closedTip = await waitFor(
    "channel-closed",
    async () => consumerShowTipJson(env.wallets.consumer, opts),
    (tip) => {
      const channel = tip.channels.find((c) => c.tag === tag);
      return channel !== undefined && channel.stage.label === "closed";
    },
    env.defaultTimeoutMs,
    env.defaultPollMs,
  );
  const closedChannel = requireChannel(closedTip, tag, "after close");
  console.log(
    `[lifecycle]   channel closed (subbed=${subbedOf(closedChannel)})`,
  );

  // --- db: the closing step must show up -------------------------------
  let indexed: IndexedLifecycleResult | null = null;
  if (db && thread) {
    console.log("[lifecycle]   checking the indexer recorded the close…");
    const closeStep = unwrapOrPanicWith(
      await waitForStepInTransaction(
        db,
        env,
        thread,
        closeResult.tx_id,
        "Cont:Close",
      ),
      (failure) => pollFailure("indexing the close step", failure),
    );
    // `consumer tx --close` submits `Cont::Close`, which the indexer stores as a
    // *transition*: a continuation output whose datum stage is now `Closed`.
    unwrapOrPanicWith(
      assertClosingStepShape(closeStep, "closed-stage"),
      (found) => violations("the indexed close step", found),
    );

    const reloaded = unwrapOrPanicWith(
      await waitForThreadSteps(
        db,
        env,
        {
          addVkeyHex: openedChannel.add_vkey,
          tagHex: openedChannel.tag,
        },
        // open + add + close
        2,
      ),
      (failure) =>
        pollFailure("reloading the indexed thread after close", failure),
    );
    if (reloaded.terminated) {
      throw new Error(
        `[lifecycle] the store reports the thread as terminated after a Cont::Close; ` +
          `only a mutual or end-of-life close may do that`,
      );
    }
    unwrapOrPanicWith(assertThreadInvariants(reloaded), (found) =>
      violations("the indexed thread after close", found),
    );
    unwrapOrPanicWith(
      assertDbMatchesCli(reloaded, {
        stageLabel: "closed",
        // The store records the UTxO value, which includes the validator's ADA
        // buffer — hence the CLI's `buffered_lovelace`.
        bufferedLovelace: closedChannel.buffered_lovelace,
      }),
      (found) => violations("the indexed thread after close", found),
    );
    const tip = db.tip();
    indexed = {
      dbPath: env.indexerDb.dbPath,
      channelTransactionId: thread.channel.transactionIdHex,
      stepRedeemers: reloaded.steps.map((s) => s.redeemerSteps.join("+")),
      indexedTipSlot: tip.isOk() ? (tip.value?.slotNo ?? 0) : 0,
      currentStage: reloaded.currentStage?.label ?? null,
      currentAmountLovelace: indexedAmount(reloaded),
      channelClosed: isChannelClosed(reloaded),
    };
    console.log(
      `[lifecycle]   ✓ close step indexed at slot ${closeStep.blockSlotNo} ` +
        `(thread steps: ${indexed.stepRedeemers.join(" → ")})`,
    );
  }

  console.log(`[lifecycle] ✓ scenario complete (tag=0x${tag})`);

  return {
    tag,
    openTxId: openResult.tx_id,
    addTxId: addResult.tx_id,
    closeTxId: closeResult.tx_id,
    indexed,
  };
};

// --- helpers ---------------------------------------------------------------

const requireChannel = (
  tip: ConsumerTipOutput,
  tag: string,
  when: string,
): ConsumerTipOutput["channels"][number] => {
  const channel = tip.channels.find((c) => c.tag === tag);
  if (!channel)
    throw new Error(`channel ${tag} is missing from the consumer tip ${when}`);
  return channel;
};

const subbedOf = (channel: ConsumerTipOutput["channels"][number]): number =>
  channel.stage.label === "opened" || channel.stage.label === "closed"
    ? channel.stage.subbed
    : 0;

export const __test_internals = { uniqueTagHex };

import * as fs from "node:fs";
import {
  type ConsumerTipOutput,
  type KonduitCliOptions,
  consumerMakeLocked,
  consumerMakeSquash,
  consumerShowKeytagJson,
  consumerShowTipJson,
  consumerTxOpen,
  describeError,
  adaptorTxReceipt,
} from "../konduitCli.js";

import { isChannelClosed, type ChannelThread } from "../indexerDb.js";
import type { UtxoMap } from "@konduit/cardano-cli";
import type { KonduitTestEnv } from "../testEnv.js";
import {
  type InvariantViolation,
  type PollFailure,
  assertChannelMatchesChain,
  assertDbMatchesCli,
  assertThreadInvariants,
  formatViolations,
  indexedAmount,
  openIndexerDbFor,
  verifyIndexerDbSchema,
  waitForIndexedChannel,
  waitForIndexedTipAfter,
  waitForStepInTransaction,
  waitForThreadSteps,
} from "./indexing.js";
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
  totalLovelaceAt,
  utxosAt,
  waitFor,
  waitForValue,
} from "./shared.js";
import {
  unwrapOrPanic,
  unwrapOrPanicWith,
} from "@konduit/konduit-consumer/neverthrow";

/**
 * @file Scenario: settle L2 cheques against the channel and pull the money L1.
 *
 * This is the "real" l1/l2 round trip. Nothing here fabricates a transfer —
 * the adaptor's balance only grows if the *validator* accepted the sub and
 * the ledger moved the value:
 *
 *   1. bootstrap (fund wallets, deploy reference script if needed, resolve
 *      the adaptor's `sub_vkey` / `close_period`),
 *   2. consumer opens a channel on a fresh, unique tag,
 *   3. consumer issues **locked cheques** — off-chain promises, each valid only
 *      after the adaptor reveals the shared secret,
 *   4. consumer squashes the earlier indices into a single `amount`,
 *   5. adaptor executes `adaptor tx --receipt`, proving every cheque against
 *      the keytag and paying the gain out of the channel UTxO,
 *   6. assert on-chain that:
 *        - the channel's `subbed` counter moved (the adaptor paid),
 *        - the channel's remaining `amount_lovelace` dropped by exactly the
 *          gain,
 *        - the **adaptor was paid**: the sub transaction paid the adaptor's
 *          address at least the gain, and the adaptor's balance moved by
 *          exactly `gain - fee` (the adaptor pays the sub's execution fee out
 *          of its own pocket, so its balance does *not* grow by the gain).
 *   7. assert the *indexer* recorded the same thing: a `Cont::Sub` step whose
 *      datum shows the reduced amount and the retired cheque indexes.
 *
 * Protocol arithmetic (see `Channel::sub` in `packages/konduit/tx/src/channel.rs`):
 *
 *   absolute_owed = squash.amount + Σ useds
 *   relative_owed = absolute_owed − subbed
 *   gain          = min(relative_owed, channel.amount)
 *
 * so with `subbed = 0` and no prior `useds`, `gain = squash.amount + Σ cheque
 * amounts`. The CLI additionally enforces `min_single = 10_000` per channel and
 * `min_total = 1_000_000` across the whole transaction.
 *
 * The db checks in step 7 are the interesting part for this scenario: a sub is
 * the only step where the *datum itself* changes in a way the CLI cannot easily
 * be asked about (the `useds` list and the retired `subbed`), so reading it back
 * out of the store is the only way to confirm the indexer understood the
 * redeemer rather than merely noticing an output.
 */

// --- Scenario parameters ---------------------------------------------------

const CHANNEL_OPEN_LOVELACE = 100n * ADA;

/**
 * What the consumer's **largest** utxo has to cover for the open: the amount
 * that goes into the channel, the validator's ada buffer, the fee and a
 * change output big enough to be valid.
 */
const CONSUMER_REQUIRED_LOVELACE =
  CHANNEL_OPEN_LOVELACE +
  MIN_ADA_BUFFER_LOVELACE +
  EXEC_FEE_BUDGET_LOVELACE +
  MIN_CHANGE_LOVELACE;
const CONSUMER_TOPUP_LOVELACE = 150n * ADA;

/**
 * The adaptor only pays the fee here — the *gain* it receives is paid out of
 * the channel UTxO, so it cannot be used to fund its own submission.
 */
const ADAPTOR_REQUIRED_LOVELACE =
  EXEC_FEE_BUDGET_LOVELACE + MIN_CHANGE_LOVELACE;
const ADAPTOR_TOPUP_LOVELACE = 100n * ADA;

const ADMIN_REQUIRED_LOVELACE = 60n * ADA;
const ADMIN_TOPUP_LOVELACE = 200n * ADA;

/**
 * Cheques are locked with this secret. `Locked::make` stores `sha256(secret)`
 * as the lock, and the adaptor has to present the pre-image to unlock — so
 * the secret is genuinely required for the sub to be accepted.
 */
const SECRET_HEX = "00".repeat(32);

/**
 * Cheque lifetime, counted from the current time.
 *
 * `konduit-cli consumer make locked --duration` resolves the duration against
 * the current time, which is the only clock that matters: the validator
 * compares a cheque's timeout with the transaction's validity range, and that
 * range is anchored at the same instant.
 */
const CHEQUE_DURATION = "1h";

/**
 * How far ahead of the current time a cheque's timeout has to be.
 *
 * The transaction's validity range reaches `now + 300 slots`, and the
 * adaptor *drops* every cheque whose timeout is not beyond that bound — so a
 * cheque that is about to expire would quietly settle less than it should.
 * Five minutes is comfortably above the validity window on every chain we
 * know of (300 slots of 200ms on the fast testnet, five minutes on the public
 * networks) while still well inside the cheque's lifetime.
 */
const CHEQUE_TIMEOUT_MARGIN_MS = 5 * 60 * 1000;

/**
 * Index of the throw-away cheque used to read the current time.
 *
 * It must not collide with a real cheque's index: the probe is signed by the
 * consumer for the same channel, so it has to be unmistakably not one of
 * them.
 */
const CLOCK_PROBE_INDEX = 0xffff_ffff;

/** Squash covers indexes `[0, SQUASH_INDEX]`; cheque indexes start above it. */
const SQUASH_INDEX = 0;
const SQUASH_LOVELACE = 2n * ADA;

/** Two cheques prove the multi-cheque path of `parse_keytag_receipt`. */
const CHEQUES: Array<{ index: number; lovelace: bigint }> = [
  { index: SQUASH_INDEX + 1, lovelace: 3n * ADA },
  { index: SQUASH_INDEX + 2, lovelace: 4n * ADA },
];

const EXPECTED_GAIN_LOVELACE =
  SQUASH_LOVELACE + CHEQUES.reduce((acc, c) => acc + c.lovelace, 0n);

// --- Scenario --------------------------------------------------------------

export type SubScenarioResult = {
  tag: string;
  openTxId: string;
  subTxId: string;
  expectedGainLovelace: bigint;
  /** What the sub transaction paid the adaptor's address. */
  adaptorPayoutLovelace: bigint;
  /** What the sub transaction cost the adaptor in execution fees. */
  adaptorFeeLovelace: bigint;
  /** What the indexer's store holds, when the db layer ran. */
  indexed: IndexedSubResult | null;
};

/** The db-visible trace of a sub: the one place the datum really changes. */
export type IndexedSubResult = {
  dbPath: string;
  channelTransactionId: string;
  /** Amount the store reports after the sub. */
  amountAfterSubLovelace: number | null;
  /** Cheque indexes the store recorded as `used`. */
  usedIndexes: number[];
  /** What the decoded redeemer said the sub carried. */
  redeemerSteps: string[];
};

const pollFailure = (label: string, failure: PollFailure): string =>
  renderPollFailure(`[sub] ${label}`, failure);

const violations = (label: string, found: InvariantViolation[]): string =>
  `[sub] ${label} disagrees with the chain:\n${formatViolations(found)}`;

export const runSub = async (
  env: KonduitTestEnv,
): Promise<SubScenarioResult> => {
  // ---------------------------------------------------------------------
  // 1. Bootstrap
  // ---------------------------------------------------------------------
  console.log("[sub] step 1/8 :: bootstrap");
  const { opts, cardano, adaptorConstants, closePeriodMs } = await bootstrap(
    env,
    {
      adminRequiredLovelace: ADMIN_REQUIRED_LOVELACE,
      adminTopupLovelace: ADMIN_TOPUP_LOVELACE,
      // The consumer is funded before the open (the only transaction it
      // pays for); the adaptor before the receipt.
      fundAlso: [],
    },
  );

  await ensureFunded(
    env,
    env.wallets.consumer.address,
    cardano,
    CONSUMER_REQUIRED_LOVELACE,
    CONSUMER_TOPUP_LOVELACE,
    "consumer (open)",
  );

  await ensureFunded(
    env,
    env.wallets.adaptor.address,
    cardano,
    ADAPTOR_REQUIRED_LOVELACE,
    ADAPTOR_TOPUP_LOVELACE,
    "adaptor (receipt)",
  );

  // Snapshot the adaptor's balance *before* the sub so we can prove the
  // transfer really moved value onto L1.
  const adaptorBefore = unwrapOrPanic(
    totalLovelaceAt(env.wallets.adaptor.address, cardano, {
      skipReferenceScriptUtxos: true,
    }),
    "Failed to query adaptor balance",
  );

  // ---------------------------------------------------------------------
  // 2. Fresh channel on a unique tag
  // ---------------------------------------------------------------------
  console.log("[sub] step 2/8 :: open a fresh channel");

  // The indexer db layer is optional; without it the scenario still proves the
  // on-chain behaviour, it just cannot prove the state was indexed.
  const indexerHandle = await openIndexerDbFor(env);
  const db = indexerHandle.db;
  let indexerTipSlot = 0;
  if (db) {
    const schema = unwrapOrPanic(
      verifyIndexerDbSchema(db),
      "[sub] konduit-indexer db is not usable",
    );
    const tip = unwrapOrPanic(db.tip(), "[sub] cannot read the indexer tip");
    if (tip === null) {
      throw new Error(
        "[sub] the konduit-indexer db has no blocks — the indexer has not " +
          "completed a sync pass yet (see .run/konduit-indexer.log)",
      );
    }
    indexerTipSlot = tip.slotNo;
    console.log(
      `[sub]   indexer db ready (${env.indexerDb.dbPath}, revision ${schema.schemaVersion}, tip slot ${indexerTipSlot})`,
    );
  } else {
    console.log(
      `[sub]   indexer db checks skipped: ${indexerHandle.skipReason}`,
    );
  }

  const tag = await findFreeTag(async () =>
    consumerShowTipJson(env.wallets.consumer, opts),
  );
  console.log(`[sub]   tag=0x${tag}`);

  const openResult = unwrapOrPanicWith(
    consumerTxOpen(
      env.wallets.consumer,
      [
        {
          tag,
          subVkeyHex: adaptorConstants.sub_vkey,
          closePeriodMs,
          ada: Number(CHANNEL_OPEN_LOVELACE / ADA),
        },
      ],
      opts,
    ),
    (error) => `consumer tx open failed: ${describeError(error)}`,
  );

  console.log(`[sub]   open tx=${openResult.tx_id}`);

  const openedTip = await waitFor(
    "channel-open",
    async () => consumerShowTipJson(env.wallets.consumer, opts),
    (tip) => tip.channels.some((c) => c.tag === tag),
    env.defaultTimeoutMs,
    env.defaultPollMs,
  );
  const openedChannel = requireChannel(openedTip, tag, "after open");
  const channelAmountAfterOpen = openedChannel.amount_lovelace;
  console.log(
    `[sub]   channel open (amount=${channelAmountAfterOpen} buffered=${openedChannel.buffered_lovelace} subbed=${subbedOf(openedChannel)})`,
  );

  // The db view of the fresh channel, kept for the post-sub comparison.
  let thread: ChannelThread | null = null;
  if (db) {
    console.log("[sub]   checking the indexer inserted the channel…");
    // Prove the indexer is still making progress before blaming it for a
    // missing row: its tip only advances when a sync pass commits.
    unwrapOrPanicWith(
      await waitForIndexedTipAfter(
        db,
        indexerTipSlot + 1,
        env.indexerDb.timeoutMs,
        env.indexerDb.pollMs,
      ),
      (error) =>
        `[sub] the konduit-indexer never advanced past slot ${indexerTipSlot}: ${error.reason}`,
    );
    thread = unwrapOrPanicWith(
      await waitForIndexedChannel(
        db,
        env,
        { addVkeyHex: openedChannel.add_vkey, tagHex: openedChannel.tag },
        openResult.tx_id,
      ),
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
        // buffer — hence the CLI's `buffered_lovelace`.
        lovelace: openedChannel.buffered_lovelace,
      }),
      (found) => violations("the indexed channel row", found),
    );
    console.log(
      `[sub]   ✓ channel indexed at slot ${thread.channel.blockSlotNo} (lovelace=${thread.channel.lovelace})`,
    );
  }

  // ---------------------------------------------------------------------
  // 3. Consumer issues locked cheques (L2 promises)
  // ---------------------------------------------------------------------
  console.log("[sub] step 3/8 :: consumer issues locked cheques");
  const cheques: Array<{
    index: number;
    bodyCborHex: string;
    signatureHex: string;
    /** Absolute timeout, in POSIX milliseconds. */
    timeoutMs: number;
  }> = [];
  for (const cheque of CHEQUES) {
    const locked = unwrapOrPanicWith(
      await consumerMakeLocked(
        env.wallets.consumer,
        {
          tag,
          index: cheque.index,
          amount: Number(cheque.lovelace),
          durationMs: CHEQUE_DURATION,
          secretHex: SECRET_HEX,
        },
        opts,
      ),
      (error) => `consumer make locked failed: ${describeError(error)}`,
    );
    console.log(
      `[sub]   cheque #${cheque.index} amount=${cheque.lovelace} timeout_ms=${locked.timeout_ms}`,
    );
    cheques.push({
      index: cheque.index,
      bodyCborHex: locked.body_cbor_hex,
      signatureHex: locked.signature_hex,
      timeoutMs: locked.timeout_ms,
    });
  }

  // ---------------------------------------------------------------------
  // 4. Consumer squashes the earlier indices
  // ---------------------------------------------------------------------
  console.log("[sub] step 4/8 :: consumer squashes");
  const squash = unwrapOrPanicWith(
    await consumerMakeSquash(
      env.wallets.consumer,
      {
        tag,
        amount: Number(SQUASH_LOVELACE),
        index: SQUASH_INDEX,
      },
      opts,
    ),
    (error) => `consumer make squash failed: ${describeError(error)}`,
  );
  console.log(`[sub]   squash amount=${squash.amount} index=${squash.index}`);

  // ---------------------------------------------------------------------
  // 5. Consumer hands the adaptor a keytag binding them together
  // ---------------------------------------------------------------------
  console.log("[sub] step 5/8 :: resolve the channel keytag");
  const keytag = unwrapOrPanicWith(
    await consumerShowKeytagJson(env.wallets.consumer, tag, opts),
    (error) => `consumer show keytag failed: ${describeError(error)}`,
  );
  console.log(`[sub]   keytag=${keytag.keytag.slice(0, 16)}…`);

  // ---------------------------------------------------------------------
  // 6. Adaptor executes the sub — this is the actual L1 money movement
  // ---------------------------------------------------------------------
  console.log("[sub] step 6/8 :: adaptor executes the receipt");

  // The cheque's timeout is absolute, and the validator compares it with the
  // transaction's validity range — both are anchored at the current time. The
  // adaptor silently drops every cheque that is not beyond the validity
  // range, so a cheque that is about to expire would settle less than it
  // should (or nothing at all) instead of failing loudly. Check the absolute
  // timeouts against the current time before handing the receipt over.
  const now = await nowMs(env, opts, tag);
  for (const cheque of cheques) {
    if (cheque.timeoutMs <= now + CHEQUE_TIMEOUT_MARGIN_MS) {
      throw new Error(
        `cheque #${cheque.index} expires at ${cheque.timeoutMs} (POSIX ms), but the clock ` +
          `is already at ${now} — the margin is ${CHEQUE_TIMEOUT_MARGIN_MS}ms. ` +
          `The adaptor would drop the cheque and the sub would settle less than expected.`,
      );
    }
  }
  console.log(
    `[sub]   now=${now} ms, cheque timeouts ${cheques
      .map((c) => c.timeoutMs)
      .join(", ")} ms`,
  );

  // Persist the exact receipt before submitting. When the node rejects the
  // transaction with an opaque machine error, this is what you need in order to
  // replay it by hand:
  //
  //   konduit-cli adaptor tx --receipt "$(cat <file>)"
  //
  const receiptArg = [
    keytag.keytag,
    `${squash.body_cbor_hex},${squash.signature_hex}`,
    ...cheques.map((c) => `${c.bodyCborHex},${c.signatureHex},${SECRET_HEX}`),
  ].join(";");
  const receiptPath = dumpReceipt(env, tag, receiptArg);

  const receiptResult = unwrapOrPanicWith(
    await adaptorTxReceipt(
      env.wallets.adaptor,
      [
        {
          keytagHex: keytag.keytag,
          squashBodyCborHex: squash.body_cbor_hex,
          squashSignatureHex: squash.signature_hex,
          // Passing the secret turns each `Locked` into an `Unlocked`, which is
          // what `Receipt::prep_sub` needs to compute the gain.
          cheques: cheques.map((c) => ({
            bodyCborHex: c.bodyCborHex,
            signatureHex: c.signatureHex,
            secretHex: SECRET_HEX,
          })),
        },
      ],
      opts,
    ),
    (error) =>
      `adaptor tx receipt failed: ${describeError(error)}\n` +
      `  receipt written to: ${receiptPath}` +
      explainNodeRejection(error),
  );
  console.log(`[sub]   sub tx=${receiptResult.tx_id}`);

  // ---------------------------------------------------------------------
  // 7. Assert the channel recorded the sub
  // ---------------------------------------------------------------------
  console.log("[sub] step 7/8 :: verify the channel recorded the sub");
  const subbedTip = await waitFor(
    "channel-subbed",
    async () => consumerShowTipJson(env.wallets.consumer, opts),
    (tip) => {
      const channel = tip.channels.find((c) => c.tag === tag);
      return channel !== undefined && subbedOf(channel) > 0;
    },
    env.defaultTimeoutMs,
    env.defaultPollMs,
  );
  const subbedChannel = requireChannel(subbedTip, tag, "after sub");
  const subbed = subbedOf(subbedChannel);
  const channelAmountAfterSub = subbedChannel.amount_lovelace;

  if (subbed !== Number(EXPECTED_GAIN_LOVELACE)) {
    throw new Error(
      `sub mismatch: expected the channel to be subbed ${EXPECTED_GAIN_LOVELACE} lovelace, got ${subbed}. ` +
        `expected gain = squash(${SQUASH_LOVELACE}) + Σcheques(${CHEQUES.map((c) => c.lovelace).join("+")})`,
    );
  }
  const expectedRemaining =
    channelAmountAfterOpen - Number(EXPECTED_GAIN_LOVELACE);
  if (channelAmountAfterSub !== expectedRemaining) {
    throw new Error(
      `channel amount mismatch: expected ${expectedRemaining} after the sub, got ${channelAmountAfterSub}`,
    );
  }
  console.log(
    `[sub]   subbed=${subbed} channel amount ${channelAmountAfterOpen} -> ${channelAmountAfterSub}`,
  );

  // The squash must have retired indexes `[0, SQUASH_INDEX]` while the cheque
  // indexes survive as `useds`.
  const usedIndexes =
    subbedChannel.stage.label === "opened"
      ? subbedChannel.stage.useds.map((u) => u.index)
      : [];
  for (const cheque of CHEQUES) {
    if (!usedIndexes.includes(cheque.index)) {
      throw new Error(
        `expected cheque index ${cheque.index} to appear in the channel's useds, got [${usedIndexes.join(",")}]`,
      );
    }
  }
  if (usedIndexes.includes(SQUASH_INDEX)) {
    throw new Error(
      `squashed index ${SQUASH_INDEX} should not reappear in useds, got [${usedIndexes.join(",")}]`,
    );
  }
  console.log(`[sub]   useds=[${usedIndexes.join(",")}]`);

  // ---------------------------------------------------------------------
  // 8. Assert the adaptor actually received the money on L1
  // ---------------------------------------------------------------------
  console.log("[sub] step 8/8 :: verify the adaptor was paid");

  // The gain is paid out of the channel UTxO into the adaptor's own output —
  // but the adaptor also pays the fee of the sub transaction out of its own
  // pocket. Its balance therefore moves by `gain - fee`, *not* by `gain`.
  //
  // What can be asserted exactly:
  //   * the sub transaction paid the adaptor's address at least the gain.
  //     Together with step 7 — the channel lost exactly the gain — this proves
  //     the value moved out of the channel and into the adaptor's output, and
  //   * the adaptor's balance moved by exactly `gain - fee`, with the fee
  //     inside the budgeted fee.
  const payout = await waitForValue(
    "adaptor-payout",
    async () => {
      const utxos = utxosAt(env.wallets.adaptor.address, cardano);
      return utxos.isOk() ? payoutOfTx(utxos.value, receiptResult.tx_id) : 0n;
    },
    (value) => value >= EXPECTED_GAIN_LOVELACE,
    env.defaultTimeoutMs,
    env.defaultPollMs,
  );

  const adaptorAfter = unwrapOrPanic(
    totalLovelaceAt(env.wallets.adaptor.address, cardano, {
      skipReferenceScriptUtxos: true,
    }),
    "Failed to query the adaptor's balance",
  );
  const fee = adaptorBefore + EXPECTED_GAIN_LOVELACE - adaptorAfter;
  if (fee <= 0n) {
    throw new Error(
      `the adaptor's balance moved from ${adaptorBefore} to ${adaptorAfter}, ` +
        `which is more than the gain (${EXPECTED_GAIN_LOVELACE}) — the sub paid out more than it should`,
    );
  }
  if (fee > EXEC_FEE_BUDGET_LOVELACE) {
    throw new Error(
      `the sub transaction cost the adaptor ${fee} lovelace in fees, which is over the ` +
        `budgeted execution fee (${EXEC_FEE_BUDGET_LOVELACE})`,
    );
  }
  console.log(
    `[sub]   adaptor paid ${payout} lovelace (gain ${EXPECTED_GAIN_LOVELACE} + fee ${fee}); ` +
      `balance ${adaptorBefore} -> ${adaptorAfter}`,
  );

  // ---------------------------------------------------------------------
  // 9. Assert the indexer stored the same sub
  // ---------------------------------------------------------------------
  let indexed: IndexedSubResult | null = null;
  if (db && thread) {
    console.log("[sub] step 9/9 :: checking the indexer recorded the sub");
    const subStep = unwrapOrPanicWith(
      await waitForStepInTransaction(
        db,
        env,
        thread,
        receiptResult.tx_id,
        "Cont:Sub",
      ),
      (failure) => pollFailure("indexing the sub step", failure),
    );
    const reloaded = unwrapOrPanicWith(
      await waitForThreadSteps(
        db,
        env,
        { addVkeyHex: openedChannel.add_vkey, tagHex: openedChannel.tag },
        1,
      ),
      (failure) =>
        pollFailure("reloading the indexed thread after the sub", failure),
    );
    unwrapOrPanicWith(assertThreadInvariants(reloaded), (found) =>
      violations("the indexed thread after the sub", found),
    );
    unwrapOrPanicWith(
      assertDbMatchesCli(reloaded, {
        stageLabel: "opened",
        // The store records the UTxO value, which includes the validator's ADA
        // buffer — hence the CLI's `buffered_lovelace`.
        bufferedLovelace: subbedChannel.buffered_lovelace,
      }),
      (found) => violations("the indexed thread after the sub", found),
    );

    // A sub is the one step that *retires cheque indexes*, so this is the only
    // place the store's copy of the state carries information the CLI cannot be
    // asked about directly. Assert the same indexes the CLI reported.
    const indexedUseds =
      reloaded.currentStage?.label === "Opened"
        ? reloaded.currentStage.useds.map((u) => Number(u.index))
        : [];
    for (const cheque of CHEQUES) {
      if (!indexedUseds.includes(cheque.index)) {
        throw new Error(
          `[sub] the indexer's useds do not contain cheque index ${cheque.index}: [${indexedUseds.join(",")}]`,
        );
      }
    }
    if (indexedUseds.includes(SQUASH_INDEX)) {
      throw new Error(
        `[sub] squashed index ${SQUASH_INDEX} reappeared in the indexer's useds: [${indexedUseds.join(",")}]`,
      );
    }
    // The sub must not have closed anything.
    if (isChannelClosed(reloaded)) {
      throw new Error(
        "[sub] the indexer reports the channel as closed after a sub",
      );
    }
    if (subStep.lovelace !== subbedChannel.buffered_lovelace) {
      throw new Error(
        `[sub] the indexed sub step holds ${subStep.lovelace} lovelace, expected ${subbedChannel.buffered_lovelace}`,
      );
    }

    indexed = {
      dbPath: env.indexerDb.dbPath,
      channelTransactionId: thread.channel.transactionIdHex,
      amountAfterSubLovelace: indexedAmount(reloaded),
      usedIndexes: indexedUseds,
      redeemerSteps: subStep.redeemerSteps,
    };
    console.log(
      `[sub]   ✓ sub step indexed at slot ${subStep.blockSlotNo} ` +
        `(redeemer=${subStep.redeemerSteps.join("+")}, amount=${indexed.amountAfterSubLovelace}, useds=[${indexedUseds.join(",")}])`,
    );
  }

  console.log(`[sub] ✓ scenario complete (tag=0x${tag})`);

  return {
    tag,
    openTxId: openResult.tx_id,
    subTxId: receiptResult.tx_id,
    expectedGainLovelace: EXPECTED_GAIN_LOVELACE,
    adaptorPayoutLovelace: payout,
    adaptorFeeLovelace: fee,
    indexed,
  };
};

// --- helpers ---------------------------------------------------------------

/**
 * Sum the value of the outputs transaction `txId` created.
 *
 * The UTxO map is keyed by `txid#ix`, so this attributes an output to the
 * transaction that created it — which is what tells "the adaptor was paid by
 * *this* sub" apart from "the adaptor holds some ada".
 */
const payoutOfTx = (utxos: UtxoMap, txId: string): bigint => {
  let total = 0n;
  for (const [reference, output] of utxos) {
    if (reference.startsWith(`${txId}#`)) {
      total += BigInt(output.value.lovelace);
    }
  }
  return total;
};

/**
 * The current time, in POSIX milliseconds.
 *
 * `konduit-cli consumer make locked --duration <d>` resolves the duration
 * against the current time — the validator compares a cheque's timeout with
 * the transaction's validity range, and that range is anchored at the same
 * instant. A zero-length duration therefore hands us that clock verbatim:
 * the probe's timeout *is* the current time.
 *
 * The probe is a throw-away cheque — it never leaves this process and is
 * never part of a receipt.
 */
const nowMs = async (
  env: KonduitTestEnv,
  opts: KonduitCliOptions,
  tag: string,
): Promise<number> => {
  const probe = unwrapOrPanicWith(
    await consumerMakeLocked(
      env.wallets.consumer,
      {
        tag,
        index: CLOCK_PROBE_INDEX,
        amount: 0,
        durationMs: 0,
        secretHex: SECRET_HEX,
      },
      opts,
    ),
    (error) =>
      `consumer make locked (clock probe) failed: ${describeError(error)}`,
  );
  return probe.timeout_ms;
};

/**
 * Write the receipt next to the other e2e artefacts so a rejected transaction
 * can be replayed by hand. Best effort: never let diagnostics turn a passing
 * scenario into a failing one.
 *
 * The receipt argument goes into its own file: the `--receipt` value is a
 * single `;`-separated string, and the CLI parses it strictly — a file with
 * comment lines in front of it would be rejected with `invalid tag`.
 */
const dumpReceipt = (
  env: KonduitTestEnv,
  tag: string,
  receiptArg: string,
): string => {
  const dir = `${env.repoRoot}/.run/e2e`;
  const argPath = `${dir}/receipt-${tag}.arg`;
  const path = `${dir}/receipt-${tag}.txt`;
  try {
    fs.mkdirSync(dir, { recursive: true });
    fs.writeFileSync(argPath, `${receiptArg}\n`);
    fs.writeFileSync(
      path,
      [
        `# tag            = ${tag}`,
        `# squash         = ${SQUASH_LOVELACE} lovelace, index ${SQUASH_INDEX}`,
        `# cheques        = ${CHEQUES.map((c) => `#${c.index}:${c.lovelace}`).join(", ")}`,
        `# secret         = ${SECRET_HEX}`,
        `# expected gain  = ${EXPECTED_GAIN_LOVELACE} lovelace`,
        "#",
        "# replay:",
        `#   konduit-cli adaptor tx --receipt "$(cat ${argPath})"`,
        "",
      ].join("\n"),
    );
  } catch {
    // ignore — diagnostics only
  }
  return path;
};

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

/**
 * `subbed` is the running total paid out of the channel. It only appears on
 * the `Opened` and `Closed` stages; `Responded` folds it into the pendings.
 */
const subbedOf = (channel: ConsumerTipOutput["channels"][number]): number =>
  channel.stage.label === "opened" || channel.stage.label === "closed"
    ? channel.stage.subbed
    : 0;

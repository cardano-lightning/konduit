import * as fs from "node:fs";
import {
  type ConsumerTipOutput,
  consumerMakeLocked,
  consumerMakeSquash,
  consumerShowKeytagJson,
  consumerShowTipJson,
  consumerTxOpen,
  describeError,
  adaptorTxReceipt,
} from "../konduitCli.js";
import { totalLovelaceAt } from "../cardanoCli.js";
import type { KonduitTestEnv } from "../testEnv.js";
import {
  ADA,
  EXEC_FEE_BUDGET_LOVELACE,
  MIN_ADA_BUFFER_LOVELACE,
  MIN_CHANGE_LOVELACE,
  bootstrap,
  explainNodeRejection,
  findFreeTag,
  waitFor,
  waitForValue,
} from "./shared.js";

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
 *        - the **adaptor's own wallet** actually gained lovelace.
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
 */

// --- Scenario parameters ---------------------------------------------------

const CHANNEL_OPEN_LOVELACE = 100n * ADA;

/**
 * Lovelace the consumer needs *free* (not locked in a channel) to cover the
 * open, the validator's own buffer, the ~24 ada execution fee, and a valid
 * change output.
 */
const CONSUMER_REQUIRED_LOVELACE =
  CHANNEL_OPEN_LOVELACE +
  MIN_ADA_BUFFER_LOVELACE +
  EXEC_FEE_BUDGET_LOVELACE +
  MIN_CHANGE_LOVELACE;
const CONSUMER_TOPUP_LOVELACE = 300n * ADA;

/**
 * The adaptor only pays the execution fee here — the *gain* it receives is
 * paid out of the channel UTxO, so it cannot be used to fund its own
 * submission.
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

/** Cheque lifetime. Must exceed the tx's upper bound (`now + 20min`). */
const CHEQUE_DURATION = "1h";

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
  observedGainLovelace: bigint;
};

export const runSub = async (
  env: KonduitTestEnv,
): Promise<SubScenarioResult> => {
  // ---------------------------------------------------------------------
  // 1. Bootstrap
  // ---------------------------------------------------------------------
  console.log("[sub] step 1/8 :: bootstrap");
  const { opts, node, adaptorConstants, closePeriodMs } = await bootstrap(env, {
    adminRequiredLovelace: ADMIN_REQUIRED_LOVELACE,
    adminTopupLovelace: ADMIN_TOPUP_LOVELACE,
    fundAlso: [
      {
        role: "consumer",
        address: env.wallets.consumer.address,
        requiredLovelace: CONSUMER_REQUIRED_LOVELACE,
        topupLovelace: CONSUMER_TOPUP_LOVELACE,
      },
      {
        role: "adaptor",
        address: env.wallets.adaptor.address,
        requiredLovelace: ADAPTOR_REQUIRED_LOVELACE,
        topupLovelace: ADAPTOR_TOPUP_LOVELACE,
      },
    ],
  });

  // Snapshot the adaptor's balance *before* the sub so we can prove the
  // transfer really moved value onto L1.
  const adaptorBefore = totalLovelaceAt(env.wallets.adaptor.address, node, {
    skipReferenceScriptUtxos: true,
  });
  if (adaptorBefore.isErr()) {
    throw new Error(
      `Failed to query adaptor balance: ${String(adaptorBefore.error)}`,
    );
  }

  // ---------------------------------------------------------------------
  // 2. Fresh channel on a unique tag
  // ---------------------------------------------------------------------
  console.log("[sub] step 2/8 :: open a fresh channel");
  const tag = await findFreeTag(async () =>
    consumerShowTipJson(env.wallets.consumer, opts),
  );
  console.log(`[sub]   tag=0x${tag}`);

  const openResult = await consumerTxOpen(
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
  );
  if (openResult.isErr()) {
    throw new Error(
      `consumer tx open failed: ${describeError(openResult.error)}`,
    );
  }
  console.log(`[sub]   open tx=${openResult.value.tx_id}`);

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

  // ---------------------------------------------------------------------
  // 3. Consumer issues locked cheques (L2 promises)
  // ---------------------------------------------------------------------
  console.log("[sub] step 3/8 :: consumer issues locked cheques");
  const cheques: Array<{
    index: number;
    bodyCborHex: string;
    signatureHex: string;
  }> = [];
  for (const cheque of CHEQUES) {
    const locked = await consumerMakeLocked(
      env.wallets.consumer,
      {
        tag,
        index: cheque.index,
        amount: Number(cheque.lovelace),
        durationMs: CHEQUE_DURATION,
        secretHex: SECRET_HEX,
      },
      opts,
    );
    if (locked.isErr()) {
      throw new Error(
        `consumer make locked failed: ${describeError(locked.error)}`,
      );
    }
    console.log(
      `[sub]   cheque #${cheque.index} amount=${cheque.lovelace} timeout_ms=${locked.value.timeout_ms}`,
    );
    cheques.push({
      index: cheque.index,
      bodyCborHex: locked.value.body_cbor_hex,
      signatureHex: locked.value.signature_hex,
    });
  }

  // ---------------------------------------------------------------------
  // 4. Consumer squashes the earlier indices
  // ---------------------------------------------------------------------
  console.log("[sub] step 4/8 :: consumer squashes");
  const squash = await consumerMakeSquash(
    env.wallets.consumer,
    {
      tag,
      amount: Number(SQUASH_LOVELACE),
      index: SQUASH_INDEX,
    },
    opts,
  );
  if (squash.isErr()) {
    throw new Error(
      `consumer make squash failed: ${describeError(squash.error)}`,
    );
  }
  console.log(
    `[sub]   squash amount=${squash.value.amount} index=${squash.value.index}`,
  );

  // ---------------------------------------------------------------------
  // 5. Consumer hands the adaptor a keytag binding them together
  // ---------------------------------------------------------------------
  console.log("[sub] step 5/8 :: resolve the channel keytag");
  const keytag = await consumerShowKeytagJson(env.wallets.consumer, tag, opts);
  if (keytag.isErr()) {
    throw new Error(
      `consumer show keytag failed: ${describeError(keytag.error)}`,
    );
  }
  console.log(`[sub]   keytag=${keytag.value.keytag.slice(0, 16)}…`);

  // ---------------------------------------------------------------------
  // 6. Adaptor executes the sub — this is the actual L1 money movement
  // ---------------------------------------------------------------------
  console.log("[sub] step 6/8 :: adaptor executes the receipt");

  // Persist the exact receipt before submitting. When the node rejects the
  // transaction with an opaque machine error, this is what you need in order to
  // replay it by hand:
  //
  //   konduit-cli adaptor tx --receipt "$(cat <file>)"
  //
  const receiptArg = [
    keytag.value.keytag,
    `${squash.value.body_cbor_hex},${squash.value.signature_hex}`,
    ...cheques.map((c) => `${c.bodyCborHex},${c.signatureHex},${SECRET_HEX}`),
  ].join(";");
  const receiptPath = dumpReceipt(env, tag, receiptArg);

  const receiptResult = await adaptorTxReceipt(
    env.wallets.adaptor,
    [
      {
        keytagHex: keytag.value.keytag,
        squashBodyCborHex: squash.value.body_cbor_hex,
        squashSignatureHex: squash.value.signature_hex,
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
  );
  if (receiptResult.isErr()) {
    throw new Error(
      `adaptor tx receipt failed: ${describeError(receiptResult.error)}\n` +
        `  receipt written to: ${receiptPath}` +
        explainNodeRejection(receiptResult.error),
    );
  }
  console.log(`[sub]   sub tx=${receiptResult.value.tx_id}`);

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
  console.log("[sub] step 8/8 :: verify the adaptor wallet gained lovelace");
  const expectedAdaptorBalance = adaptorBefore.value + EXPECTED_GAIN_LOVELACE;
  const adaptorAfter = await waitForValue(
    "adaptor-balance",
    async () => {
      const r = totalLovelaceAt(env.wallets.adaptor.address, node, {
        skipReferenceScriptUtxos: true,
      });
      return r.isOk() ? r.value : 0n;
    },
    (value) => value >= expectedAdaptorBalance,
    env.defaultTimeoutMs,
    env.defaultPollMs,
  );
  const observedGain = adaptorAfter - adaptorBefore.value;
  console.log(
    `[sub]   adaptor balance ${adaptorBefore.value} -> ${adaptorAfter} (+${observedGain})`,
  );

  console.log(`[sub] ✓ scenario complete (tag=0x${tag})`);

  return {
    tag,
    openTxId: openResult.value.tx_id,
    subTxId: receiptResult.value.tx_id,
    expectedGainLovelace: EXPECTED_GAIN_LOVELACE,
    observedGainLovelace: observedGain,
  };
};

// --- helpers ---------------------------------------------------------------

/**
 * Write the single-line `--receipt` argument next to the other e2e artefacts so
 * a rejected transaction can be replayed by hand. Best effort: never let
 * diagnostics turn a passing scenario into a failing one.
 */
const dumpReceipt = (
  env: KonduitTestEnv,
  tag: string,
  receiptArg: string,
): string => {
  const dir = `${env.repoRoot}/.run/e2e`;
  const path = `${dir}/receipt-${tag}.txt`;
  try {
    fs.mkdirSync(dir, { recursive: true });
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
        `#   konduit-cli adaptor tx --receipt "$(cat ${path})"`,
        "",
        receiptArg,
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

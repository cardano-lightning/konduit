import {
  type ConsumerTipOutput,
  consumerShowTipJson,
  consumerTxAdd,
  consumerTxClose,
  consumerTxOpen,
  describeError,
} from "../konduitCli.js";
import type { KonduitTestEnv } from "../testEnv.js";
import {
  ADA,
  EXEC_FEE_BUDGET_LOVELACE,
  MIN_ADA_BUFFER_LOVELACE,
  MIN_CHANGE_LOVELACE,
  bootstrap,
  explainNodeRejection,
  findFreeTag,
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
 * `consumer show tip`, and every step is resilient: funding and deployment are
 * skipped when they are not needed, and a fresh tag is chosen per run so the
 * scenario never collides with channels left behind by earlier runs.
 */

const OPEN_AMOUNT_LOVELACE = 20n * ADA;
const ADD_AMOUNT_LOVELACE = 80n * ADA;

/**
 * Lovelace the consumer needs *free* for the open and the add. Note that each
 * of those is a separate script transaction carrying roughly a 24 ada execution
 * fee, so the budget is per-transaction, not per-channel.
 */
const CONSUMER_REQUIRED_LOVELACE =
  OPEN_AMOUNT_LOVELACE +
  ADD_AMOUNT_LOVELACE +
  MIN_ADA_BUFFER_LOVELACE +
  2n * EXEC_FEE_BUDGET_LOVELACE +
  MIN_CHANGE_LOVELACE;
const CONSUMER_TOPUP_LOVELACE = 300n * ADA;

const ADMIN_REQUIRED_LOVELACE = 60n * ADA;
const ADMIN_TOPUP_LOVELACE = 200n * ADA;

export type LifecycleScenarioResult = {
  tag: string;
  openTxId: string;
  addTxId: string;
  closeTxId?: string;
};

export const runLifecycle = async (
  env: KonduitTestEnv,
): Promise<LifecycleScenarioResult> => {
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
    ],
  });
  void node;

  // --- open --------------------------------------------------------------
  console.log("[lifecycle] step 1/5 :: open a fresh channel");
  const tag = await findFreeTag(async () =>
    consumerShowTipJson(env.wallets.consumer, opts),
  );
  console.log(`[lifecycle]   tag=0x${tag}`);

  const openResult = await consumerTxOpen(
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
  );
  if (openResult.isErr()) {
    throw new Error(
      `consumer tx open failed: ${describeError(openResult.error)}`,
    );
  }
  console.log(`[lifecycle]   open tx=${openResult.value.tx_id}`);

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

  // --- add ---------------------------------------------------------------
  console.log("[lifecycle] step 2/5 :: add more funds");
  const addResult = await consumerTxAdd(
    env.wallets.consumer,
    [{ tag, ada: Number(ADD_AMOUNT_LOVELACE / ADA) }],
    opts,
  );
  if (addResult.isErr()) {
    throw new Error(
      `consumer tx add failed: ${describeError(addResult.error)}`,
    );
  }
  console.log(`[lifecycle]   add tx=${addResult.value.tx_id}`);

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

  // --- close -------------------------------------------------------------
  console.log("[lifecycle] step 3/5 :: close the channel");
  const closeResult = await consumerTxClose(env.wallets.consumer, [tag], opts);
  if (closeResult.isErr()) {
    throw new Error(
      `consumer tx close failed: ${describeError(closeResult.error)}` +
        explainNodeRejection(closeResult.error),
    );
  }
  console.log(`[lifecycle]   close tx=${closeResult.value.tx_id}`);

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
  console.log(`[lifecycle] ✓ scenario complete (tag=0x${tag})`);

  return {
    tag,
    openTxId: openResult.value.tx_id,
    addTxId: addResult.value.tx_id,
    closeTxId: closeResult.value.tx_id,
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

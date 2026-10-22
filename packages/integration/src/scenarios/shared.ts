import {
  type AdaptorConstantsOutput,
  type AdminTipOutput,
  type ConsumerTipOutput,
  type KonduitCliOptions,
  adaptorShowConstantsJson,
  adminShowTipJson,
  adminTxDeploy,
  describeError,
} from "../konduitCli.js";
import {
  type CardanoNode,
  fundFromFaucet,
  totalLovelaceAt,
} from "../cardanoCli.js";
import type { KonduitTestEnv } from "../testEnv.js";
import { Result } from "neverthrow";

/**
 * @file Primitives shared by the e2e scenarios.
 *
 * Everything here is option-driven: `process.env` is read exclusively by
 * `testEnv.loadTestEnv`, and values are threaded down explicitly from there.
 */

export const ADA = 1_000_000n;
/** `konduit_tx::MIN_ADA_BUFFER` — lovelace the validator reserves per channel. */
export const MIN_ADA_BUFFER_LOVELACE = 2n * ADA;
/** `konduit_tx::FEE_BUFFER` — lovelace the tx builder always keeps for fees. */
export const FEE_BUFFER_LOVELACE = 3n * ADA;
/**
 * Measured cost of the *script execution* portion of a konduit transaction.
 *
 * `Transaction::build` evaluates the Plutus scripts locally and prices the
 * resulting execution units into the fee. On the local `conway_fast` testnet
 * (`priceMemory = 0.0577`, `priceSteps = 0.0000721` lovelace per unit) a single
 * `open` transaction comes out at roughly **24 ada** of fee — orders of
 * magnitude more than the base fee (155,381 + 44/byte).
 *
 * This is why the wallets need far more head-room than the raw amounts being
 * moved suggest: the locked channel value does not help pay for the
 * transaction that spends it, only the consumer's *free* balance does.
 *
 * Measured via `cardano-sdk`'s `with_change`:
 *   `change=10_650_007 fee=23_762_409` → `insufficient balance`.
 */
export const EXEC_FEE_BUDGET_LOVELACE = 30n * ADA;
/** Cardano requires every change output to hold at least `minUTxO` worth of ADA. */
export const MIN_CHANGE_LOVELACE = 3n * ADA;

export const sleep = (ms: number) =>
  new Promise((resolve) => setTimeout(resolve, ms));

// ---------------------------------------------------------------------------
// Unique channel tags
// ---------------------------------------------------------------------------

/**
 * Build an 8-byte (16 hex char) tag: 6 bytes of wall-clock milliseconds plus
 * 2 bytes of salt. `konduit_data::Tag` parses hex, so no `0x` prefix here.
 */
export const uniqueTagHex = (randomSalt: number = 0): string => {
  const nowMs = BigInt(Date.now());
  const millisBytes = Buffer.alloc(6);
  millisBytes.writeUIntBE(Number(nowMs & 0xffffffffffffn), 0, 6);
  const saltBytes = Buffer.alloc(2);
  saltBytes.writeUInt16BE(randomSalt & 0xffff, 0);
  return Buffer.concat([millisBytes, saltBytes]).toString("hex");
};

/**
 * Pick a tag that is not already in use by any channel this consumer owns.
 *
 * Tags are globally scoped per channel on-chain, but we can only cheaply
 * enumerate the channels this particular consumer owns — so a timestamp-derived
 * tag plus a bounded retry is the pragmatic approach. Any residual collision
 * is caught downstream, when `consumer tx --open` would fail or the channel
 * simply would not show up in the tip.
 */
export const findFreeTag = async (
  consumerTip: () => Promise<Result<ConsumerTipOutput, unknown>>,
  maxRetries: number = 8,
): Promise<string> => {
  for (let attempt = 0; attempt < maxRetries; attempt++) {
    const candidate = uniqueTagHex(attempt);
    const tip = await consumerTip();
    if (tip.isErr()) continue;
    const taken = new Set(tip.value.channels.map((c) => c.tag));
    if (!taken.has(candidate)) return candidate;
  }
  throw new Error(
    `findFreeTag: no free 8-byte tag after ${maxRetries} attempts`,
  );
};

// ---------------------------------------------------------------------------
// Polling
// ---------------------------------------------------------------------------

export const waitFor = async <T>(
  label: string,
  fn: () => Promise<Result<T, unknown>>,
  predicate: (value: T) => boolean,
  timeoutMs: number,
  pollMs: number,
): Promise<T> => {
  const start = Date.now();
  let lastResult: Result<T, unknown> | undefined;
  while (Date.now() - start < timeoutMs) {
    const result = await fn();
    lastResult = result;
    if (result.isOk() && predicate(result.value)) return result.value;
    await sleep(pollMs);
  }
  const detail = lastResult
    ? lastResult.isErr()
      ? `last err: ${describeError(lastResult.error)}`
      : `predicate still false: ${describeError(lastResult.value)}`
    : "no attempts completed";
  throw new Error(
    `waitFor(${label}) timed out after ${timeoutMs}ms; ${detail}`,
  );
};

/**
 * Same as {@link waitFor} but for probes that return a plain value rather than
 * a `Result`. Use it when the probe has no meaningful error channel — e.g.
 * `totalLovelaceAt`, where a transient query failure should simply be retried.
 */
export const waitForValue = async <T>(
  label: string,
  fn: () => Promise<T>,
  predicate: (value: T) => boolean,
  timeoutMs: number,
  pollMs: number,
): Promise<T> => {
  const start = Date.now();
  let lastValue: T | undefined;
  while (Date.now() - start < timeoutMs) {
    lastValue = await fn();
    if (predicate(lastValue)) return lastValue;
    await sleep(pollMs);
  }
  throw new Error(
    `waitForValue(${label}) timed out after ${timeoutMs}ms; last value: ${describeError(lastValue)}`,
  );
};

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

const debugEnabled = (): boolean => process.env.KONDUIT_E2E_DEBUG === "1";

export const buildKonduitCliOptions = (
  env: KonduitTestEnv,
): KonduitCliOptions => ({
  binary: env.konduitCliBinary,
  backend: "node",
  nodeSocket: env.cardanoNodeSocket,
  networkId: env.networkId,
  debug: debugEnabled(),
});

export const buildCardanoNode = (env: KonduitTestEnv): CardanoNode => ({
  nodeSocket: env.cardanoNodeSocket,
  networkMagic: env.networkId,
  debug: debugEnabled(),
});

// ---------------------------------------------------------------------------
// Funding
// ---------------------------------------------------------------------------

/**
 * Top a wallet up from the faucet, but only when it holds less than
 * `minLovelace`. Keeps reruns cheap and idempotent.
 */
export const ensureFunded = async (
  env: KonduitTestEnv,
  targetAddr: string,
  node: CardanoNode,
  minLovelace: bigint,
  topupLovelace: bigint,
  label: string,
): Promise<{ funded: boolean; txId?: string }> => {
  const balance = totalLovelaceAt(targetAddr, node, {
    skipReferenceScriptUtxos: true,
  });
  if (balance.isErr()) {
    throw new Error(
      `Failed to query utxos for ${label}: ${String(balance.error)}`,
    );
  }
  if (balance.value >= minLovelace) {
    return { funded: false };
  }
  console.log(
    `[e2e] ${label} has ${balance.value} lovelace (< ${minLovelace}); funding ${topupLovelace} from faucet`,
  );
  const fundingResult = await fundFromFaucet(
    { address: env.faucet.address, skeyFile: env.faucet.skeyFile },
    [{ address: targetAddr, lovelace: topupLovelace }],
    node,
    { timeoutMs: env.defaultTimeoutMs, pollMs: env.defaultPollMs },
  );
  if (fundingResult.isErr()) {
    throw new Error(
      `Funding ${label} from faucet failed: ${fundingResult.error}`,
    );
  }
  return { funded: true, txId: fundingResult.value.txId };
};

// ---------------------------------------------------------------------------
// Reference script bootstrap
// ---------------------------------------------------------------------------

const resolveExpectedScriptHashHex = (): string | undefined =>
  process.env.KONDUIT_VALIDATOR_HASH || undefined;

const waitForReferenceScript = async (
  env: KonduitTestEnv,
  opts: KonduitCliOptions,
  expectedHashHex: string | undefined,
  timeoutMs: number,
  pollMs: number,
): Promise<AdminTipOutput> =>
  waitFor(
    "reference-script",
    async () => adminShowTipJson(env.wallets.admin, opts),
    (tip) => {
      if (!tip.reference_script) return false;
      if (
        expectedHashHex &&
        tip.reference_script.script_hash_hex !== expectedHashHex
      )
        return false;
      return true;
    },
    timeoutMs,
    pollMs,
  );

export const publishReferenceScriptIfNeeded = async (
  env: KonduitTestEnv,
  opts: KonduitCliOptions,
): Promise<{ deployed: boolean; txId?: string }> => {
  const expectedHash = resolveExpectedScriptHashHex();
  const tipResult = await adminShowTipJson(env.wallets.admin, opts);
  if (tipResult.isErr()) {
    throw new Error(
      `admin show tip failed before deploy: ${describeError(tipResult.error)}`,
    );
  }
  if (tipResult.value.reference_script) {
    if (
      expectedHash &&
      tipResult.value.reference_script.script_hash_hex !== expectedHash
    ) {
      throw new Error(
        `Reference script already deployed but its hash ` +
          `(${tipResult.value.reference_script.script_hash_hex}) does not match the expected ${expectedHash}.`,
      );
    }
    return { deployed: false };
  }
  console.log("[e2e] reference script not deployed; running admin tx deploy");
  const deployResult = await adminTxDeploy(env.wallets.admin, opts);
  if (deployResult.isErr()) {
    throw new Error(
      `admin tx deploy failed: ${describeError(deployResult.error)}`,
    );
  }
  const finalTip = await waitForReferenceScript(
    env,
    opts,
    expectedHash,
    env.defaultTimeoutMs,
    env.defaultPollMs,
  );
  if (!finalTip.reference_script) {
    throw new Error(
      "admin show tip still missing reference_script after deploy",
    );
  }
  return { deployed: true, txId: deployResult.value.tx_id };
};

// ---------------------------------------------------------------------------
// Shared bootstrap for every scenario
// ---------------------------------------------------------------------------

export type Bootstrap = {
  opts: KonduitCliOptions;
  node: CardanoNode;
  adaptorConstants: AdaptorConstantsOutput;
  closePeriodMs: number;
};

export type BootstrapOpts = {
  /** Lovelace the admin wallet must hold to be able to deploy the reference script. */
  adminRequiredLovelace: bigint;
  adminTopupLovelace: bigint;
  /** Extra roles (besides admin) to make sure are funded, keyed by role name. */
  fundAlso: Array<{
    role: "consumer" | "adaptor";
    address: string;
    requiredLovelace: bigint;
    topupLovelace: bigint;
  }>;
};

/**
 * Everything every scenario needs before it can touch a channel:
 *  1. the admin wallet is funded,
 *  2. the reference script is deployed (only if it is not already there),
 *  3. the other participating wallets are funded,
 *  4. the adaptor constants (`sub_vkey` + `close_period`) are resolved.
 */
export const bootstrap = async (
  env: KonduitTestEnv,
  bOpts: BootstrapOpts,
): Promise<Bootstrap> => {
  const opts = buildKonduitCliOptions(env);
  const node = buildCardanoNode(env);

  const admin = await ensureFunded(
    env,
    env.wallets.admin.address,
    node,
    bOpts.adminRequiredLovelace,
    bOpts.adminTopupLovelace,
    "admin",
  );
  console.log(
    admin.funded
      ? `[e2e] admin funded (tx=${admin.txId ?? "pending"})`
      : "[e2e] admin already funded",
  );

  const publish = await publishReferenceScriptIfNeeded(env, opts);
  console.log(
    publish.deployed
      ? `[e2e] reference script deployed (tx=${publish.txId})`
      : "[e2e] reference script already present",
  );

  for (const target of bOpts.fundAlso) {
    const res = await ensureFunded(
      env,
      target.address,
      node,
      target.requiredLovelace,
      target.topupLovelace,
      target.role,
    );
    console.log(
      res.funded
        ? `[e2e] ${target.role} funded (tx=${res.txId ?? "pending"})`
        : `[e2e] ${target.role} already funded`,
    );
  }

  const constants = await adaptorShowConstantsJson(env.wallets.adaptor, opts);
  if (constants.isErr()) {
    throw new Error(
      `adaptor show constants failed: ${describeError(constants.error)}`,
    );
  }
  const adaptorConstants = constants.value;
  const closePeriodMs = Number(BigInt(adaptorConstants.close_period_ms));
  console.log(
    `[e2e] adaptor sub_vkey=${adaptorConstants.sub_vkey.slice(0, 12)}… close_period_ms=${adaptorConstants.close_period_ms}`,
  );

  return { opts, node, adaptorConstants, closePeriodMs };
};

// ---------------------------------------------------------------------------
// Node rejection explanation
// ---------------------------------------------------------------------------

/**
 * `konduit-cli` surfaces node rejections verbatim, e.g.
 *
 *   Error: local node rejected transaction: 0x8202818206818201820083028280811992891a0002c78b
 *
 * The blob is a CBOR-encoded Conway machine error from the ledger. Every
 * rejection we have seen decodes to the same shape:
 *
 *   [ 2, [[ 6, [[ 1, [ 0, [ 2, [[], [<steps>]], <mem> ]] ]] ]] ]
 *
 * where `<steps>`/`<mem>` are the script's Aiken `Costs` (mem is always 0). The
 * costs are far below `maxTxExecutionUnits` on this testnet
 * ({ memory: 14_000_000, steps: 10_000_000_000 }), so this is **not** budget
 * exhaustion — the script is failing an assertion and the ledger surfaces only
 * the opaque machine error.
 *
 * We print the raw bytes and the decoded costs rather than guess at a cause.
 */
export const explainNodeRejection = (error: unknown): string => {
  const text = describeError(error);
  const match = text.match(/rejected transaction:\s*0x([0-9a-fA-F]+)/);
  if (!match) return "";
  const raw = match[1];
  const decoded = decodeConwayMachineError(raw);
  return (
    "\n\n  ↳ the local node rejected the transaction with a Conway machine error.\n" +
    `    raw CBOR:  0x${raw}\n` +
    `    shape:     [2, [[6, [[1, [0, [2, [[], [<steps>]], <mem>]]]]]]]\n` +
    (decoded
      ? `    costs:     steps=${decoded.steps} mem=${decoded.mem}\n` +
        "               (well under maxTxExecutionUnits = 14_000_000 mem / 10_000_000_000 steps,\n" +
        "                so this is an assertion failure, not budget exhaustion)\n"
      : "") +
    "    On this branch `consumer tx --open` and `--add` succeed, while `--close`\n" +
    "    and `adaptor tx --receipt` (a Sub) are rejected this way. See\n" +
    "    packages/kernel/lib/konduit/steps/{add,close,sub}.ak."
  );
};

/**
 * Pull the two `Costs` integers out of a machine-error blob. Deliberately
 * narrow: we only trust this exact shape and bail out (returning `null`)
 * rather than guessing at ledger variants we have not verified.
 */
const decodeConwayMachineError = (
  hex: string,
): { steps: number; mem: number } | null => {
  const m = hex.match(
    /^82028182068182018200830282808119([0-9a-f]{4})1a([0-9a-f]{8})$/,
  );
  if (!m) return null;
  return {
    steps: parseInt(m[1]!, 16),
    mem: parseInt(m[2]!, 16),
  };
};

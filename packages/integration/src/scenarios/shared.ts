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
  queryUtxosByAddress,
  transferFunds,
  type TransferFundsError,
  type UtxoMap,
} from "@konduit/cardano-cli";
import {
  AddressBech32,
  NetworkMagicNumber,
} from "@konduit/konduit-consumer/cardano";
import { PositiveBigInt } from "@konduit/codec/integers/big";
import { deserialiseCbor } from "@konduit/codec/cbor/codecs/sync";
import type { Cbor } from "@konduit/codec/cbor/core";
import type { KonduitTestEnv } from "../testEnv.js";
import * as uint8Array from "@konduit/codec/uint8Array";
import { err, ok, Result } from "neverthrow";
import {
  unwrapOrPanic,
  unwrapOrPanicWith,
} from "@konduit/konduit-consumer/neverthrow";

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
 * Budget for the *whole* fee of one script transaction.
 *
 * A konduit transaction carries the validator as a **reference script**, and
 * the ledger charges for every byte of it: 15 lovelace per byte, plus the
 * base fee (44 lovelace per byte of the transaction plus a flat 155 381) and
 * the script's own execution units. The konduit validator is ~5.5 kB, so the
 * reference-script part alone is ~82 000 lovelace.
 *
 * Measured on the local testnet (`conway_fast`): a `sub` transaction costs
 * **319 759 lovelace** — about 0.32 ada — of which ~15 000 is the script's
 * execution units and ~82 000 the reference script.
 *
 * (This used to be budgeted at 30 ada, because the node connector fed the
 * ledger's `adaPerUTxOByte` — 4310, the price of *storing* a utxo — into the
 * reference-script rate. Every transaction was charged 23.6 ada of pure
 * script fee. See `packages/cardano/connector-node/src/params.rs`.)
 *
 * The budget is three times the measurement, so a chain that is a bit more
 * expensive does not fail the scenario — but a *much* more expensive one
 * does, loudly, which is what we want.
 */
export const EXEC_FEE_BUDGET_LOVELACE = 1n * ADA;
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

/** Re-render a structured poll failure from the db layer. */
export const pollFailure = (
  prefix: string,
  failure: {
    reason: string;
    attempts: number;
    elapsedMs: number;
  },
): string =>
  `${prefix} timed out after ${failure.attempts} poll(s) / ${failure.elapsedMs}ms; ` +
  `last seen: ${failure.reason}`;

/** Re-render structured invariant violations from the db layer. */
export const invariantViolations = (
  prefix: string,
  format: (violations: Array<{ invariant: string; detail: string }>) => string,
  found: Array<{ invariant: string; detail: string }>,
): string => `${prefix}\n${format(found)}`;

export const buildKonduitCliOptions = (
  env: KonduitTestEnv,
): KonduitCliOptions => ({
  binary: env.konduitCliBinary,
  backend: "node",
  nodeSocket: env.cardanoNodeSocket,
  networkId: env.networkId,
  debug: env.debug,
});

// ---------------------------------------------------------------------------
// Chain access
// ---------------------------------------------------------------------------

/**
 * Everything the `@konduit/cardano-cli` helpers need to talk to our chain.
 *
 * The node socket is deliberately *not* part of it: the upstream helpers read
 * it from `CARDANO_NODE_SOCKET_PATH`, which `testEnv.loadTestEnv` exports.
 */
export type CardanoContext = {
  /** Network magic of the chain we talk to. */
  networkMagic: NetworkMagicNumber;
  /** Echo every command that is executed. */
  debug: boolean;
};

export const buildCardanoContext = (env: KonduitTestEnv): CardanoContext => {
  const magic = unwrapOrPanic(
    PositiveBigInt.fromBigInt(BigInt(env.networkId)),
    `Invalid network id: ${env.networkId}`,
  );
  return {
    networkMagic: NetworkMagicNumber.fromPositiveBigInt(magic),
    debug: env.debug,
  };
};

/**
 * The UTxOs at `address`, keyed by their `txid#ix` reference.
 *
 * `totalLovelaceAt` only sums the values; keeping the references is what
 * lets a caller attribute an output to the transaction that created it.
 */
export const utxosAt = (
  address: string,
  cardano: CardanoContext,
): Result<UtxoMap, string> => {
  const parsed = AddressBech32.fromString(address);
  if (parsed.isErr()) {
    return err(`Invalid address ${address}: ${parsed.error}`);
  }
  const utxos = queryUtxosByAddress(
    parsed.value,
    cardano.networkMagic,
    cardano.debug,
  );
  if (utxos.isErr()) {
    return err(
      `Failed to query utxos for ${address}: ${describeError(utxos.error)}`,
    );
  }
  return ok(utxos.value);
};

/**
 * The largest spendable UTxO at `address`, in lovelace.
 *
 * The transaction builder pays the fee with the wallet's *largest* UTxO (it
 * selects UTxOs until it has covered `FEE_BUFFER`, which one UTxO does), so
 * this — not the total balance — is what decides whether the wallet can pay
 * for a transaction. A wallet whose value is spread over many small UTxOs
 * can hold more than it can spend.
 */
export const largestUtxoAt = (
  address: string,
  cardano: CardanoContext,
): Result<bigint, string> => {
  const utxos = utxosAt(address, cardano);
  if (utxos.isErr()) {
    return err(utxos.error);
  }
  let largest = 0n;
  for (const utxo of utxos.value.values()) {
    if (utxo.referenceScript !== null) continue;
    const value = BigInt(utxo.value.lovelace);
    if (value > largest) largest = value;
  }
  return ok(largest);
};

/**
 * Total lovelace held at `address`.
 *
 * Reference-script UTxOs are included by default, but callers that want to
 * know what is actually *spendable* can skip them.
 */
export const totalLovelaceAt = (
  address: string,
  cardano: CardanoContext,
  opts: { skipReferenceScriptUtxos?: boolean } = {},
): Result<bigint, string> => {
  const utxos = utxosAt(address, cardano);
  if (utxos.isErr()) {
    return err(utxos.error);
  }
  let total = 0n;
  for (const utxo of utxos.value.values()) {
    if (opts.skipReferenceScriptUtxos && utxo.referenceScript !== null)
      continue;
    total += BigInt(utxo.value.lovelace);
  }
  return ok(total);
};

/** Render a funding failure the way the scenario logs want to read it. */
const describeTransferFundsError = (error: TransferFundsError): string => {
  switch (error.type) {
    case "TooManyRecipients":
      return `too many recipients (${error.requested}, max ${error.max})`;
    case "InsufficientFunds":
      return `insufficient funds: need ${error.requiredLovelace} lovelace, have ${error.availableLovelace}`;
    case "UtxoQueryFailed":
      return `utxo query failed: ${describeError(error.error)}`;
    case "CommandFailed":
      return `${error.step} failed: ${describeError(error.error)}`;
    case "SubmitFailed":
      return `submit failed: ${describeError(error.error)}`;
  }
};

// ---------------------------------------------------------------------------
// Funding
// ---------------------------------------------------------------------------

/**
 * Top a wallet up from the faucet, but only when it cannot pay for the
 * transaction it is about to make. Keeps reruns cheap and idempotent.
 *
 * "Cannot pay" is decided by the wallet's *largest* UTxO, not by its total
 * balance: the transaction builder pays the fee with a single UTxO, so a
 * wallet whose value is spread over many small UTxOs holds more than it can
 * spend (and the transaction then fails with "not enough funds to create a
 * sufficiently large change output").
 *
 * `required` is what the *next* transaction needs from that largest UTxO:
 * the amount it moves (`konduit_tx`'s fuel), the fee, and a change output
 * big enough to be valid. Call this **before every transaction** — the
 * wallet shrinks with each one (the open locks the amount into the channel
 * and pays a fee), so a check made once at the start of a scenario says
 * nothing about the third transaction.
 */
export const ensureFunded = async (
  env: KonduitTestEnv,
  targetAddr: string,
  cardano: CardanoContext,
  required: bigint,
  topupLovelace: bigint,
  label: string,
): Promise<{ funded: boolean; txId?: string }> => {
  const balance = unwrapOrPanic(
    totalLovelaceAt(targetAddr, cardano, {
      skipReferenceScriptUtxos: true,
    }),
    `Failed to query utxos for ${label}`,
  );
  const spendable = unwrapOrPanic(
    largestUtxoAt(targetAddr, cardano),
    `Failed to query utxos for ${label}`,
  );
  if (spendable >= required) {
    return { funded: false };
  }
  console.log(
    `[e2e] ${label} holds ${balance} lovelace, but its largest UTxO is only ` +
      `${spendable} (< ${required}); funding ${topupLovelace} from faucet`,
  );
  const faucet = unwrapOrPanicWith(
    AddressBech32.fromString(env.faucet.address),
    (error) => `Invalid faucet address ${env.faucet.address}: ${error}`,
  );
  const recipient = unwrapOrPanicWith(
    AddressBech32.fromString(targetAddr),
    (error) => `Invalid address ${targetAddr}: ${error}`,
  );
  const fundingResult = unwrapOrPanicWith(
    await transferFunds(
      faucet,
      env.faucet.skeyFile,
      [{ address: recipient, amount: Number(topupLovelace) }],
      cardano.networkMagic,
      null,
      cardano.debug,
    ),
    (error) =>
      `Funding ${label} from faucet failed: ${describeTransferFundsError(error)}`,
  );

  // The transfer is only *submitted* here: the money becomes spendable once
  // the transaction is confirmed and its output shows up in the UTxO set.
  // Building the next transaction before that would pick the wallet's old
  // UTxOs, which may be too small to pay a script transaction's fee, and
  // fail with "not enough funds to create a sufficiently large change
  // output".
  //
  // The top-up lands as a *single* utxo, so it has to be at least as large as
  // what the next transaction needs — otherwise the wait below would never be
  // satisfied.
  if (topupLovelace < required) {
    throw new Error(
      `ensureFunded(${label}): the top-up (${topupLovelace}) is smaller than the ` +
        `amount the next transaction needs (${required}); the top-up lands as a ` +
        `single utxo and would not make the wallet spendable`,
    );
  }
  await waitForValue(
    "funding",
    async () => {
      const funded = largestUtxoAt(targetAddr, cardano);
      return funded.isOk() ? funded.value : 0n;
    },
    (value) => value >= required,
    env.defaultTimeoutMs,
    env.defaultPollMs,
  );

  return { funded: true, txId: fundingResult };
};

// ---------------------------------------------------------------------------
// Reference script bootstrap
// ---------------------------------------------------------------------------

const resolveExpectedScriptHashHex = (
  env: KonduitTestEnv,
): string | undefined => env.validatorHashHex;

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
  const expectedHash = resolveExpectedScriptHashHex(env);
  const tipResult = unwrapOrPanicWith(
    await adminShowTipJson(env.wallets.admin, opts),
    (error) => `admin show tip failed before deploy: ${describeError(error)}`,
  );
  if (tipResult.reference_script) {
    if (
      expectedHash &&
      tipResult.reference_script.script_hash_hex !== expectedHash
    ) {
      throw new Error(
        `Reference script already deployed but its hash ` +
          `(${tipResult.reference_script.script_hash_hex}) does not match the expected ${expectedHash}.`,
      );
    }
    return { deployed: false };
  }
  console.log("[e2e] reference script not deployed; running admin tx deploy");
  const deployResult = unwrapOrPanicWith(
    await adminTxDeploy(env.wallets.admin, opts),
    (error) => `admin tx deploy failed: ${describeError(error)}`,
  );
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
  return { deployed: true, txId: deployResult.tx_id };
};

// ---------------------------------------------------------------------------
// Shared bootstrap for every scenario
// ---------------------------------------------------------------------------

export type Bootstrap = {
  opts: KonduitCliOptions;
  cardano: CardanoContext;
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
  const cardano = buildCardanoContext(env);

  const admin = await ensureFunded(
    env,
    env.wallets.admin.address,
    cardano,
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
      cardano,
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

  const adaptorConstants = unwrapOrPanicWith(
    await adaptorShowConstantsJson(env.wallets.adaptor, opts),
    (error) => `adaptor show constants failed: ${describeError(error)}`,
  );
  const closePeriodMs = Number(BigInt(adaptorConstants.close_period_ms));
  console.log(
    `[e2e] adaptor sub_vkey=${adaptorConstants.sub_vkey.slice(0, 12)}… close_period_ms=${adaptorConstants.close_period_ms}`,
  );

  return { opts, cardano, adaptorConstants, closePeriodMs };
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
 * where `<steps>`/`<mem>` are the script's Aiken `Costs`. The costs are far
 * below `maxTxExecutionUnits` on this testnet
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
    "    The ledger only reports the costs the script burned before it gave up,\n" +
    "    so the failure is inside the script — see\n" +
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
  const bytes = uint8Array.stringCodec.deserialise(hex);
  if (bytes.isErr()) return null;
  const decoded = deserialiseCbor(bytes.value);
  if (decoded.isErr()) return null;

  const costs = findCosts(decoded.value);
  if (costs === null) return null;
  return { steps: Number(costs.steps), mem: Number(costs.mem) };
};

/**
 * Find the script's costs inside a Conway machine error.
 *
 * The ledger encodes them as the last three items of the innermost list:
 * an empty list, a single-element list holding the step count, and the
 * memory usage — `[[], [steps], mem]`.
 *
 * The counts are CBOR unsigned integers whose *width* is the encoder's
 * choice (one byte for a small cost, four for a large one), so the blob
 * cannot be matched with a fixed-width pattern; it has to be parsed.
 */
const findCosts = (value: Cbor): { steps: bigint; mem: bigint } | null => {
  const items = asArray(value);
  if (items === null) return null;

  if (items.length === 3) {
    const [head, steps, mem] = items;
    const stepItems = asArray(steps);
    if (
      Array.isArray(head) &&
      head.length === 0 &&
      stepItems !== null &&
      stepItems.length === 1
    ) {
      const stepCount = asUint(stepItems[0]!);
      const memory = asUint(mem);
      if (stepCount !== null && memory !== null) {
        return { steps: stepCount, mem: memory };
      }
    }
  }

  for (const item of items) {
    const found = findCosts(item);
    if (found !== null) return found;
  }
  return null;
};

/** A CBOR list, or `null` for anything else. */
const asArray = (value: Cbor): Cbor[] | null =>
  Array.isArray(value) ? value : null;

/** A CBOR unsigned integer, or `null` for anything else. */
const asUint = (value: Cbor): bigint | null =>
  typeof value === "bigint" ? value : null;

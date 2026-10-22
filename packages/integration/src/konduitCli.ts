import { execFileSync } from "node:child_process";
import {
  type CliArg,
  type CliArgs,
  type CommandError,
  type Path as TaggedPath,
  buildCliCommand,
} from "@konduit/cardano-cli/exec";
import { err, ok, Result } from "neverthrow";

/**
 * @file Wrapper around the `konduit-cli` binary.
 *
 * This wrapper is intentionally **env-free**: every call is driven by the
 * values passed via `KonduitCliOptions` + `RoleWallet`. Wallets, addresses,
 * sockets etc. are never read from `process.env` here — that responsibility
 * belongs to the test entry point, which loads `.env*` files once and then
 * threads the values downward through options.
 *
 * Returns follow the `Result<T, CommandError | JsonError>` pattern from
 * `neverthrow`. Callers must not panic — they should compose with `.match` /
 * `.andThen`. The motivation is to mirror `marlowe-integration/app/src/marloweCli.ts`
 * so the surrounding ecosystem stays consistent.
 */

// ---------------------------------------------------------------------------
// Public re-exports of plumbing types so callers don't need to depend on
// @konduit/cardano-cli directly.
// ---------------------------------------------------------------------------

export type { CliArg, CliArgs, CommandError };
export type Path = string;

export type Role = "admin" | "consumer" | "adaptor";

export type KonduitBackend = "node" | "blockfrost" | "utxorpc";

export type KonduitCliOptions = {
  /** Absolute path to a built `konduit-cli` binary. */
  binary: string;
  /** Cardano backend to use for the spawned process. */
  backend: KonduitBackend;
  /** Required when `backend === 'node'`. */
  nodeSocket?: string;
  /** Cardano network id (`42` for the local dev testnet) or `mainnet`/`preprod`/`preview`. */
  networkId: number | "mainnet" | "preprod" | "preview";
  /** Required when `backend === 'blockfrost'`. */
  blockfrostProjectId?: string;
  /** Required when `backend === 'utxorpc'`. */
  utxorpcUri?: string;
  /** Optional extra env-vars injected on every invocation. */
  extraEnv?: Record<string, string>;
  /** Echo every command line being executed. */
  debug?: boolean;
};

/** Per-role wallet material — passed exclusively through options (never read from env). */
export type RoleWallet = {
  /** 32-byte signing key as hex (no `5820`/`x` prefix). */
  skeyHex: string;
  /** Bech32 address hosting the reference script deployment. */
  hostAddress: string;
  /** Adaptor-only: close period in milliseconds (default: 24h). */
  closePeriodMs?: number;
  /** Adaptor-only: flat fee in lovelace (default: 10 000). */
  feeLovelace?: number;
};

// ---------------------------------------------------------------------------
// Internal: subprocess + env plumbing.
// ---------------------------------------------------------------------------

const KONDUIT_CLOSE_PERIOD_DEFAULT_MS = 24 * 60 * 60 * 1000;
const KONDUIT_FEE_DEFAULT_LOVELACE = 10_000;

const buildRoleEnv = (
  role: Role,
  wallet: RoleWallet,
  opts: KonduitCliOptions,
): Record<string, string> => {
  const env: Record<string, string> = {
    KONDUIT_CARDANO_BACKEND: opts.backend,
    KONDUIT_NETWORK: String(opts.networkId),
    KONDUIT_WALLET: wallet.skeyHex,
    KONDUIT_HOST_ADDRESS: wallet.hostAddress,
    ...(opts.extraEnv ?? {}),
  };
  if (opts.backend === "node") {
    if (!opts.nodeSocket)
      throw new Error(
        'KonduitCliOptions.nodeSocket is required for backend="node"',
      );
    env.KONDUIT_NODE_SOCKET = opts.nodeSocket;
  } else if (opts.backend === "blockfrost") {
    if (opts.blockfrostProjectId)
      env.KONDUIT_BLOCKFROST_PROJECT_ID = opts.blockfrostProjectId;
  } else if (opts.backend === "utxorpc") {
    if (opts.utxorpcUri) env.KONDUIT_UTXORPC_URI = opts.utxorpcUri;
  }
  if (role === "adaptor") {
    env.KONDUIT_CLOSE_PERIOD = `${wallet.closePeriodMs ?? KONDUIT_CLOSE_PERIOD_DEFAULT_MS}ms`;
    env.KONDUIT_FEE = String(
      wallet.feeLovelace ?? KONDUIT_FEE_DEFAULT_LOVELACE,
    );
  }
  return env;
};

const invokeWithEnv = (
  cmd: string,
  env: Record<string, string>,
  debug: boolean,
): Result<string, CommandError> => {
  if (debug) {
    console.debug(`[konduitCli] $ ${cmd}`);
    console.debug(`[konduitCli] env=${JSON.stringify(env)}`);
  }
  try {
    const stdout = execFileSync("/bin/sh", ["-c", cmd, "--"], {
      env: { ...process.env, ...env },
      encoding: "utf-8",
      stdio: ["ignore", "pipe", "pipe"],
      maxBuffer: 100 * 1024 * 1024,
    });
    return ok(stdout.toString().trim());
  } catch (e: any) {
    return err({
      type: "CommandError",
      status: BigInt(e.status ?? e.code ?? 0),
      stdout: (e.stdout ?? "").toString(),
      stderr: (e.stderr ?? "").toString(),
    });
  }
};

const runKonduitRoleCli = (
  role: Role,
  args: CliArgs,
  wallet: RoleWallet,
  opts: KonduitCliOptions,
): Result<string, CommandError> => {
  const env = buildRoleEnv(role, wallet, opts);
  // Reuse the shell-safe builder from `@konduit/cardano-cli/exec` for the
  // argument serialisation so we stay consistent with the rest of the
  // integration suite.
  const cmd = buildCliCommand(`${opts.binary} ${role}`, args);
  return invokeWithEnv(cmd, env, opts.debug ?? false);
};

// ---------------------------------------------------------------------------
// Internal: minimal `Json` deserialisation helpers.
//
// We intentionally avoid depending on the *root* entry of `@konduit/cardano-cli`
// (which transitively imports `@konduit/codec` and `@konduit/konduit-consumer`,
// neither of which is installed in this indexer workspace). Each decoder here
// returns a `Result<T, JsonError>` to mirror the codec API of those packages.
// ---------------------------------------------------------------------------

export type Json =
  | string
  | number
  | boolean
  | null
  | Json[]
  | { [key: string]: Json };
export type JsonError = string;

/**
 * `JSON.stringify` throws on `BigInt`, and `CommandError.status` is a
 * `bigint`. Scenario code wants to embed errors into messages, so provide a
 * stringify that is total.
 */
export const describeError = (error: unknown): string => {
  if (typeof error === "string") return error;
  if (typeof error !== "object" || error === null) return String(error);
  const e = error as Record<string, unknown>;
  if (e.type === "CommandError") {
    const status = String(e.status);
    const stderr = typeof e.stderr === "string" ? e.stderr.trim() : "";
    const stdout = typeof e.stdout === "string" ? e.stdout.trim() : "";
    return `CommandError(status=${status})\n--- stderr ---\n${stderr}\n--- stdout ---\n${stdout}`;
  }
  try {
    return JSON.stringify(error, (_k, v) =>
      typeof v === "bigint" ? `${v.toString()}n` : v,
    );
  } catch {
    return String(error);
  }
};

const isObject = (raw: unknown): raw is Record<string, Json> =>
  typeof raw === "object" && raw !== null && !Array.isArray(raw);

const expectString = (raw: Json, where: string): Result<string, JsonError> => {
  if (typeof raw !== "string")
    return err(`${where}: expected string, got ${typeof raw}`);
  return ok(raw);
};

const expectNumber = (raw: Json, where: string): Result<number, JsonError> => {
  if (typeof raw !== "number")
    return err(`${where}: expected number, got ${typeof raw}`);
  return ok(raw);
};

const expectObject = (
  raw: Json,
  where: string,
): Result<Record<string, Json>, JsonError> => {
  if (!isObject(raw))
    return err(`${where}: expected object, got ${typeof raw}`);
  return ok(raw);
};

const expectNullableObject = (
  raw: Json,
  where: string,
): Result<Record<string, Json> | null, JsonError> => {
  if (raw === null) return ok(null);
  if (!isObject(raw))
    return err(`${where}: expected object|null, got ${typeof raw}`);
  return ok(raw);
};

const decodeJson = (
  text: string,
  decoder: (raw: Json) => Result<unknown, JsonError>,
): Result<unknown, CommandError | JsonError> => {
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch (e: any) {
    return err(`json: failed to parse: ${(e as Error).message ?? String(e)}`);
  }
  return decoder(raw as Json).mapErr((msg) => msg as JsonError);
};

// ---------------------------------------------------------------------------
// Output DTOs — mirror the shapes in `packages/konduit/cli/src/output.rs`.
// ---------------------------------------------------------------------------

export type AddressOutput = { address: string };
export type AdaptorConstantsOutput = {
  sub_vkey: string;
  close_period_ms: string;
};
export type KeytagOutput = {
  keytag: string;
  verification_key: string;
  tag: string;
};
export type SquashOutput = {
  body_cbor_hex: string;
  signature_hex: string;
  amount: number;
  index: number;
};
export type LockedOutput = {
  body_cbor_hex: string;
  signature_hex: string;
  lock_hex: string;
  index: number;
  amount: number;
  timeout_ms: number;
};
export type TxOutput = { tx_id: string };

export type ReferenceScriptDto = {
  tx_id_hex: string;
  output_index: number;
  script_version: string;
  script_hash_hex: string;
};

export type ChannelStage =
  | {
      label: "opened";
      subbed: number;
      useds: Array<{ index: number; amount: number }>;
    }
  | {
      label: "closed";
      subbed: number;
      useds: Array<{ index: number; amount: number }>;
      elapse_at_ms: number;
    }
  | {
      label: "responded";
      pendings_amount: number;
      pendings: Array<{ amount: number; timeout_ms: number; lock_hex: string }>;
    };

export type ChannelOutput = {
  tag: string;
  add_vkey: string;
  sub_vkey: string;
  close_period_ms: number;
  amount_lovelace: number;
  buffered_lovelace: number;
  keytag_hex: string;
  stage: ChannelStage;
};

export type WalletSummary = {
  utxo_count: number;
  total_lovelace: number;
  reference_script: ReferenceScriptDto | null;
};

export type AdminTipOutput = {
  wallet: WalletSummary;
  reference_script: ReferenceScriptDto | null;
};
export type ConsumerTipOutput = {
  wallet: WalletSummary;
  reference_script: ReferenceScriptDto | null;
  channels: ChannelOutput[];
};
export type AdaptorTipOutput = {
  wallet: WalletSummary;
  reference_script: ReferenceScriptDto | null;
  channels: ChannelOutput[];
};

const decodeReferenceScript = (
  raw: Json,
  where: string,
): Result<ReferenceScriptDto, JsonError> =>
  expectObject(raw, where).andThen((o) =>
    Result.combine([
      expectString(o.tx_id_hex ?? null, `${where}.tx_id_hex`),
      expectNumber(o.output_index ?? null, `${where}.output_index`),
    ]).map(([tx_id_hex, output_index]) => ({
      tx_id_hex,
      output_index,
      script_version: String(o.script_version ?? ""),
      script_hash_hex: String(o.script_hash_hex ?? ""),
    })),
  );

const decodeWalletSummary = (
  raw: Json,
  where: string,
): Result<WalletSummary, JsonError> =>
  expectObject(raw, where).andThen((o) => {
    const refResult = expectNullableObject(
      o.reference_script ?? null,
      `${where}.reference_script`,
    ).andThen((maybe) =>
      maybe === null
        ? ok(null)
        : decodeReferenceScript(maybe, `${where}.reference_script`),
    );
    return refResult.andThen((reference_script) =>
      Result.combine([
        expectNumber(o.utxo_count ?? null, `${where}.utxo_count`),
        expectNumber(o.total_lovelace ?? null, `${where}.total_lovelace`),
      ]).map(([utxo_count, total_lovelace]) => ({
        utxo_count,
        total_lovelace,
        reference_script,
      })),
    );
  });

const decodeChannelStage = (
  raw: Json,
  where: string,
): Result<ChannelStage, JsonError> => {
  if (!isObject(raw))
    return err(`${where}: expected object, got ${typeof raw}`);
  const obj = raw;
  switch (obj.label) {
    case "opened":
      return Result.combine([
        expectNumber(obj.subbed ?? null, `${where}.subbed`),
        ok(Array.isArray(obj.useds) ? obj.useds : []),
      ]).map(([subbed, usedsRaw]) => ({
        label: "opened" as const,
        subbed,
        useds: usedsRaw.map((u, i) => {
          if (!isObject(u))
            throw new Error(`${where}.useds[${i}] expected object`);
          return {
            index: Number(u.index ?? 0),
            amount: Number(u.amount ?? 0),
          };
        }),
      }));
    case "closed":
      return Result.combine([
        expectNumber(obj.subbed ?? null, `${where}.subbed`),
        expectNumber(obj.elapse_at_ms ?? null, `${where}.elapse_at_ms`),
        ok(Array.isArray(obj.useds) ? obj.useds : []),
      ]).map(([subbed, elapse_at_ms, usedsRaw]) => ({
        label: "closed" as const,
        subbed,
        elapse_at_ms,
        useds: usedsRaw.map((u, i) => {
          if (!isObject(u))
            throw new Error(`${where}.useds[${i}] expected object`);
          return {
            index: Number(u.index ?? 0),
            amount: Number(u.amount ?? 0),
          };
        }),
      }));
    case "responded":
      return Result.combine([
        expectNumber(obj.pendings_amount ?? null, `${where}.pendings_amount`),
        ok(Array.isArray(obj.pendings) ? obj.pendings : []),
      ]).map(([pendings_amount, pendingsRaw]) => ({
        label: "responded" as const,
        pendings_amount,
        pendings: pendingsRaw.map((p, i) => {
          if (!isObject(p))
            throw new Error(`${where}.pendings[${i}] expected object`);
          return {
            amount: Number(p.amount ?? 0),
            timeout_ms: Number(p.timeout_ms ?? 0),
            lock_hex: String(p.lock_hex ?? ""),
          };
        }),
      }));
    default:
      return err(`${where}: unknown channel stage label: ${String(obj.label)}`);
  }
};

const decodeChannel = (
  raw: Json,
  where: string,
): Result<ChannelOutput, JsonError> =>
  expectObject(raw, where).andThen((o) =>
    decodeChannelStage(o.stage ?? null, `${where}.stage`).map((stage) => ({
      tag: String(o.tag ?? ""),
      add_vkey: String(o.add_vkey ?? ""),
      sub_vkey: String(o.sub_vkey ?? ""),
      close_period_ms: Number(o.close_period_ms ?? 0),
      amount_lovelace: Number(o.amount_lovelace ?? 0),
      buffered_lovelace: Number(o.buffered_lovelace ?? 0),
      keytag_hex: String(o.keytag_hex ?? ""),
      stage,
    })),
  );

const decodeChannels = (
  raw: Json,
  where: string,
): Result<ChannelOutput[], JsonError> => {
  if (!Array.isArray(raw))
    return err(`${where}: expected array, got ${typeof raw}`);
  const channels: ChannelOutput[] = [];
  for (let i = 0; i < raw.length; i++) {
    const ch = raw[i] as Json;
    const r = decodeChannel(ch, `${where}[${i}]`);
    if (r.isErr()) return err(r.error);
    channels.push(r.value);
  }
  return ok(channels);
};

const decodeAdminTip = (raw: Json): Result<AdminTipOutput, JsonError> =>
  expectObject(raw, "adminTip").andThen((o) =>
    expectNullableObject(
      o.reference_script ?? null,
      "adminTip.reference_script",
    )
      .andThen((maybe) =>
        maybe === null
          ? ok(null)
          : decodeReferenceScript(maybe, "adminTip.reference_script"),
      )
      .andThen((reference_script) =>
        decodeWalletSummary(o.wallet ?? null, "adminTip.wallet").map(
          (wallet) => ({
            wallet,
            reference_script,
          }),
        ),
      ),
  );

const decodeConsumerTip = (raw: Json): Result<ConsumerTipOutput, JsonError> =>
  expectObject(raw, "consumerTip").andThen((o) => {
    const refResult = expectNullableObject(
      o.reference_script ?? null,
      "consumerTip.reference_script",
    ).andThen((maybe) =>
      maybe === null
        ? ok(null)
        : decodeReferenceScript(maybe, "consumerTip.reference_script"),
    );
    return Result.combine([
      refResult,
      decodeWalletSummary(o.wallet ?? null, "consumerTip.wallet"),
    ]).andThen(([reference_script, wallet]) =>
      decodeChannels(o.channels ?? null, "consumerTip.channels").map(
        (channels) => ({
          wallet,
          reference_script,
          channels,
        }),
      ),
    );
  });

const decodeAdaptorTip = (raw: Json): Result<AdaptorTipOutput, JsonError> =>
  expectObject(raw, "adaptorTip").andThen((o) => {
    const refResult = expectNullableObject(
      o.reference_script ?? null,
      "adaptorTip.reference_script",
    ).andThen((maybe) =>
      maybe === null
        ? ok(null)
        : decodeReferenceScript(maybe, "adaptorTip.reference_script"),
    );
    return Result.combine([
      refResult,
      decodeWalletSummary(o.wallet ?? null, "adaptorTip.wallet"),
    ]).andThen(([reference_script, wallet]) =>
      decodeChannels(o.channels ?? null, "adaptorTip.channels").map(
        (channels) => ({
          wallet,
          reference_script,
          channels,
        }),
      ),
    );
  });

const decodeAddressOutput = (raw: Json): Result<AddressOutput, JsonError> =>
  expectObject(raw, "addressOutput").andThen((o) =>
    expectString(o.address ?? null, "addressOutput.address").map((address) => ({
      address,
    })),
  );

const decodeAdaptorConstants = (
  raw: Json,
): Result<AdaptorConstantsOutput, JsonError> =>
  expectObject(raw, "adaptorConstants").andThen((o) =>
    expectString(o.sub_vkey ?? null, "adaptorConstants.sub_vkey").andThen(
      (sub_vkey) => {
        // `close_period_ms` is emitted as a JSON number by konduit-cli.
        const ms = o.close_period_ms;
        if (typeof ms === "number")
          return ok({ sub_vkey, close_period_ms: String(ms) });
        if (typeof ms === "string")
          return ok({ sub_vkey, close_period_ms: ms });
        return err(
          `adaptorConstants.close_period_ms: expected string|number, got ${typeof ms}`,
        );
      },
    ),
  );

const decodeKeytag = (raw: Json): Result<KeytagOutput, JsonError> =>
  expectObject(raw, "keytag").andThen((o) =>
    Result.combine([
      expectString(o.keytag ?? null, "keytag.keytag"),
      expectString(o.verification_key ?? null, "keytag.verification_key"),
      expectString(o.tag ?? null, "keytag.tag"),
    ]).map(([keytag, verification_key, tag]) => ({
      keytag,
      verification_key,
      tag,
    })),
  );

const decodeSquash = (raw: Json): Result<SquashOutput, JsonError> =>
  expectObject(raw, "squash").andThen((o) =>
    Result.combine([
      expectString(o.body_cbor_hex ?? null, "squash.body_cbor_hex"),
      expectString(o.signature_hex ?? null, "squash.signature_hex"),
      expectNumber(o.amount ?? null, "squash.amount"),
      expectNumber(o.index ?? null, "squash.index"),
    ]).map(([body_cbor_hex, signature_hex, amount, index]) => ({
      body_cbor_hex,
      signature_hex,
      amount,
      index,
    })),
  );

const decodeLocked = (raw: Json): Result<LockedOutput, JsonError> =>
  expectObject(raw, "locked").andThen((o) =>
    Result.combine([
      expectString(o.body_cbor_hex ?? null, "locked.body_cbor_hex"),
      expectString(o.signature_hex ?? null, "locked.signature_hex"),
      expectString(o.lock_hex ?? null, "locked.lock_hex"),
      expectNumber(o.index ?? null, "locked.index"),
      expectNumber(o.amount ?? null, "locked.amount"),
      expectNumber(o.timeout_ms ?? null, "locked.timeout_ms"),
    ]).map(
      ([
        body_cbor_hex,
        signature_hex,
        lock_hex,
        index,
        amount,
        timeout_ms,
      ]) => ({
        body_cbor_hex,
        signature_hex,
        lock_hex,
        index,
        amount,
        timeout_ms,
      }),
    ),
  );

const decodeTxOutput = (raw: Json): Result<TxOutput, JsonError> =>
  expectObject(raw, "txOutput").andThen((o) =>
    expectString(o.tx_id ?? null, "txOutput.tx_id").map((tx_id) => ({ tx_id })),
  );

// ---------------------------------------------------------------------------
// JSON-emitting command runner.
// ---------------------------------------------------------------------------

const runJson = <T>(
  role: Role,
  args: CliArgs,
  wallet: RoleWallet,
  opts: KonduitCliOptions,
  decoder: (raw: Json) => Result<T, JsonError>,
): Result<T, CommandError | JsonError> => {
  const stdoutResult = runKonduitRoleCli(role, args, wallet, opts);
  if (stdoutResult.isErr()) return err(stdoutResult.error);
  const decoded = decodeJson(stdoutResult.value, decoder);
  return decoded as Result<T, CommandError | JsonError>;
};

// ---------------------------------------------------------------------------
// Admin
// ---------------------------------------------------------------------------

export const adminShowAddressJson = (
  wallet: RoleWallet,
  opts: KonduitCliOptions,
): Result<AddressOutput, CommandError | JsonError> =>
  runJson(
    "admin",
    ["show", "address", ["--output-json", true]],
    wallet,
    opts,
    decodeAddressOutput,
  );

export const adminShowTipJson = (
  wallet: RoleWallet,
  opts: KonduitCliOptions,
): Result<AdminTipOutput, CommandError | JsonError> =>
  runJson(
    "admin",
    ["show", "tip", ["--output-json", true]],
    wallet,
    opts,
    decodeAdminTip,
  );

export const adminTxDeploy = (
  wallet: RoleWallet,
  opts: KonduitCliOptions,
): Result<TxOutput, CommandError | JsonError> =>
  runJson(
    "admin",
    ["tx", "deploy", ["--output-json", true]],
    wallet,
    opts,
    decodeTxOutput,
  );

export type AdminSendRecipient = {
  /** Bech32 address (shelley). */
  address: string;
  /** Amount in **ada** (the CLI multiplies internally by `ADA`). */
  ada: number;
};

export const adminTxSend = (
  wallet: RoleWallet,
  recipients: AdminSendRecipient[],
  opts: KonduitCliOptions,
): Result<TxOutput, CommandError | JsonError> =>
  runJson(
    "admin",
    [
      "tx",
      "send",
      ["--output-json", true],
      ...recipients.map((r): CliArg => ["--to", `${r.address},${r.ada}`]),
    ],
    wallet,
    opts,
    decodeTxOutput,
  );

// ---------------------------------------------------------------------------
// Consumer
// ---------------------------------------------------------------------------

export const consumerShowAddressJson = (
  wallet: RoleWallet,
  opts: KonduitCliOptions,
): Result<AddressOutput, CommandError | JsonError> =>
  runJson(
    "consumer",
    ["show", "address", ["--output-json", true]],
    wallet,
    opts,
    decodeAddressOutput,
  );

export const consumerShowTipJson = (
  wallet: RoleWallet,
  opts: KonduitCliOptions,
): Result<ConsumerTipOutput, CommandError | JsonError> =>
  runJson(
    "consumer",
    ["show", "tip", ["--output-json", true]],
    wallet,
    opts,
    decodeConsumerTip,
  );

export const consumerShowKeytagJson = (
  wallet: RoleWallet,
  tag: string,
  opts: KonduitCliOptions,
): Result<KeytagOutput, CommandError | JsonError> =>
  runJson(
    "consumer",
    ["show", "keytag", tag, ["--output-json", true]],
    wallet,
    opts,
    decodeKeytag,
  );

export type ConsumerOpenRequest = {
  tag: string;
  /** Hex-encoded adaptor verification key (32 bytes, 64 hex chars). */
  subVkeyHex: string;
  /** Close period in milliseconds — must match the adaptor's value. */
  closePeriodMs: number;
  /** Amount in **ada**. */
  ada: number;
};

export const consumerTxOpen = (
  wallet: RoleWallet,
  requests: ConsumerOpenRequest[],
  opts: KonduitCliOptions,
): Result<TxOutput, CommandError | JsonError> =>
  runJson(
    "consumer",
    [
      "tx",
      ["--output-json", true],
      ...requests.map(
        (r): CliArg => [
          "--open",
          `${r.tag},${r.subVkeyHex},${r.closePeriodMs}ms,${r.ada}`,
        ],
      ),
    ],
    wallet,
    opts,
    decodeTxOutput,
  );

export type ConsumerAddRequest = { tag: string; ada: number };

export const consumerTxAdd = (
  wallet: RoleWallet,
  requests: ConsumerAddRequest[],
  opts: KonduitCliOptions,
): Result<TxOutput, CommandError | JsonError> =>
  runJson(
    "consumer",
    [
      "tx",
      ["--output-json", true],
      ...requests.map((r): CliArg => ["--add", `${r.tag},${r.ada}`]),
    ],
    wallet,
    opts,
    decodeTxOutput,
  );

export const consumerTxClose = (
  wallet: RoleWallet,
  tags: string[],
  opts: KonduitCliOptions,
): Result<TxOutput, CommandError | JsonError> =>
  runJson(
    "consumer",
    ["tx", ["--output-json", true], ...tags.map((t): CliArg => ["--close", t])],
    wallet,
    opts,
    decodeTxOutput,
  );

export const consumerMakeSquash = (
  wallet: RoleWallet,
  args: { tag: string; amount: number; index: number; exclude?: number[] },
  opts: KonduitCliOptions,
): Result<SquashOutput, CommandError | JsonError> =>
  runJson(
    "consumer",
    [
      "make",
      "squash",
      ["--tag", args.tag],
      ["--amount", args.amount],
      ["--index", args.index],
      ["--exclude", args.exclude?.join(",") ?? ""],
      ["--output-json", true],
    ],
    wallet,
    opts,
    decodeSquash,
  );

/**
 * `--timeout` / `--duration` accept either an absolute POSIX offset
 * (`timeoutMs`) or a value relative to now (`durationMs`). The latter is what
 * you normally want for a cheque, because the adaptor validates the cheque
 * against the tx's own upper bound, which is also computed from `now`.
 *
 * The suffix form (`'1h'`, `'30min'`, `'5000ms'`) is accepted by
 * `konduit_data::Duration`'s `FromStr`, so string durations are passed through
 * verbatim; numeric durations are converted to an explicit `ms` suffix.
 */
const durationArg = (ms: number): [string, string] => ["--duration", `${ms}ms`];

export const consumerMakeLocked = (
  wallet: RoleWallet,
  args: {
    tag: string;
    index: number;
    amount: number;
    timeoutMs?: number;
    durationMs?: number | string;
    secretHex: string;
  },
  opts: KonduitCliOptions,
): Result<LockedOutput, CommandError | JsonError> => {
  if (args.timeoutMs === undefined && args.durationMs === undefined) {
    return err("locked: either timeoutMs or durationMs is required");
  }
  const cliArgs: CliArgs = [
    "make",
    "locked",
    ["--tag", args.tag],
    ["--index", args.index],
    ["--amount", args.amount],
  ];
  if (args.timeoutMs !== undefined)
    cliArgs.push(["--timeout", `${args.timeoutMs}ms`]);
  if (args.durationMs !== undefined) {
    cliArgs.push(
      typeof args.durationMs === "string"
        ? ["--duration", args.durationMs]
        : durationArg(args.durationMs),
    );
  }
  cliArgs.push(["--secret", args.secretHex], ["--output-json", true]);
  return runJson("consumer", cliArgs, wallet, opts, decodeLocked);
};

// ---------------------------------------------------------------------------
// Adaptor
// ---------------------------------------------------------------------------

export const adaptorShowAddressJson = (
  wallet: RoleWallet,
  opts: KonduitCliOptions,
): Result<AddressOutput, CommandError | JsonError> =>
  runJson(
    "adaptor",
    ["show", "address", ["--output-json", true]],
    wallet,
    opts,
    decodeAddressOutput,
  );

export const adaptorShowConstantsJson = (
  wallet: RoleWallet,
  opts: KonduitCliOptions,
): Result<AdaptorConstantsOutput, CommandError | JsonError> =>
  runJson(
    "adaptor",
    ["show", "constants", ["--output-json", true]],
    wallet,
    opts,
    decodeAdaptorConstants,
  );

export const adaptorShowTipJson = (
  wallet: RoleWallet,
  opts: KonduitCliOptions,
): Result<AdaptorTipOutput, CommandError | JsonError> =>
  runJson(
    "adaptor",
    ["show", "tip", ["--output-json", true]],
    wallet,
    opts,
    decodeAdaptorTip,
  );

export type AdaptorReceiptRequest = {
  keytagHex: string;
  squashBodyCborHex: string;
  squashSignatureHex: string;
  cheques: Array<
    | { bodyCborHex: string; signatureHex: string }
    | { bodyCborHex: string; signatureHex: string; secretHex: string }
  >;
};

export const adaptorTxReceipt = (
  wallet: RoleWallet,
  receipts: AdaptorReceiptRequest[],
  opts: KonduitCliOptions,
): Result<TxOutput, CommandError | JsonError> => {
  const receiptArgs: CliArgs = receipts.map((r): CliArg => {
    const chequeSegments = r.cheques.map((c) =>
      "secretHex" in c
        ? `${c.bodyCborHex},${c.signatureHex},${c.secretHex}`
        : `${c.bodyCborHex},${c.signatureHex}`,
    );
    return [
      "--receipt",
      [
        r.keytagHex,
        `${r.squashBodyCborHex},${r.squashSignatureHex}`,
        ...chequeSegments,
      ].join(";"),
    ];
  });
  return runJson(
    "adaptor",
    ["tx", ["--output-json", true], ...receiptArgs],
    wallet,
    opts,
    decodeTxOutput,
  );
};

// Re-export `TaggedPath` so callers do not need a separate import from
// `@konduit/cardano-cli/exec`.
export type { TaggedPath };

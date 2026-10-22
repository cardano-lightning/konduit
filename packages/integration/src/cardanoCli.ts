import { execFileSync } from "node:child_process";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { err, ok, Result } from "neverthrow";
import {
  buildCliCommand,
  type CliArgs,
  type CommandError,
} from "@konduit/cardano-cli/exec";

/**
 * @file A thin, *self-contained* `cardano-cli` helper used for wallet funding.
 *
 * Why not `@konduit/cardano-cli`'s `transferFunds`?
 * ------------------------------------------------
 * `transferFunds` finishes by calling `getTxIdFromTxFile`, which parses the
 * `conway transaction txid` output as `{"txhash": "..."}`. Modern
 * `cardano-cli` (>= 9) prints the **bare hex** txid instead, so the helper
 * fails with `Invalid JSON format: [object Object]` *after* the transfer has
 * already been submitted. Since the transaction is on-chain by then, the
 * failure is misleading and non-idempotent on retry.
 *
 * We therefore implement funding here directly, using only the stable
 * `execCli`/`buildCliCommand` surface from `@konduit/cardano-cli/exec` and
 * parsing only the two shapes we actually depend on:
 *   - `conway query utxo --output-json` (a `Record<TxOutRefHex, TxOut>`)
 *   - `conway transaction txid --tx-file <f>` (bare hex string)
 *
 * TODO: once the upstream helpers are fixed we should drop this module and go
 * back to `queryUtxosByAddress` + `transferFunds`.
 */

export type CardanoNode = {
  /** Path to the cardano-node unix socket. */
  nodeSocket: string;
  /** Testnet network magic (e.g. `42`). */
  networkMagic: number;
  /** Echo every command that is executed. */
  debug?: boolean;
};

const runCardanoCli = (
  node: CardanoNode,
  args: CliArgs,
): Result<string, CommandError> => {
  const cmd = buildCliCommand("cardano-cli", args);
  if (node.debug) console.debug(`[cardanoCli] $ ${cmd}`);
  try {
    const stdout = execFileSync("/bin/sh", ["-c", cmd, "--"], {
      encoding: "utf-8",
      stdio: ["ignore", "pipe", "pipe"],
      maxBuffer: 100 * 1024 * 1024,
    });
    return ok(stdout.toString().trim());
  } catch (e: any) {
    return err({
      type: "CommandError" as const,
      status: BigInt(e.status ?? e.code ?? 0),
      stdout: (e.stdout ?? "").toString(),
      stderr: (e.stderr ?? "").toString(),
    });
  }
};

// ---------------------------------------------------------------------------
// UTxO queries
// ---------------------------------------------------------------------------

export type UtxoSummary = {
  txOutRef: string;
  lovelace: bigint;
  hasReferenceScript: boolean;
  datum: string | null;
};

/**
 * Query every UTxO at `address`. Reference-script-bearing UTxOs are included
 * but flagged so callers can exclude them when selecting spendable inputs.
 */
export const queryUtxosByAddress = (
  address: string,
  node: CardanoNode,
): Result<UtxoSummary[], CommandError | string> => {
  const cmdResult = runCardanoCli(node, [
    "conway",
    "query",
    "utxo",
    ["--address", address],
    ["--testnet-magic", node.networkMagic],
    ["--socket-path", node.nodeSocket],
    "--output-json",
  ]);
  if (cmdResult.isErr()) return err(cmdResult.error);
  let parsed: unknown;
  try {
    parsed = JSON.parse(cmdResult.value === "" ? "{}" : cmdResult.value);
  } catch (e: any) {
    return err(
      `queryUtxosByAddress: failed to parse cardano-cli output: ${(e as Error).message}`,
    );
  }
  if (typeof parsed !== "object" || parsed === null) {
    return err("queryUtxosByAddress: unexpected cardano-cli output shape");
  }
  const utxos: UtxoSummary[] = [];
  for (const [txOutRef, raw] of Object.entries(
    parsed as Record<string, unknown>,
  )) {
    if (typeof raw !== "object" || raw === null) continue;
    const o = raw as Record<string, unknown>;
    const value = (o.value ?? {}) as Record<string, unknown>;
    const lovelace = BigInt(String(value.lovelace ?? 0));
    utxos.push({
      txOutRef,
      lovelace,
      hasReferenceScript:
        o.referenceScript !== null && o.referenceScript !== undefined,
      datum: typeof o.datumhash === "string" ? o.datumhash : null,
    });
  }
  return ok(utxos);
};

/** Total lovelace held by `address`, optionally ignoring reference-script UTxOs. */
export const totalLovelaceAt = (
  address: string,
  node: CardanoNode,
  opts: { skipReferenceScriptUtxos?: boolean } = {},
): Result<bigint, CommandError | string> =>
  queryUtxosByAddress(address, node).map((utxos) =>
    utxos
      .filter((u) => !(opts.skipReferenceScriptUtxos && u.hasReferenceScript))
      .reduce((acc, u) => acc + u.lovelace, 0n),
  );

// ---------------------------------------------------------------------------
// Funding
// ---------------------------------------------------------------------------

export type Recipient = {
  address: string;
  /** Amount in lovelace. */
  lovelace: bigint;
};

export type FundResult = {
  txId: string;
  /** Address of every input consumed from the faucet. */
  spentUtxos: string[];
};

/**
 * Select enough faucet UTxOs (largest-first) to cover `total`, skipping
 * reference-script UTxOs.
 */
const selectUtxos = (
  utxos: UtxoSummary[],
  total: bigint,
): Result<string[], string> => {
  const sorted = [...utxos].sort((a, b) =>
    b.lovelace > a.lovelace ? 1 : b.lovelace < a.lovelace ? -1 : 0,
  );
  const selected: string[] = [];
  let acc = 0n;
  for (const utxo of sorted) {
    selected.push(utxo.txOutRef);
    acc += utxo.lovelace;
    if (acc >= total) break;
  }
  if (acc < total) {
    return err(
      `Insufficient funds: need ${total} lovelace, faucet holds ${acc} across ${selected.length} spendable utxo(s)`,
    );
  }
  return ok(selected);
};

const makeTmpDir = (prefix: string): string =>
  fs.mkdtempSync(path.join(os.tmpdir(), `${prefix}-`));

/**
 * Move `recipients` out of the faucet wallet. Waits until the transaction is
 * confirmed on-chain before returning.
 */
export const fundFromFaucet = async (
  faucet: { address: string; skeyFile: string },
  recipients: Recipient[],
  node: CardanoNode,
  opts: { timeoutMs?: number; pollMs?: number } = {},
): Promise<Result<FundResult, string>> => {
  const timeoutMs = opts.timeoutMs ?? 180_000;
  const pollMs = opts.pollMs ?? 1_000;

  if (recipients.length === 0) return err("fundFromFaucet: no recipients");
  const totalSplit = recipients.reduce((acc, r) => acc + r.lovelace, 0n);

  const faucetUtxos = queryUtxosByAddress(faucet.address, node);
  if (faucetUtxos.isErr())
    return err(
      `fundFromFaucet: query faucet failed: ${String(faucetUtxos.error)}`,
    );
  const spendable = faucetUtxos.value.filter((u) => !u.hasReferenceScript);
  const selected = selectUtxos(spendable, totalSplit);
  if (selected.isErr()) return err(selected.error);

  const tmpDir = makeTmpDir("konduit-fund");
  const bodyFile = path.join(tmpDir, "fund.body.json");
  const signedFile = path.join(tmpDir, "fund.tx.json");

  try {
    const buildArgs: CliArgs = [
      "conway",
      "transaction",
      "build",
      ...selected.value.map((u): [string, string] => ["--tx-in", u]),
      ...recipients.map((r): [string, string] => [
        "--tx-out",
        `${r.address}+${r.lovelace}`,
      ]),
      ["--change-address", faucet.address],
      ["--out-file", bodyFile],
      ["--testnet-magic", node.networkMagic],
      ["--socket-path", node.nodeSocket],
    ];
    const build = runCardanoCli(node, buildArgs);
    if (build.isErr()) {
      return err(
        `fundFromFaucet: build failed (status=${build.error.status}): ${build.error.stderr || build.error.stdout}`,
      );
    }

    const sign = runCardanoCli(node, [
      "conway",
      "transaction",
      "sign",
      ["--tx-body-file", bodyFile],
      ["--signing-key-file", faucet.skeyFile],
      ["--out-file", signedFile],
      ["--testnet-magic", node.networkMagic],
    ]);
    if (sign.isErr()) {
      return err(
        `fundFromFaucet: sign failed (status=${sign.error.status}): ${sign.error.stderr || sign.error.stdout}`,
      );
    }

    // NOTE: `transaction txid` prints a *bare hex* string on modern cardano-cli
    // (no JSON envelope) — that is precisely what broke the upstream helper.
    const txIdResult = runCardanoCli(node, [
      "conway",
      "transaction",
      "txid",
      ["--tx-file", signedFile],
    ]);
    if (txIdResult.isErr()) {
      return err(
        `fundFromFaucet: txid failed (status=${txIdResult.error.status}): ${txIdResult.error.stderr || txIdResult.error.stdout}`,
      );
    }
    const txId = txIdResult.value.trim();
    if (!/^[0-9a-f]{64}$/.test(txId)) {
      return err(
        `fundFromFaucet: unexpected txid output: ${JSON.stringify(txIdResult.value.slice(0, 200))}`,
      );
    }

    const submit = runCardanoCli(node, [
      "conway",
      "transaction",
      "submit",
      ["--tx-file", signedFile],
      ["--testnet-magic", node.networkMagic],
      ["--socket-path", node.nodeSocket],
    ]);
    if (submit.isErr()) {
      return err(
        `fundFromFaucet: submit failed (status=${submit.error.status}): ${submit.error.stderr || submit.error.stdout}`,
      );
    }

    // Await confirmation: the change output of `txId` must appear at the faucet.
    const deadline = Date.now() + timeoutMs;
    while (Date.now() < deadline) {
      const utxos = queryUtxosByAddress(faucet.address, node);
      if (
        utxos.isOk() &&
        utxos.value.some((u) => u.txOutRef.startsWith(`${txId}#`))
      ) {
        return ok({ txId, spentUtxos: selected.value });
      }
      await new Promise((resolve) => setTimeout(resolve, pollMs));
    }
    return err(
      `fundFromFaucet: tx ${txId} was not confirmed within ${timeoutMs}ms`,
    );
  } finally {
    try {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    } catch {
      // best effort — the OS will reap /tmp eventually
    }
  }
};

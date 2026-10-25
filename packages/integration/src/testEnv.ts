import * as fs from "node:fs";
import * as path from "node:path";
import { execFileSync } from "node:child_process";
import type { IndexerDbConfig } from "./indexerDb.js";

/**
 * @file Top-level test environment loader.
 *
 * This is the **only** place in the integration suite that reads environment
 * variables or `.env*` files. Wallets, sockets, addresses, db paths etc. are
 * then threaded down into the wrappers as explicit options — keeping the
 * wrappers themselves `process.env`-free.
 *
 * The loader also:
 *   1. Talks to the `process-compose` daemon over `process-compose -p <port>`
 *      to make sure the dev/testnet is up.
 *   2. Locates (and builds, if missing) the `konduit-cli` binary used by the
 *      wrapper.
 *   3. Resolves the faucet info exposed by `set-faucet-info` so the test can
 *      pre-fund wallets when needed.
 *   4. Resolves where the running `konduit-indexer` keeps its SQLite store, so
 *      the db-check layer can assert on what the indexer actually persisted.
 *
 * ## Environment variables
 *
 * Everything below is optional unless stated otherwise; the defaults mirror the
 * dev-env exported by `flake.nix`, so a plain `nix develop` needs no extra setup.
 *
 * | variable | default | meaning |
 * | --- | --- | --- |
 * | `KONDUIT_REPO_ROOT` | autodetected | indexer repo root |
 * | `PC_PORT` | `8100` | process-compose daemon port |
 * | `TESTNET_DIR` | `$repo/.run/testnet` | testnet work dir |
 * | `FAUCET_ADDR_FILE` / `FAUCET_SKEY_FILE` / `FAUCET_WALLET_FILE` | under `TESTNET_DIR` | faucet material |
 * | `KONDUIT_E2E_TIMEOUT_MS` | `180000` | soft timeout for on-chain polls |
 * | `KONDUIT_E2E_POLL_MS` | `1500` | on-chain poll interval |
 * | `KONDUIT_E2E_DEBUG` | unset | echo every CLI command line |
 * | `KONDUIT_VALIDATOR_HASH` | unset | reference script hash, hex; cross-checked against both the chain and the db |
 * | **`KONDUIT_INDEXER_DB_PATH`** | `$repo/.run/konduit-indexer/konduit-indexer.db` | the SQLite file the running indexer writes |
 * | `KONDUIT_INDEXER_DB_CHECKS` | `auto` | `1`/`0` to force the db layer on/off; `auto` enables it iff the db file exists |
 * | `KONDUIT_INDEXER_DB_TIMEOUT_MS` | `120000` | soft timeout for indexer polls |
 * | `KONDUIT_INDEXER_DB_POLL_MS` | `1000` | indexer poll interval |
 * | `KONDUIT_INDEXER_DB_BUSY_TIMEOUT_MS` | `5000` | SQLite busy timeout for reads |
 * | `KONDUIT_INDEXER_SCHEMA_DIR` | `$repo/packages/konduit/indexer/sqlite` | migration dir, used to cross-check the schema |
 */

const PC_PORT_DEFAULT = 8100;

/** Echo every spawned command line / SQL statement. */
export const debugEnabled = (): boolean =>
  process.env.KONDUIT_E2E_DEBUG === "1";

export type KonduitTestEnv = {
  /** Absolute path to the indexer repo root (where `.env*` lives). */
  repoRoot: string;
  /** Process-compose port the test should talk to. */
  processComposePort: number;
  /** Network the local testnet is running on. */
  networkId: number;
  /** Path to the cardano-node unix socket for the local testnet. */
  cardanoNodeSocket: string;
  /** Absolute path to the built `konduit-cli` binary. */
  konduitCliBinary: string;
  /** Bech32 address hosting the konduit reference script. */
  hostAddress: string;
  /**
   * Hash of the konduit validator (the Plutus script), hex encoded.
   *
   * The dev-env exports this as `KONDUIT_VALIDATOR_HASH` and `kupo` is started
   * with `--match "$KONDUIT_VALIDATOR_HASH/*"`, so it is the contract between
   * the chain, the indexer and the tests: every `channel.script_hash` in the
   * store must equal it. Optional — when unset the db checks skip that
   * particular assertion rather than guessing.
   */
  validatorHashHex?: string;
  /** Echo every spawned command line. */
  debug: boolean;
  /** All wallet material needed for the scenario. */
  wallets: {
    admin: { skeyHex: string; address: string; hostAddress: string };
    consumer: { skeyHex: string; address: string; hostAddress: string };
    adaptor: { skeyHex: string; address: string; hostAddress: string };
  };
  /** Faucet info resolved from the process-compose `set-faucet-info` process. */
  faucet: { address: string; skeyFile: string; walletFile: string };
  /**
   * Where the running `konduit-indexer` persists its state.
   *
   * `enabled` is false when no db file is present (or when
   * `KONDUIT_INDEXER_DB_CHECKS=0`), which is how the e2e scenarios stay
   * runnable on a machine without the indexer process.
   */
  indexerDb: IndexerDbConfig & {
    enabled: boolean;
    /** Why the db layer is disabled — surfaced in test skips. */
    disabledReason?: string;
    timeoutMs: number;
    pollMs: number;
  };
  /** Soft timeout (ms) used by helpers like `waitForTip*` and `waitForChannel`. */
  defaultTimeoutMs: number;
  defaultPollMs: number;
};

// ---------------------------------------------------------------------------
// Tiny .env parser — we do this explicitly to avoid a `dotenv` dependency.
// ---------------------------------------------------------------------------

const parseEnvFile = (filePath: string): Record<string, string> => {
  const out: Record<string, string> = {};
  if (!fs.existsSync(filePath)) return out;
  const lines = fs.readFileSync(filePath, "utf-8").split("\n");
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const eq = trimmed.indexOf("=");
    if (eq < 0) continue;
    const key = trimmed.slice(0, eq).trim();
    let value = trimmed.slice(eq + 1).trim();
    if (value.startsWith('"') && value.endsWith('"'))
      value = value.slice(1, -1);
    if (value.startsWith("'") && value.endsWith("'"))
      value = value.slice(1, -1);
    out[key] = value;
  }
  return out;
};

// ---------------------------------------------------------------------------
// process-compose lightweight client (only what we actually need).
// ---------------------------------------------------------------------------

/**
 * Ask the `process-compose` daemon whether it is up.
 *
 * The daemon is local, so a couple of seconds is plenty; the deadline matters
 * because `execFileSync` blocks the event loop and a wedged daemon would
 * otherwise hang the whole suite (vitest's timeouts cannot fire while the
 * loop is blocked).
 */
export const pingProcessCompose = (port: number = PC_PORT_DEFAULT): boolean => {
  try {
    const text = execFileSync(
      "process-compose",
      ["-p", String(port), "process", "list", "-o", "json"],
      {
        encoding: "utf-8",
        stdio: ["ignore", "pipe", "ignore"],
        timeout: 10_000,
        killSignal: "SIGKILL",
      },
    );
    return text.includes('"name"');
  } catch {
    return false;
  }
};

// ---------------------------------------------------------------------------
// Indexer db location.
//
// The running indexer gets its `--db-path` from `flake/process-compose/dev-env.nix`,
// which reads `KONDUIT_INDEXER_DB_PATH` and the dev-shell sets it to
// `$KONDUIT_INDEXER_DIR/konduit-indexer.db`. We mirror those names so the tests
// automatically target the *live* store rather than a copy.
// ---------------------------------------------------------------------------

const parseDbChecksFlag = (raw: string | undefined): "1" | "0" | "auto" => {
  if (raw === undefined || raw === "") return "auto";
  const normalised = raw.trim().toLowerCase();
  if (["1", "true", "yes", "on"].includes(normalised)) return "1";
  if (["0", "false", "no", "off"].includes(normalised)) return "0";
  return "auto";
};

export const resolveIndexerDbConfig = (
  repoRoot: string,
): KonduitTestEnv["indexerDb"] => {
  const dbPath =
    process.env.KONDUIT_INDEXER_DB_PATH ||
    path.join(repoRoot, ".run", "konduit-indexer", "konduit-indexer.db");
  const schemaDir =
    process.env.KONDUIT_INDEXER_SCHEMA_DIR ||
    path.join(repoRoot, "packages", "konduit", "indexer", "sqlite");

  const flag = parseDbChecksFlag(process.env.KONDUIT_INDEXER_DB_CHECKS);
  const exists = fs.existsSync(dbPath);
  const enabled = flag === "1" || (flag === "auto" && exists);

  let disabledReason: string | undefined;
  if (!enabled) {
    disabledReason =
      flag === "0"
        ? "KONDUIT_INDEXER_DB_CHECKS=0"
        : `no konduit-indexer db at ${dbPath}`;
  }

  return {
    dbPath,
    schemaDir,
    enabled,
    disabledReason,
    busyTimeoutMs: tryReadNumber(
      process.env.KONDUIT_INDEXER_DB_BUSY_TIMEOUT_MS,
      5_000,
    ),
    timeoutMs: tryReadNumber(
      process.env.KONDUIT_INDEXER_DB_TIMEOUT_MS,
      2 * 60 * 1000,
    ),
    pollMs: tryReadNumber(process.env.KONDUIT_INDEXER_DB_POLL_MS, 1_000),
    debug: debugEnabled(),
  };
};

// ---------------------------------------------------------------------------
// konduit-cli binary discovery / build.
// ---------------------------------------------------------------------------

const resolveKonduitCliBinary = (repoRoot: string): string => {
  const candidates = [
    path.join(repoRoot, "target", "debug", "konduit-cli"),
    path.join(repoRoot, "target", "release", "konduit-cli"),
    path.join(repoRoot, ".run", "konduit-cli"),
  ];
  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) return candidate;
  }
  return "";
};

const buildKonduitCliBinary = (repoRoot: string): string => {
  console.log(
    "[konduitCli] binary missing; running `cargo build --bin konduit-cli`...",
  );
  execFileSync("cargo", ["build", "--bin", "konduit-cli"], {
    cwd: repoRoot,
    stdio: ["ignore", "inherit", "inherit"],
    env: process.env,
  });
  const resolved = resolveKonduitCliBinary(repoRoot);
  if (!resolved) {
    throw new Error(
      `cargo build reported success but konduit-cli is not present under ${path.join(repoRoot, "target")}`,
    );
  }
  return resolved;
};

// ---------------------------------------------------------------------------
// Faucet info (paths set by the `set-faucet-info` process-compose process).
// The dev-env nix shell hardcodes them under `$TESTNET_DIR`.
// ---------------------------------------------------------------------------

const resolveFaucetInfo = (repoRoot: string): KonduitTestEnv["faucet"] => {
  const defaultDir = path.join(repoRoot, ".run", "testnet");
  const testnetDir = process.env.TESTNET_DIR || defaultDir;
  const addrFile =
    process.env.FAUCET_ADDR_FILE || path.join(testnetDir, "faucet.addr");
  const skeyFile =
    process.env.FAUCET_SKEY_FILE || path.join(testnetDir, "faucet.skey");
  const walletFile =
    process.env.FAUCET_WALLET_FILE || path.join(testnetDir, "faucet.wallet");
  for (const p of [addrFile, skeyFile]) {
    if (!fs.existsSync(p)) throw new Error(`Faucet info missing at ${p}`);
  }
  return {
    address: fs.readFileSync(addrFile, "utf-8").trim(),
    skeyFile,
    walletFile,
  };
};

// ---------------------------------------------------------------------------
// Wallet material — read from .env.admin / .env.consumer / .env.adaptor.
// ---------------------------------------------------------------------------

const readRoleSkey = (
  repoRoot: string,
  role: "admin" | "consumer" | "adaptor",
): string => {
  const envPath = path.join(repoRoot, `.env.${role}`);
  const values = parseEnvFile(envPath);
  const skeyHex = values.KONDUIT_WALLET;
  if (!skeyHex) throw new Error(`${envPath} is missing KONDUIT_WALLET`);
  return skeyHex.trim();
};

/**
 * Ask `konduit-cli <role> show address` for the role's own bech32 address.
 *
 * We cannot derive it in TypeScript cheaply, and — importantly — it must be
 * derived by the CLI itself so that we never conflate a role's *own* address
 * with `KONDUIT_HOST_ADDRESS` (the address that hosts the reference script).
 * Conflating the two silently funds/targets the wrong wallet.
 */
const resolveRoleAddress = (
  binary: string,
  role: "admin" | "consumer" | "adaptor",
  skeyHex: string,
  hostAddress: string,
  nodeSocket: string,
  networkId: number,
): string => {
  const script = `
set -eu
KONDUIT_CARDANO_BACKEND=node
KONDUIT_NETWORK=${networkId}
KONDUIT_NODE_SOCKET=${nodeSocket}
KONDUIT_WALLET=${skeyHex}
KONDUIT_HOST_ADDRESS=${hostAddress}
export KONDUIT_CARDANO_BACKEND KONDUIT_NETWORK KONDUIT_NODE_SOCKET KONDUIT_WALLET KONDUIT_HOST_ADDRESS
exec ${binary} ${role} show address
`;
  const out = execFileSync("/bin/sh", ["-c", script], {
    encoding: "utf-8",
    // `execFileSync` blocks the event loop: a command that never returns would
    // freeze the whole suite, with vitest unable to even time the test out.
    timeout: 60_000,
    killSignal: "SIGKILL",
  }).trim();
  if (!out)
    throw new Error(`konduit-cli ${role} show address returned empty output`);
  return out;
};

// ---------------------------------------------------------------------------
// Top-level loader.
// ---------------------------------------------------------------------------

const tryReadNumber = (raw: string | undefined, fallback: number): number => {
  if (!raw) return fallback;
  const n = Number(raw);
  return Number.isFinite(n) ? n : fallback;
};

const findRepoRoot = (start: string): string => {
  let dir = start;
  for (let i = 0; i < 12; i++) {
    if (
      fs.existsSync(path.join(dir, "Cargo.toml")) &&
      fs.existsSync(path.join(dir, ".env"))
    ) {
      return dir;
    }
    const parent = path.dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  return start;
};

export const loadTestEnv = (
  overrides: Partial<KonduitTestEnv> = {},
): KonduitTestEnv => {
  const configuredRoot = overrides.repoRoot ?? process.env.KONDUIT_REPO_ROOT;
  const repoRoot = configuredRoot ?? findRepoRoot(process.cwd());
  if (!fs.existsSync(path.join(repoRoot, "Cargo.toml"))) {
    throw new Error(
      `loadTestEnv: ${repoRoot} does not look like an indexer repo (no Cargo.toml)`,
    );
  }
  const rootEnv = parseEnvFile(path.join(repoRoot, ".env"));

  const processComposePort = tryReadNumber(
    process.env.PC_PORT,
    PC_PORT_DEFAULT,
  );
  if (!pingProcessCompose(processComposePort)) {
    throw new Error(
      `process-compose daemon not reachable on port ${processComposePort}. ` +
        `Run \`nix develop\` and \`process-compose-dev-env\` first.`,
    );
  }

  const networkId = tryReadNumber(rootEnv.KONDUIT_NETWORK, 42);
  const cardanoNodeSocket =
    process.env.CARDANO_NODE_SOCKET_PATH ||
    rootEnv.KONDUIT_NODE_SOCKET ||
    path.join(repoRoot, ".run", "testnet", "state-cluster9", "bft1.socket");

  // Propagate `CARDANO_NODE_SOCKET_PATH` into the env so that helpers like
  // `@konduit/cardano-cli`'s `queryUtxosByAddress` reach our local testnet
  // even when called outside of `nix develop` (where the dev-env normally
  // exports it). Setting it for the entire test process is the simplest fix
  // and matches what `marlowe-integration` does.
  if (!process.env.CARDANO_NODE_SOCKET_PATH) {
    process.env.CARDANO_NODE_SOCKET_PATH = cardanoNodeSocket;
  }

  let konduitCliBinary = resolveKonduitCliBinary(repoRoot);
  if (!konduitCliBinary) konduitCliBinary = buildKonduitCliBinary(repoRoot);

  const wallets = {
    admin: readRoleSkey(repoRoot, "admin"),
    consumer: readRoleSkey(repoRoot, "consumer"),
    adaptor: readRoleSkey(repoRoot, "adaptor"),
  };

  const hostAddress = rootEnv.KONDUIT_HOST_ADDRESS;
  if (!hostAddress)
    throw new Error(
      `${path.join(repoRoot, ".env")} is missing KONDUIT_HOST_ADDRESS`,
    );

  const faucet = resolveFaucetInfo(repoRoot);

  return {
    repoRoot,
    processComposePort,
    networkId,
    cardanoNodeSocket,
    konduitCliBinary,
    hostAddress,
    validatorHashHex: process.env.KONDUIT_VALIDATOR_HASH || undefined,
    debug: debugEnabled(),
    wallets: {
      admin: {
        skeyHex: wallets.admin,
        address: resolveRoleAddress(
          konduitCliBinary,
          "admin",
          wallets.admin,
          hostAddress,
          cardanoNodeSocket,
          networkId,
        ),
        hostAddress,
      },
      consumer: {
        skeyHex: wallets.consumer,
        address: resolveRoleAddress(
          konduitCliBinary,
          "consumer",
          wallets.consumer,
          hostAddress,
          cardanoNodeSocket,
          networkId,
        ),
        hostAddress,
      },
      adaptor: {
        skeyHex: wallets.adaptor,
        address: resolveRoleAddress(
          konduitCliBinary,
          "adaptor",
          wallets.adaptor,
          hostAddress,
          cardanoNodeSocket,
          networkId,
        ),
        hostAddress,
      },
    },
    faucet,
    indexerDb: resolveIndexerDbConfig(repoRoot),
    defaultTimeoutMs: tryReadNumber(
      process.env.KONDUIT_E2E_TIMEOUT_MS,
      3 * 60 * 1000,
    ),
    defaultPollMs: tryReadNumber(process.env.KONDUIT_E2E_POLL_MS, 1500),
    ...overrides,
  };
};

import * as fs from "node:fs";
import * as path from "node:path";
import { execFileSync } from "node:child_process";

/**
 * @file Top-level test environment loader.
 *
 * This is the **only** place in the integration suite that reads environment
 * variables or `.env*` files. Wallets, sockets, addresses, etc. are then
 * threaded down into the wrappers as explicit options — keeping the wrappers
 * themselves `process.env`-free.
 *
 * The loader also:
 *   1. Talks to the `process-compose` daemon over `process-compose -p <port>`
 *      to make sure the dev/testnet is up.
 *   2. Locates (and builds, if missing) the `konduit-cli` binary used by the
 *      wrapper.
 *   3. Resolves the faucet info exposed by `set-faucet-info` so the test can
 *      pre-fund wallets when needed.
 */

const PC_PORT_DEFAULT = 8100;

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
  /** All wallet material needed for the scenario. */
  wallets: {
    admin: { skeyHex: string; address: string; hostAddress: string };
    consumer: { skeyHex: string; address: string; hostAddress: string };
    adaptor: { skeyHex: string; address: string; hostAddress: string };
  };
  /** Faucet info resolved from the process-compose `set-faucet-info` process. */
  faucet: { address: string; skeyFile: string; walletFile: string };
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

export const pingProcessCompose = (port: number = PC_PORT_DEFAULT): boolean => {
  try {
    const text = execFileSync(
      "process-compose",
      ["-p", String(port), "process", "list", "-o", "json"],
      { encoding: "utf-8", stdio: ["ignore", "pipe", "ignore"] },
    );
    return text.includes('"name"');
  } catch {
    return false;
  }
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
    defaultTimeoutMs: tryReadNumber(
      process.env.KONDUIT_E2E_TIMEOUT_MS,
      3 * 60 * 1000,
    ),
    defaultPollMs: tryReadNumber(process.env.KONDUIT_E2E_POLL_MS, 1500),
    ...overrides,
  };
};

import { beforeAll, describe, it, expect } from "vitest";
import { execCli } from "@konduit/cardano-cli/exec";
import { getTip } from "@konduit/cardano-cli";
import {
  loadTestEnv,
  pingProcessCompose,
  type KonduitTestEnv,
} from "../src/testEnv.js";
import { buildCardanoContext } from "../src/scenarios/shared.js";

describe("cardano-cli wrapper", () => {
  it("runs `cardano-cli --version` and returns a valid version string", () => {
    const result = execCli("cardano-cli", ["--version"]);
    result.match(
      (output) => {
        expect(output).toMatch(/^cardano-cli \d+\.\d+\.\d+\.\d+/);
      },
      (err) => {
        throw new Error(`cardano-cli --version failed: ${JSON.stringify(err)}`);
      },
    );
  });
});

describe("cardano-cli package :: live testnet", () => {
  const daemonAlive = pingProcessCompose();
  if (!daemonAlive) {
    it.skip("process-compose daemon is not reachable on port 8100", () => {});
    return;
  }

  let env: KonduitTestEnv;

  beforeAll(() => {
    env = loadTestEnv();
  }, 30_000);

  it("queries the synchronized chain tip", () => {
    const cardano = buildCardanoContext(env);
    const result = getTip(cardano.networkMagic, cardano.debug);
    expect(result.isOk()).toBe(true);
    if (result.isOk()) {
      expect(result.value.syncProgress).toBeGreaterThanOrEqual(1);
    }
  });
});

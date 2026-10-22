import { beforeAll, describe, expect, it } from "vitest";
import {
  loadTestEnv,
  pingProcessCompose,
  type KonduitTestEnv,
} from "../../src/testEnv.js";
import { runSub } from "../../src/scenarios/sub.js";

describe("konduit e2e :: sub (L2 cheques -> L1 transfer)", () => {
  // Skip rather than fail when the dev-env is not running — otherwise the
  // suite just burns time on a confusing connection error.
  const daemonAlive = pingProcessCompose();
  if (!daemonAlive) {
    it.skip("process-compose daemon is not reachable on port 8100", () => {});
    return;
  }

  let env: KonduitTestEnv;
  beforeAll(() => {
    env = loadTestEnv();
  }, 60_000);

  it(
    "issues locked cheques, squashes, and settles them on-chain",
    async () => {
      const result = await runSub(env);

      // The protocol arithmetic is checked inside `runSub`; these assertions
      // restate the headline facts so a failure names them explicitly.
      expect(result.tag).toMatch(/^[0-9a-f]{16}$/);
      expect(result.expectedGainLovelace).toBeGreaterThan(0n);
      expect(result.openTxId).toMatch(/^[0-9a-f]{64}$/);
      expect(result.subTxId).toMatch(/^[0-9a-f]{64}$/);

      // The money really moved: the adaptor's wallet grew by at least the gain.
      expect(result.observedGainLovelace).toBeGreaterThanOrEqual(
        result.expectedGainLovelace,
      );
    },
    10 * 60 * 1000,
  );
});

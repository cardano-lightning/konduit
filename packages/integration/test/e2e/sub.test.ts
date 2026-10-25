import { beforeAll, describe, expect, it } from "vitest";
import {
  loadTestEnv,
  pingProcessCompose,
  type KonduitTestEnv,
} from "../../src/testEnv.js";
import { runSub } from "../../src/scenarios/sub.js";
import { EXEC_FEE_BUDGET_LOVELACE } from "../../src/scenarios/shared.js";

describe("konduit e2e :: sub (L2 cheques -> L1 transfer)", () => {
  // Fail fast rather than wasting time..
  const daemonAlive = pingProcessCompose();
  if (!daemonAlive) {
    throw new Error(
      "process-compose daemon is not reachable on port 8100; run `pnpm dev-env` first",
    );
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

      // The money really moved: the sub paid the adaptor at least the gain.
      expect(result.adaptorPayoutLovelace).toBeGreaterThanOrEqual(
        result.expectedGainLovelace,
      );
      // … and the adaptor paid for it with the execution fee, which is why
      // its balance does not grow by the whole gain.
      expect(result.adaptorFeeLovelace).toBeGreaterThan(0n);
      expect(result.adaptorFeeLovelace).toBeLessThanOrEqual(
        EXEC_FEE_BUDGET_LOVELACE,
      );
    },
    10 * 60 * 1000,
  );
});

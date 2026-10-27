import { beforeAll, describe, expect, it } from "vitest";
import {
  loadTestEnv,
  pingProcessCompose,
  type KonduitTestEnv,
} from "../../src/testEnv.js";
import {
  runLifecycle,
  __test_internals,
} from "../../src/scenarios/lifecycle.js";

describe("konduit e2e :: lifecycle", () => {
  // We only run the e2e test when the dev-env is up — running it without
  // `process-compose` would just burn time and produce a confusing failure.
  const daemonAlive = pingProcessCompose();
  if (!daemonAlive) {
    it.skip("process-compose daemon is not reachable on port 8100", () => {});
    return;
  }

  let env: KonduitTestEnv;
  beforeAll(() => {
    env = loadTestEnv();
  }, 30_000);

  it(
    "opens, adds funds to and closes a channel with a unique tag",
    async () => {
      await runLifecycle(env);
    },
    10 * 60 * 1000,
  );

  describe("tag uniqueness helper", () => {
    it("produces 16 hex chars (8 bytes) for an arbitrary salt", () => {
      expect(__test_internals.uniqueTagHex(0)).toMatch(/^[0-9a-f]{16}$/);
      expect(__test_internals.uniqueTagHex(42)).toMatch(/^[0-9a-f]{16}$/);
    });

    it("two salts at the same millisecond produce distinct tags", () => {
      const a = __test_internals.uniqueTagHex(1);
      const b = __test_internals.uniqueTagHex(2);
      expect(a).not.toBe(b);
    });
  });
});

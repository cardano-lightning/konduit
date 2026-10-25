import { beforeAll, describe, expect, it } from "vitest";
import { AddressBech32 } from "@konduit/konduit-consumer/cardano";
import { transferFunds } from "@konduit/cardano-cli";
import {
  unwrapOrPanic,
  unwrapOrPanicWith,
} from "@konduit/konduit-consumer/neverthrow";
import {
  loadTestEnv,
  pingProcessCompose,
  type KonduitTestEnv,
} from "../../src/testEnv.js";
import {
  buildCardanoContext,
  totalLovelaceAt,
} from "../../src/scenarios/shared.js";
import { describeError } from "../../src/konduitCli.js";

/**
 * The funding path used to live in a local `cardanoCli.ts` module, because
 * `@konduit/cardano-cli`'s `transferFunds` could not read the transaction id
 * out of the cardano-cli output (it parses `transaction txid` as JSON, while
 * cardano-cli prints a bare hex string) and swallowed the build/sign/submit
 * errors — so it reported a failure *after* the transaction was already
 * on-chain.
 *
 * These tests pin the fixed behaviour down: funding must report the id of the
 * transaction the node confirmed, and the money must really be there.
 */
describe("konduit e2e :: funding", () => {
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
    "transfers funds from the faucet and reports the confirmed tx id",
    async () => {
      const cardano = buildCardanoContext(env);
      const faucet = unwrapOrPanicWith(
        AddressBech32.fromString(env.faucet.address),
        (error) => `invalid faucet address: ${error}`,
      );

      const recipientAddress = env.wallets.consumer.address;
      const recipient = unwrapOrPanicWith(
        AddressBech32.fromString(recipientAddress),
        (error) => `invalid recipient address: ${error}`,
      );

      const before = unwrapOrPanic(
        totalLovelaceAt(recipientAddress, cardano),
        "failed to query the recipient",
      );

      const amount = 5_000_000;
      const result = unwrapOrPanicWith(
        await transferFunds(
          faucet,
          env.faucet.skeyFile,
          [{ address: recipient, amount }],
          cardano.networkMagic,
          null,
          cardano.debug,
        ),
        (error) => `transferFunds failed: ${describeError(error)}`,
      );

      // The id is the one of the transaction the node confirmed.
      expect(result).toMatch(/^[0-9a-f]{64}$/);

      const after = unwrapOrPanic(
        totalLovelaceAt(recipientAddress, cardano),
        "failed to query the recipient",
      );
      expect(after).toBe(before + BigInt(amount));
    },
    5 * 60 * 1000,
  );
});

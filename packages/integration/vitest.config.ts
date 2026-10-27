import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["test/**/*.test.ts"],
    environment: "node",

    // The e2e scenarios all drive the *same* local testnet through the *same*
    // admin/consumer/adaptor wallets and the *same* faucet. Running test files
    // in parallel makes them race for those UTxOs — the loser's funding
    // transaction fails with `BadInputsUTxO`, or worse, one scenario spends the
    // UTxO another one selected moments earlier.
    //
    // So: one file at a time, and one test at a time within a file.
    fileParallelism: false,
    sequence: {
      concurrent: false,
    },
  },
});

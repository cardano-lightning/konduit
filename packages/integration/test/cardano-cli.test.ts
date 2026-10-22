import { describe, it, expect } from "vitest";
import { execCli } from "@konduit/cardano-cli/exec";

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

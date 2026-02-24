import blake2b from "blake2b";

const MAX_PLUTUS_VERSION = 3;

export function guessPlutusVersion(scriptHash, script) {
  const scriptHashV = (v) =>
    blake2b(28)
      .update(Buffer.concat([Buffer.from([v]), Buffer.from(script, "hex")]))
      .digest("hex");

  for (let version = MAX_PLUTUS_VERSION; version > 0; version -= 1) {
    if (scriptHash === scriptHashV(version)) {
      return version;
    }
  }

  // Likely a native script?
  return 0;
}

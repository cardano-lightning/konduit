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

/**
 * Aggregate pieces of information from various blockfrost endpoint into a cardano-connector-server's Transaction
 */
export function toTransaction(tip, meta, utxos) {
  return {
    id: meta.hash,
    index: meta.index,
    depth: Math.max(0, tip.height - meta.block_height),
    timestamp: Number(meta.block_time),
    ...(meta.invalid_before != null && { invalid_before: Number(meta.invalid_before) }),
    ...(meta.invalid_after != null && { invalid_after: Number(meta.invalid_after) }),
    ...partitionInputsOutputs(meta, utxos),
  }
}

/**
 * Convert a blockfrost output to a connector-server output.
 */
export function toOutput(json) {
  return {
    address: json.address,
    value: json.amount,
    ...(json.data_hash != null && { datum_hash: json.data_hash }),
    ...(json.inline_datum != null && { datum_inline: json.inline_datum }),
    ...(json.reference_script_hash != null && {
      reference_script_hash: json.reference_script_hash,
    }),
  };
}

/**
 * Retrieve spent inputs and produced outputs from Blockfrost's transactions. Blockfrost returns reference inputs,
 * collateral inputs and normal inputs as part of the same inputs field. Similarly, it mixes collateral outputs and
 * normal output.
 *
 * This function filters both based on the transaction phase-2 success to return the effectively spent and effectively
 * produced inputs and outputs.
 *
 * @param {any} meta The result from the /txs/:id Blockfrost endpoint.
 * @param {any} utxos The result from /txs/:id/utxos Blockfrost endpoint.
 * @returns {{ inputs: any[], outputs: any[] }}
 */
export function partitionInputsOutputs(meta, utxos) {
  return {
    inputs: utxos.inputs
      .filter(
        (i) =>
          !i.reference &&
          (meta.valid_contract ? !i.collateral : i.collateral),
      )
      .map((i) => ({
        transaction_id: i.tx_hash,
        output_index: i.output_index,
        ...toOutput(i),
      })),
    outputs: utxos.outputs
      .filter((o) =>
        meta.valid_contract ? !o.collateral : o.collateral,
      )
      .map(toOutput),
  };
}

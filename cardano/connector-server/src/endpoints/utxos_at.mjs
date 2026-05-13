import { guessPlutusVersion, toOutput } from "../helpers/index.mjs";

const DEFAULT_VALUE = [];

export async function endpointUtxosAt(ctx) {
  return await ctx.endpoint(DEFAULT_VALUE, async () => {
    const utxos = await ctx.blockfrost(
      `/addresses/${ctx.req.param("address")}/utxos`,
    );

    return await Promise.all(
      utxos.map(async (utxo) => {
        let reference_script = null;
        let reference_script_version = null;

        let output = toOutput(utxo);

        if (output.reference_script_hash) {
          const { cbor } = await ctx.blockfrost(
            `/scripts/${output.reference_script_hash}/cbor`,
          );
          output.reference_script = cbor;
          output.reference_script_version = guessPlutusVersion(
            output.reference_script_hash,
            cbor,
          );
        }

        return {
          transaction_id: utxo.tx_hash,
          output_index: utxo.tx_index,
          ...output,
        };
      }),
    );
  });
}

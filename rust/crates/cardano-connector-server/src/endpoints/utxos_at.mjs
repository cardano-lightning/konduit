import { guessPlutusVersion } from '../helpers';

export async function endpointUtxosAt(ctx) {
  try {
    const utxos = await ctx.blockfrost(
      `/addresses/${ctx.req.param("address")}/utxos`,
    );

    return ctx.json(
      await Promise.all(
        utxos.map(async (utxo) => {
          let reference_script = null;
          let reference_script_version = null;

          if (utxo.reference_script_hash) {
            const { cbor } = await ctx.blockfrost(
              `/scripts/${utxo.reference_script_hash}/cbor`,
            );
            reference_script = cbor;
            reference_script_version = guessPlutusVersion(
              utxo.reference_script_hash,
              cbor,
            );
          }

          return {
            transaction_id: utxo.tx_hash,
            output_index: utxo.tx_index,
            address: utxo.address,
            value: utxo.amount,
            datum_hash: utxo.data_hash,
            datum_inline: utxo.inline_datum,
            reference_script_version,
            reference_script,
          };
        }),
      ),
    );
  } catch (res) {
    if (res.status === 404) {
      return ctx.json([]);
    }
    if (res.status && res.statusText) {
      console.log(`${res.status} ${res.statusText}: ${await res.text()}`);
    } else {
      console.log(res);
    }
    throw "unexpected error";
  }
}

export async function endpointTransactions(ctx) {
  try {
    const [tip, transactions] = await Promise.all([
      await ctx.blockfrost(`/blocks/latest`),
      await ctx.blockfrost(`/addresses/${ctx.req.param("address")}/transactions`)
    ]);

    return ctx.json(
      await Promise.all(
        transactions.map(async (tx) => {
          const meta = await ctx.blockfrost(`/txs/${tx.tx_hash}`);
          const utxos = await ctx.blockfrost(`/txs/${tx.tx_hash}/utxos`);

          return {
            id: tx.tx_hash,
            index: tx.tx_index,
            depth: Math.max(0, tip.height - tx.block_height),
            timestamp: tx.block_time,
            invalid_before: meta.invalid_before,
            invalid_after: meta.invalid_after,
            inputs: utxos.inputs
              .filter(i => !i.reference && (meta.valid_contract ? !i.collateral : i.collateral))
              .map((i) => ({
                transaction_id: i.tx_hash,
                output_index: i.output_index,
                ...toOutput(i),
              })),
            outputs: utxos.outputs
              .filter(o => meta.valid_contract ? !o.collateral : o.collateral)
              .map(toOutput),
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

function toOutput(json) {
  return {
    address: json.address,
    value: json.amount,
    ...(json.data_hash != null && { datum_hash: json.data_hash }),
    ...(json.inline_datum != null && { datum_inline: json.inline_datum }),
    ...(json.reference_script_hash != null && { reference_script_hash: json.reference_script_hash }),
  };
}

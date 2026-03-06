import { toOutput } from "./transactions.mjs";

export async function endpointTransaction(ctx) {
  try {
    const [meta, utxos, tip] = await Promise.all([
      await ctx.blockfrost(`/txs/${ctx.req.param("id")}`),
      await ctx.blockfrost(`/txs/${ctx.req.param("id")}/utxos`),
      await ctx.blockfrost(`/blocks/latest`),
    ]);
    return ctx.json({
      block_time: meta.block_time,
      depth: Math.max(0, tip.height - meta.block_height),
      invalid_before: meta.invalid_before,
      invalid_after: meta.invalid_after,
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
        .filter((o) => (meta.valid_contract ? !o.collateral : o.collateral))
        .map((o) => ({
          output_index: o.output_index,
          ...toOutput(o),
          collateral: o.collateral,
          consumed_by_tx: o.consumed_by_tx,
        })),
    });
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

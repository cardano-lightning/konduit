import { toTransaction } from '../helpers/index.mjs';

const DEFAULT_VALUE = null;

export async function endpointTransaction(ctx) {
  return await ctx.endpoint(DEFAULT_VALUE, async () => {
    const [tip, meta, utxos] = await Promise.all([
      await ctx.blockfrost(`/blocks/latest`),
      await ctx.blockfrost(`/txs/${ctx.req.param("id")}`),
      await ctx.blockfrost(`/txs/${ctx.req.param("id")}/utxos`),
    ]);
    return toTransaction(tip, meta, utxos);
  });
}

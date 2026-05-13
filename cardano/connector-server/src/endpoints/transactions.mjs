import { toTransaction } from "../helpers/index.mjs";

const DEFAULT_VALUE = [];

export async function endpointTransactions(ctx) {
  return await ctx.endpoint(DEFAULT_VALUE, async () => {
    const [tip, transactions] = await Promise.all([
      await ctx.blockfrost(`/blocks/latest`),
      await ctx.blockfrost(
        `/addresses/${ctx.req.param("address")}/transactions`,
      ),
    ]);

    return await Promise.all(
      transactions.map(async (tx) => {
        const meta = await ctx.blockfrost(`/txs/${tx.tx_hash}`);
        const utxos = await ctx.blockfrost(`/txs/${tx.tx_hash}/utxos`);
        return toTransaction(tip, meta, utxos);
      }),
    );
  });
}

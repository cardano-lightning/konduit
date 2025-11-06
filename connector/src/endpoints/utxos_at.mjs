export async function endpointUtxosAt(ctx) {
  try {
    const utxos = await ctx.blockfrost(`/addresses/${ctx.req.param('address')}/utxos`);
    return ctx.json(utxos);
  } catch (res) {
    if (res.status === 404) {
      return ctx.json({});
    }
    console.log(res);
    throw 'unexpected error';
  }
}

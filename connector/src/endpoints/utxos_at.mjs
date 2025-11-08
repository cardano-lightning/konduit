export async function endpointUtxosAt(ctx) {
  try {
    const utxos = await ctx.blockfrost(`/addresses/${ctx.req.param('address')}/utxos`);
    return ctx.json(utxos);
  } catch (res) {
    if (res.status === 404) {
      return ctx.json({});
    }
    if (res.status && res.statusText) {
      console.log(`${res.status} ${res.statusText}: ${await res.text()}`);
    } else {
      console.log(res);
    }
    throw 'unexpected error';
  }
}

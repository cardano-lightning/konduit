export async function endpointNetwork(ctx) {
  return ctx.json({ network: ctx.network });
}

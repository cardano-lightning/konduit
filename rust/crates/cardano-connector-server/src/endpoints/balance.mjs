const DEFAULT_VALUE = { lovelace: "0" };

export async function endpointBalance(ctx) {
  return await ctx.endpoint(DEFAULT_VALUE, async () => {
    const addressInfo = await ctx.blockfrost(
      `/addresses/${ctx.req.param("address")}`,
    );
    const quantity = addressInfo.amount?.find(
      (asset) => asset.unit === "lovelace",
    )?.quantity;
    const lovelace = quantity ?? DEFAULT_VALUE.lovelace;
    return { lovelace };
  });
}

export async function endpointBalance(ctx) {
  try {
    const addressInfo = await ctx.blockfrost(
      `/addresses/${ctx.req.param("address")}`,
    );
    const lovelace =
      addressInfo.amount?.find((asset) => asset.unit === "lovelace")
        ?.quantity ?? "0";
    return ctx.json({ lovelace });
  } catch (res) {
    if (res.status === 404) {
      return ctx.json({ lovelace: "0" });
    }
    if (res.status && res.statusText) {
      console.log(`${res.status} ${res.statusText}: ${await res.text()}`);
    } else {
      console.log(res);
    }
    throw "unexpected error";
  }
}

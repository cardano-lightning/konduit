import { Buffer } from "node:buffer";

export async function endpointSubmit(ctx) {
  return await ctx.endpoint(undefined, async () => {
    const { transaction } = await ctx.req.json();
    const transaction_id = await ctx.koios(`/submittx`, {
      method: "POST",
      body: Buffer.from(transaction, "hex"),
      headers: {
        "Content-Type": "application/cbor",
      },
    });
    return { transaction_id };
  });
}

import { Buffer } from "node:buffer";

export async function endpointSubmit(ctx) {
  try {
    const { transaction } = await ctx.req.json();
    const transaction_id = await ctx.koios(`/submittx`, {
      method: "POST",
      body: Buffer.from(transaction, "hex"),
      headers: {
        "Content-Type": "application/cbor",
      },
    });
    return ctx.json({ transaction_id });
  } catch (res) {
    if (res.status && res.statusText) {
      console.log(`${res.status} ${res.statusText}: ${await res.text()}`);
    } else {
      console.log(res);
    }
    throw 'unexpected error';
  }
}

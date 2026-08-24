import { Buffer } from "node:buffer";

import { requireLease } from "./session.mjs";

export async function endpointSubmit(ctx) {
  const lease = await requireLease(ctx);
  if (!lease) return ctx.json({ error: "invalid or stale lease" }, 401);
  const { operation_id, transaction } = await ctx.req.json();
  if (!/^[0-9a-f-]{36}$/.test(operation_id) || !/^[0-9a-f]+$/.test(transaction) || transaction.length % 2) {
    return ctx.json({ error: "invalid submission" }, 400);
  }
  return await ctx.endpoint(undefined, async () => {
    const request = { ...lease.payload, operationId: operation_id };
    const previous = await lease.object.fetch("https://session/submit/get", { method: "POST", body: JSON.stringify(request) });
    const { transactionId } = await previous.json();
    if (transactionId) return { transaction_id: transactionId };

    const transaction_id = await ctx.koios(`/submittx`, {
      method: "POST",
      body: Buffer.from(transaction, "hex"),
      headers: { "Content-Type": "application/cbor" },
    });
    const recorded = await lease.object.fetch("https://session/submit/put", {
      method: "POST",
      body: JSON.stringify({ ...request, transactionId: transaction_id }),
    });
    if (!recorded.ok) throw recorded;
    return { transaction_id };
  });
}

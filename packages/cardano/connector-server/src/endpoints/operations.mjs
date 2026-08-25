const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
const TX_ID = /^[0-9a-f]{64}$/;
const TX_CBOR = /^[0-9a-f]+$/;
const MAX_TRANSACTION_HEX_LENGTH = 32_768;

export class L1Operation {
  constructor(state) {
    this.state = state;
  }

  async fetch(request) {
    const path = new URL(request.url).pathname;
    if (request.method === "GET") {
      const operation = await this.state.storage.get("operation");
      return operation
        ? Response.json(operation)
        : Response.json({ error: "operation not found" }, { status: 404 });
    }

    const body = await request.json();
    if (path === "/prepare") {
      const result = await this.state.storage.transaction(async (storage) => {
        const existing = await storage.get("operation");
        if (existing) {
          if (
            existing.expectedTransactionId !== body.expectedTransactionId ||
            existing.transactionHash !== body.transactionHash
          ) {
            return { conflict: true };
          }
          return { operation: existing, submit: false };
        }
        const operation = { ...body, state: "pending" };
        await storage.put("operation", operation);
        return { operation, submit: true };
      });
      return result.conflict
        ? Response.json({ error: "operation conflict" }, { status: 409 })
        : Response.json(result);
    }
    if (path === "/submitted" || path === "/confirmed") {
      const operation = await this.state.storage.get("operation");
      if (
        !operation ||
        body.transactionId !== operation.expectedTransactionId
      ) {
        return Response.json(
          { error: "transaction conflict" },
          { status: 409 },
        );
      }
      const updated = { ...operation, state: path.slice(1) };
      await this.state.storage.put("operation", updated);
      return Response.json(updated);
    }
    return Response.json(
      { error: "invalid operation request" },
      { status: 400 },
    );
  }
}

export async function endpointSubmitOperation(ctx) {
  let body;
  try {
    body = await ctx.req.json();
    validateSubmission(body);
  } catch (error) {
    return ctx.json({ error: error.message || "invalid submission" }, 400);
  }

  const object = operationObject(ctx, body.operation_id);
  const prepared = await object.fetch("https://operation/prepare", {
    method: "POST",
    body: JSON.stringify({
      operationId: body.operation_id,
      expectedTransactionId: body.expected_transaction_id,
      transactionHash: await sha256(body.transaction),
    }),
  });
  if (!prepared.ok)
    return new Response(prepared.body, {
      status: prepared.status,
      headers: prepared.headers,
    });
  const { operation, submit } = await prepared.json();
  if (!submit) return ctx.json(publicOperation(operation));

  return ctx.endpoint(undefined, async () => {
    const transactionId = await ctx.koios("/submittx", {
      method: "POST",
      body: Uint8Array.from(body.transaction.match(/../g), (byte) =>
        Number.parseInt(byte, 16),
      ),
      headers: { "Content-Type": "application/cbor" },
    });
    if (transactionId !== body.expected_transaction_id)
      throw new Error("upstream transaction id mismatch");
    const recorded = await object.fetch("https://operation/submitted", {
      method: "POST",
      body: JSON.stringify({ transactionId }),
    });
    if (!recorded.ok) throw recorded;
    return publicOperation(await recorded.json());
  });
}

export async function endpointGetOperation(ctx) {
  const operationId = ctx.req.param("operation_id");
  if (!UUID.test(operationId))
    return ctx.json({ error: "invalid operation id" }, 400);
  const object = operationObject(ctx, operationId);
  const stored = await object.fetch("https://operation/");
  if (!stored.ok)
    return new Response(stored.body, {
      status: stored.status,
      headers: stored.headers,
    });
  let operation = await stored.json();

  try {
    await ctx.blockfrost(`/txs/${operation.expectedTransactionId}`);
    if (operation.state !== "confirmed") {
      const confirmed = await object.fetch("https://operation/confirmed", {
        method: "POST",
        body: JSON.stringify({
          transactionId: operation.expectedTransactionId,
        }),
      });
      operation = await confirmed.json();
    }
  } catch (response) {
    if (response?.status !== 404) throw response;
  }
  return ctx.json(publicOperation(operation));
}

export function validateSubmission(body) {
  if (!body || typeof body !== "object" || Array.isArray(body))
    throw new Error("invalid submission");
  if (
    Object.keys(body).sort().join(",") !==
    "expected_transaction_id,operation_id,transaction"
  )
    throw new Error("invalid submission fields");
  if (!UUID.test(body.operation_id)) throw new Error("invalid operation id");
  if (!TX_ID.test(body.expected_transaction_id))
    throw new Error("invalid transaction id");
  if (
    typeof body.transaction !== "string" ||
    !TX_CBOR.test(body.transaction) ||
    body.transaction.length % 2 ||
    body.transaction.length > MAX_TRANSACTION_HEX_LENGTH
  ) {
    throw new Error("invalid transaction");
  }
}

function operationObject(ctx, operationId) {
  const id = ctx.env.L1_OPERATIONS.idFromName(operationId);
  return ctx.env.L1_OPERATIONS.get(id);
}

async function sha256(value) {
  const digest = await crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(value),
  );
  return Array.from(new Uint8Array(digest), (byte) =>
    byte.toString(16).padStart(2, "0"),
  ).join("");
}

function publicOperation(operation) {
  return {
    operation_id: operation.operationId,
    transaction_id: operation.expectedTransactionId,
    state: operation.state,
  };
}

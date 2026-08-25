import { describe, expect, it } from "vitest";
import {
  endpointGetOperation,
  endpointSubmitOperation,
  L1Operation,
  validateSubmission,
} from "./operations.mjs";

class MemoryStorage {
  values = new Map();
  lock = Promise.resolve();
  async get(key) {
    return this.values.get(key);
  }
  async put(key, value) {
    this.values.set(key, value);
  }
  async transaction(action) {
    const previous = this.lock;
    let release;
    this.lock = new Promise((resolve) => {
      release = resolve;
    });
    await previous;
    try {
      return await action(this);
    } finally {
      release();
    }
  }
}

class OperationNamespace {
  objects = new Map();
  idFromName(value) {
    return value;
  }
  get(id) {
    if (!this.objects.has(id))
      this.objects.set(id, new L1Operation({ storage: new MemoryStorage() }));
    const object = this.objects.get(id);
    return {
      fetch: (url, init) =>
        object.fetch(url instanceof Request ? url : new Request(url, init)),
    };
  }
}

const operationId = "12345678-1234-1234-1234-123456789abc";
const transactionId = "ab".repeat(32);
const submission = (
  transaction = "00",
  expected_transaction_id = transactionId,
) => ({
  operation_id: operationId,
  expected_transaction_id,
  transaction,
});

function submitContext(namespace, body, koios) {
  return {
    req: { json: async () => body },
    env: { L1_OPERATIONS: namespace },
    koios,
    json: (value, status = 200) => Response.json(value, { status }),
    endpoint: async (_, action) => Response.json(await action()),
  };
}

function lookupContext(namespace, blockfrost) {
  return {
    req: { param: () => operationId },
    env: { L1_OPERATIONS: namespace },
    blockfrost,
    json: (value, status = 200) => Response.json(value, { status }),
  };
}

describe("L1 operation contract", () => {
  it("validates canonical UUID, transaction id, fields, and bounded even hex", () => {
    expect(() => validateSubmission(submission())).not.toThrow();
    for (const body of [
      submission("0"),
      submission("zz"),
      submission("00", "AB".repeat(32)),
      { ...submission(), operation_id: "1234" },
      { ...submission(), extra: true },
    ])
      expect(() => validateSubmission(body)).toThrow();
    expect(() => validateSubmission(submission("00".repeat(16_385)))).toThrow();
  });

  it("rejects conflicting reuse without a second upstream submission", async () => {
    const namespace = new OperationNamespace();
    let submissions = 0;
    const koios = async () => {
      submissions += 1;
      return transactionId;
    };
    expect(
      (
        await endpointSubmitOperation(
          submitContext(namespace, submission(), koios),
        )
      ).status,
    ).toBe(200);
    const conflict = await endpointSubmitOperation(
      submitContext(namespace, submission("01"), koios),
    );
    expect(conflict.status).toBe(409);
    expect(submissions).toBe(1);
  });

  it("collapses concurrent duplicate submissions to one upstream mutation", async () => {
    const namespace = new OperationNamespace();
    let submissions = 0;
    const koios = async () => {
      submissions += 1;
      await new Promise((resolve) => setTimeout(resolve, 5));
      return transactionId;
    };
    const responses = await Promise.all([
      endpointSubmitOperation(submitContext(namespace, submission(), koios)),
      endpointSubmitOperation(submitContext(namespace, submission(), koios)),
    ]);
    expect(responses.every((response) => response.status === 200)).toBe(true);
    expect(submissions).toBe(1);
  });

  it("reconciles pending and accepted operations by the expected chain transaction", async () => {
    const namespace = new OperationNamespace();
    const object = namespace.get(operationId);
    await object.fetch("https://operation/prepare", {
      method: "POST",
      body: JSON.stringify({
        operationId,
        expectedTransactionId: transactionId,
        transactionHash: "11".repeat(32),
      }),
    });

    const missing = new Response(null, { status: 404 });
    const pending = await endpointGetOperation(
      lookupContext(namespace, async () => {
        throw missing;
      }),
    );
    expect(await pending.json()).toEqual({
      operation_id: operationId,
      transaction_id: transactionId,
      state: "pending",
    });

    const confirmed = await endpointGetOperation(
      lookupContext(namespace, async () => ({ hash: transactionId })),
    );
    expect(await confirmed.json()).toEqual({
      operation_id: operationId,
      transaction_id: transactionId,
      state: "confirmed",
    });
  });

  it("retains an accepted operation when the submit response is discarded", async () => {
    const namespace = new OperationNamespace();
    await endpointSubmitOperation(
      submitContext(namespace, submission(), async () => transactionId),
    );
    const reconciled = await endpointGetOperation(
      lookupContext(namespace, async () => ({ hash: transactionId })),
    );
    expect((await reconciled.json()).transaction_id).toBe(transactionId);
    expect(
      (
        await namespace
          .get(operationId)
          .fetch("https://operation/")
          .then((response) => response.json())
      ).state,
    ).toBe("confirmed");
  });
});

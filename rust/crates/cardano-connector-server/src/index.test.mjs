import * as fs from 'node:fs';
import { describe, it, expect } from "vitest";
import vitestOpenAPI from 'vitest-openapi';
import axios from 'axios'

vitestOpenAPI(__dirname + '/../openapi.yaml');

const BASE_URL = "http://localhost:8787";

const FIXTURE_ADDR_1 = "addr_test1wp0mhyfzxh9r0yuuwr29py0smf7a58srkjwltt94pnjrqlshl0e5m";
const FIXTURE_ADDR_2 = "addr_test1vrpynvza5vswczszkjhe5cvqz2awmzukf84xa5wway8durqpmfm2m";
const FIXTURE_ADDR_UNKNOWN = "addr_test1wp0mhyfzxh9r0yuuwr29py0smf7a58srkjwltt94pnjrqlcftt42x";

const FIXTURE_TX_1 = "2dab762f41753d3fd6561d89b3169b4ff5c3060ccaa843d7dacbe38031cd1c62";
const FIXTURE_TX_2 = "46420909302a95c05a2ae50175b442e5e52aa46c64438a68c263303c046d229b";
const FIXTURE_TX_UNKNOWN = "0000000000000000000000000000000000000000000000000000000000000000";


describe("/health", () => {
  it("responds with 200 OK", async () => {
    const response = await axios.get(`${BASE_URL}/health`);
    expect(response.status).toBe(200);
    expect(response).toSatisfyApiSpec();
  });
});

describe("/network", () => {
  it("responds with 200 OK", async () => {
    const response = await axios.get(`${BASE_URL}/network`);
    expect(response.status).toBe(200);
    expect(response).toSatisfyApiSpec();
  });
});

describe("/balance/:address", () => {
  it("responds with 200 and value when address exists", async () => {
    const response = await axios.get(`${BASE_URL}/balance/${FIXTURE_ADDR_1}`);
    expect(response.status).toBe(200);
    expect(response).toSatisfyApiSpec();
  });

  it("responds with 200 and zero value when address does not exist ", async () => {
    const response = await axios.get(`${BASE_URL}/balance/${FIXTURE_ADDR_UNKNOWN}`);
    expect(response.status).toBe(200);
    expect(response).toSatisfyApiSpec();
    expect(response.data).toStrictEqual({ lovelace: "0" });
  });
});

describe("/utxos_at/:address", () => {
  it("responds with 200 and utxos when address exists (1)", async () => {
    const response = await axios.get(`${BASE_URL}/utxos_at/${FIXTURE_ADDR_1}`);
    expect(response.status).toBe(200);
    expect(response).toSatisfyApiSpec();
  });

  it("responds with 200 and utxos when address exists (2)", async () => {
    const response = await axios.get(`${BASE_URL}/utxos_at/${FIXTURE_ADDR_2}`);
    expect(response.status).toBe(200);
    expect(response).toSatisfyApiSpec();
  });

  it("responds with 200 and empty list when address does not exist ", async () => {
    const response = await axios.get(`${BASE_URL}/utxos_at/${FIXTURE_ADDR_UNKNOWN}`);
    expect(response.status).toBe(200);
    expect(response).toSatisfyApiSpec();
    expect(response.data).toStrictEqual([]);
  });
})

describe("/transactions/:address", () => {
  it("responds with 200 and transactions when address exists (1)", async () => {
    const response = await axios.get(`${BASE_URL}/transactions/${FIXTURE_ADDR_1}`);
    expect(response.status).toBe(200);
    expect(response).toSatisfyApiSpec();
  });

  it("responds with 200 and transactions when address exists (2)", async () => {
    const response = await axios.get(`${BASE_URL}/transactions/${FIXTURE_ADDR_2}`);
    expect(response.status).toBe(200);
    expect(response).toSatisfyApiSpec();
  });

  it("responds with 200 and empty list when address does not exist ", async () => {
    const response = await axios.get(`${BASE_URL}/transactions/${FIXTURE_ADDR_UNKNOWN}`);
    expect(response.status).toBe(200);
    expect(response).toSatisfyApiSpec();
    expect(response.data).toStrictEqual([]);
  });
})

describe("/transactions/:id", () => {
  it("responds with 200 and transactions when id exists (1)", async () => {
    const response = await axios.get(`${BASE_URL}/transaction/${FIXTURE_TX_1}`);
    expect(response.status).toBe(200);
    expect(response).toSatisfyApiSpec();
  });

  it("responds with 200 and transactions when id exists (2)", async () => {
    const response = await axios.get(`${BASE_URL}/transaction/${FIXTURE_TX_1}`);
    expect(response.status).toBe(200);
    expect(response).toSatisfyApiSpec();
  });

  it("responds with 200 and null when address does not exist ", async () => {
    const response = await axios.get(`${BASE_URL}/transaction/${FIXTURE_TX_UNKNOWN}`);
    expect(response.status).toBe(200);
    expect(response).toSatisfyApiSpec();
    expect(response.data).toStrictEqual(null);
  });
})

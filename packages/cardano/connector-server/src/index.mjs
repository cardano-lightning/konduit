import { Hono } from "hono";

import { endpointBalance } from "./endpoints/balance.mjs";
import { endpointDocs, endpointOpenApi } from "./endpoints/openapi.mjs";
import { endpointNetwork } from "./endpoints/network.mjs";
import { endpointProtocolParameters } from "./endpoints/protocol_parameters.mjs";
import {
  endpointGetOperation,
  endpointSubmitOperation,
  L1Operation,
} from "./endpoints/operations.mjs";
import { endpointSubmit } from "./endpoints/submit.mjs";
import {
  endpointSessionCheck,
  endpointSessionClaim,
  WalletSession,
} from "./endpoints/session.mjs";
import { endpointTransaction } from "./endpoints/transaction.mjs";
import { endpointTransactions } from "./endpoints/transactions.mjs";
import { endpointUtxosAt } from "./endpoints/utxos_at.mjs";

import blockfrost from "./middleware/blockfrost.mjs";
import cors from "./middleware/cors.mjs";
import endpoint from "./middleware/endpoint.mjs";
import koios from "./middleware/koios.mjs";

const app = new Hono();

app.use(cors);
app.use(endpoint);
app.use(blockfrost);
app.use(koios);

app.get("/", endpointDocs);
app.get("/openapi.yaml", endpointOpenApi);
app.get("/balance/:address", endpointBalance);
app.get("/network", endpointNetwork);
app.post("/operations", endpointSubmitOperation);
app.get("/operations/:operation_id", endpointGetOperation);
app.get("/protocol-parameters", endpointProtocolParameters);
app.post("/submit", endpointSubmit);
app.post("/session/claim", endpointSessionClaim);
app.get("/session/check", endpointSessionCheck);
app.get("/utxos_at/:address", endpointUtxosAt);
app.get("/transaction/:id", endpointTransaction);
app.get("/transactions/:address", endpointTransactions);

app.get("/health", (ctx) => ctx.json({ status: "ok" }));

export default app;

export { L1Operation, WalletSession };

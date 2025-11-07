import { Hono } from "hono";

import { endpointNetwork } from "./endpoints/network.mjs";
import { endpointUtxosAt } from "./endpoints/utxos_at.mjs";

import blockfrost from "./middleware/blockfrost.mjs";
import cors from "./middleware/cors.mjs";

const app = new Hono();

app.use(cors)
app.use(blockfrost);

app.get("/network", endpointNetwork);
app.get("/health", (ctx) => ctx.json({ status: "ok" }));
app.get("/utxos_at/:address", endpointUtxosAt);

export default app;

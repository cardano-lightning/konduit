import { Hono } from "hono";

import { endpointBalance } from "./endpoints/balance.mjs";
import { endpointNetwork } from "./endpoints/network.mjs";
import { endpointSubmit } from "./endpoints/submit.mjs";
import { endpointUtxosAt } from "./endpoints/utxos_at.mjs";

import blockfrost from "./middleware/blockfrost.mjs";
import koios from "./middleware/koios.mjs";
import cors from "./middleware/cors.mjs";

const app = new Hono();

app.use(cors);
app.use(blockfrost);
app.use(koios);

app.get("/health", (ctx) => ctx.json({ status: "ok" }));

app.get("/balance/:address", endpointBalance);
app.get("/network", endpointNetwork);
app.post("/submit", endpointSubmit);
app.get("/utxos_at/:address", endpointUtxosAt);

export default app;

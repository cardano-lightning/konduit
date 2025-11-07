import { Hono } from "hono";
import { endpointNetwork } from "./endpoints/network.mjs";
import { endpointUtxosAt } from "./endpoints/utxos_at.mjs";
import blockfrost from "./middleware/blockfrost.mjs";

const app = new Hono();

app.use(blockfrost);
app.get("/network", endpointNetwork);
app.get("/health", (ctx) => ctx.json({ status: "ok" }));
app.get("/utxos_at/:address", endpointUtxosAt);

export default app;

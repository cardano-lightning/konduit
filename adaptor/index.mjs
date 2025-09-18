import Koa from 'koa';
import Router from '@koa/router';
import { Agent } from 'undici';
import * as fs from 'node:fs';

const MIME_TYPES = {
  JSON: "application/json; charset=utf-8",
};

const HEADERS = {
  CONTENT_TYPE: "Content-Type",
  GRPC_METADATA_MACAROON: "Grpc-Metadata-Macaroon",
};

const DEFAULTS = {
  LISTEN_PORT: 4444,
};

const ENV_PREFIX = "KONDUIT_ADAPTOR";

const CONFIG = {
  LISTEN_PORT: Number.parseInt(get_env("LISTEN_PORT"), 10),
  LN_BASE_URL: get_env("LN_BASE_URL"),
  MACAROON: fs.readFileSync(get_env("LN_MACAROON")).toString("hex"),
  TLS_CERTIFICATE: fs.readFileSync(get_env("LN_TLS_CERT")),
};

const router = new Router();
router.get('/quote/:payload', get_quote)

const app = new Koa();
app
  .use(set_content_type(MIME_TYPES.JSON))
  .use(router.routes())
  .use(router.allowedMethods())
  .listen(CONFIG.LISTEN_PORT, () => {
    console.log(`listening on port ${CONFIG.LISTEN_PORT}`);
  });

function get_env(key) {
  const env_var = process.env[`${ENV_PREFIX}_${key}`] ?? DEFAULTS[key];

  if (env_var === undefined) {
    throw new Error(`missing (required) ENV var ${ENV_PREFIX}_${key}`);
  }

  return env_var;
}

async function get_quote(ctx) {
  const payload = ctx.params.payload;

  const quote = await ln_decode_quote(payload);

  const expiry = new Date(
    Number.parseInt(quote.timestamp, 10) +
    Number.parseInt(quote.expiry + 10)
  );

  ctx.body = JSON.stringify({
    recipient: quote.destination,
    payment_hash: quote.payment_hash,
    routing_fee: 0, // FIXME: Obtain from computing the payment route.
    expiry: expiry.toISOString(),
  });
}

function set_content_type(content_type) {
  return (ctx, next) => {
    ctx.set(HEADERS.CONTENT_TYPE, content_type);
    return next();
  }
}

function ln_decode_quote(payload) {
  return ln_get(`v1/payreq/${payload}`);
}

async function ln_get(path) {
  const res = await fetch(`${CONFIG.LN_BASE_URL}/${path}`, {
    dispatcher,
    headers: {
      [HEADERS.GRPC_METADATA_MACAROON]: CONFIG.MACAROON,
    },
  });

  return res.json();
}

const dispatcher = new Agent({
  connect: {
    ca: CONFIG.TLS_CERTIFICATE,
    keepAlive: true
  }
});

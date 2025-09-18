import { get_env } from './env.mjs';
import * as fs from 'node:fs';

const DEFAULTS = {
  /** TCP port to listen to for incoming client connections to the Adaptor server */
  LISTEN_PORT: 4444,

  /** Fixed amount charged by the Adaptor for routing payments, in milli-satoshis */
  ADAPTOR_FEE: 42n,
};

export default {
  LISTEN_PORT: Number.parseInt(get_env("LISTEN_PORT", DEFAULTS), 10),
  LN_BASE_URL: get_env("LN_BASE_URL", DEFAULTS),
  LN_MACAROON: fs.readFileSync(get_env("LN_MACAROON", DEFAULTS)),
  LN_TLS_CERT: fs.readFileSync(get_env("LN_TLS_CERT", DEFAULTS)),
  ADAPTOR_FEE: BigInt(get_env("ADAPTOR_FEE", DEFAULTS)),
};

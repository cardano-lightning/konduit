import { get_env } from './env.mjs';
import * as fs from 'node:fs';

const DEFAULTS = {
  LISTEN_PORT: 4444,
};

export default {
  LISTEN_PORT: Number.parseInt(get_env("LISTEN_PORT", DEFAULTS), 10),
  LN_BASE_URL: get_env("LN_BASE_URL", DEFAULTS),
  LN_MACAROON: fs.readFileSync(get_env("LN_MACAROON", DEFAULTS)),
  LN_TLS_CERT: fs.readFileSync(get_env("LN_TLS_CERT", DEFAULTS)),
};

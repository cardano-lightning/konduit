import { get_env_option, get_env } from "./env.mjs";
import * as fs from "node:fs";

const DEFAULTS = {
  /** TCP port to listen to for incoming client connections to the Adaptor server */
  LISTEN_PORT: 4444,

  /** Fixed amount charged by the Adaptor for routing payments, in milli-satoshis */
  FEE: 42n,
};

export default {
  LISTEN_PORT: Number.parseInt(get_env("LISTEN_PORT", DEFAULTS), 10),
  LN_BASE_URL: get_env("LN_BASE_URL", DEFAULTS),
  LN_MACAROON: getMacaroon(),
  LN_TLS_CERT: getTlsCert(),
  ADAPTOR_FEE: BigInt(get_env("FEE", DEFAULTS)),
};

function getMacaroon() {
  const maybeMacaroonHex = get_env_option("LN_MACAROON_HEX");
  if (maybeMacaroonHex) {
    console.log(Buffer.from(maybeMacaroonHex, "hex"));
    return Buffer.from(maybeMacaroonHex, "hex");
  }
  return fs.readFileSync(get_env("LN_MACAROON", DEFAULTS));
}

function getTlsCert() {
  const maybeTlsCertPath = get_env_option("LN_TLS_CERT", DEFAULTS);
  if (maybeTlsCertPath) {
    return fs.readFileSync(maybeTlsCertPath);
  } else {
    return null;
  }
}

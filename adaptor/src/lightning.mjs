import * as assert from "node:assert";
import { HEADERS } from "./constants.mjs";
import { Agent } from "undici";

export default class LightningClient {
  #base_url;
  #dispatcher;
  #macaroon;

  constructor(base_url, tls_certificate, macaroon) {
    assert.ok(
      typeof base_url === "string",
      "missing/invalid lightning base url",
    );
    assert.ok(macaroon instanceof Buffer, "missing/invalid macaroon");

    this.#base_url = base_url;

    this.#dispatcher = new Agent({
      connect: {
        ...(tls_certificate
          ? { ca: tls_certificate }
          : { rejectUnauthorized: false }),
        keepAlive: true,
      },
    });

    this.#macaroon = macaroon.toString("hex");
  }

  async decode_payment_request(payload) {
    return this.#get(`v1/payreq/${payload}`);
  }

  /**
   * @param recipient {String} 33-byte public key (base16-encoded) of the recipient.
   * @param amount {BigInt} Amount to send (i.e. received by the recipient), in milli-satoshis.
   */
  async query_route(recipient, amount) {
    return this.#get(`v1/graph/routes/${recipient}/0?amt_msat=${amount}`);
  }

  async #get(path) {
    const res = await fetch(`${this.#base_url}/${path}`, {
      dispatcher: this.#dispatcher,
      headers: {
        [HEADERS.GRPC_METADATA_MACAROON]: this.#macaroon,
      },
    });

    if (res.status >= 400) {
      throw new Error(`unable to GET ${path}`, { cause: await res.json() });
    }
    return res.json();
  }
}

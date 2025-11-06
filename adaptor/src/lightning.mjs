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

  /**
   * @param recipient {String} 33-byte public key (base16-encoded) of the recipient.
   * @param amount {BigInt} Amount to send (i.e. received by the recipient), in milli-satoshis.
   */
  async pay({ dest, paymentHash, paymentAddr, amount, fixedFeeLimit }) {
    console.log({ dest, paymentHash, paymentAddr, amount, fixedFeeLimit });
    let requestBody = {
      dest: dest.toString("base64"),
      payment_hash: paymentHash.toString("base64"),
      amt_msat: amount,
      final_cltv_delta: 100,
      fee_limit: { fixed: fixedFeeLimit },
      payment_addr: paymentAddr,
    };
    //   dest: <string>, // <bytes> (base64 encoded)
    //   dest_string: <string>, // <string>
    //   amt: <string>, // <int64>
    //   amt_msat: <string>, // <int64>
    //   payment_hash: <string>, // <bytes> (base64 encoded)
    //   payment_hash_string: <string>, // <string>
    //   payment_request: <string>, // <string>
    //   final_cltv_delta: <integer>, // <int32>
    //   fee_limit: <object>, // <FeeLimit>
    //   outgoing_chan_id: <string>, // <uint64>
    //   last_hop_pubkey: <string>, // <bytes> (base64 encoded)
    //   cltv_limit: <integer>, // <uint32>
    //   dest_custom_records: <object>, // <DestCustomRecordsEntry>
    //   allow_self_payment: <boolean>, // <bool>
    //   dest_features: <array>, // <FeatureBit>
    //   payment_addr: <string>, // <bytes> (base64 encoded)
    // };
    console.log(requestBody);
    return this.#post(`v1/channels/transactions`, requestBody);
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

  async #post(path, body) {
    const res = await fetch(`${this.#base_url}/${path}`, {
      dispatcher: this.#dispatcher,
      method: "POST",
      headers: {
        [HEADERS.GRPC_METADATA_MACAROON]: this.#macaroon,
      },
      body: JSON.stringify(body),
    });

    if (res.status >= 400) {
      throw new Error(`unable to GET ${path}`, { cause: await res.json() });
    }
    return res.json();
  }
}

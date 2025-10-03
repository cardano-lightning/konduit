import config from "../config.mjs";
import * as Json from "@cardanosolutions/json-bigint";

export function get_quote(lightning) {
  return async (ctx) => {
    const payload = ctx.params.payload;

    const quote = await ctx.effect("decode payment request", () =>
      lightning.decode_payment_request(payload),
    );

    const recipient = quote.destination;

    const expiry = new Date(
      Number.parseInt(quote.timestamp, 10) + Number.parseInt(quote.expiry + 10),
    );

    const amount = BigInt(quote.num_msat);

    const { routes } = await ctx.effect("query route", () =>
      lightning.query_route(recipient, amount),
    );

    const first_route = routes[0];

    if (first_route.length <= 0) {
      return ctx.throw(400, `no available route to recipient: ${recipient}`);
    }

    ctx.body = Json.stringify({
      amount,
      recipient,
      payment_hash: quote.payment_hash,
      routing_fee: BigInt(first_route.total_fees_msat) + config.ADAPTOR_FEE,
      expiry: expiry.toISOString(),
    });
  };
}

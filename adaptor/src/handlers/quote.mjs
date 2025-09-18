export function get_quote(lightning) {
  return async (ctx) => {
    const payload = ctx.params.payload;

    const quote = await ctx.effect(
      "decode payment request",
      () => lightning.decode_payment_request(payload),
    );

    const expiry = new Date(
      Number.parseInt(quote.timestamp, 10) +
      Number.parseInt(quote.expiry + 10)
    );

    ctx.body = {
      recipient: quote.destination,
      payment_hash: quote.payment_hash,
      routing_fee: 0, // FIXME: Obtain from computing the payment route.
      expiry: expiry.toISOString(),
    };
  };
}

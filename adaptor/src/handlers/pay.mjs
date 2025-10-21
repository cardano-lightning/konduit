import config from "../config.mjs";
import * as Json from "@cardanosolutions/json-bigint";
import { unpack } from "../cheque.mjs";

export function pay(lightning) {
  return async (ctx) => {
    const { tag, verificationKey, paymentAddr, cheque, dest, msat } =
      ctx.request.body;
    const { index, amount, timeout, lock } = unpack({
      tag,
      verificationKey,
      cheque,
    });
    console.log(cheque, dest, msat);

    const fixedFeeLimit = 42;
    const result = await ctx.effect("PAY", () =>
      lightning.pay({
        dest: Buffer.from(dest, "hex"),
        paymentHash: lock,
        amount: msat,
        fixedFeeLimit,
        paymentAddr,
      }),
    );
    console.log(result);

    ctx.body = Json.stringify(result);
  };
}

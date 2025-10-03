export function effect(ctx, next) {
  ctx.effect = async (title, run) => {
    let result;

    try {
      result = await run();
    } catch (e) {
      const error = new Error(`failed to ${title}`, { cause: e });
      error.status = 500;
      return ctx.throw(error);
    }

    return result;
  };

  return next();
}

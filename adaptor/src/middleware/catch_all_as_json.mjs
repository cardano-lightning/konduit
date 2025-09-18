export function catch_all_as_json (ctx, next) {
  if (typeof ctx.response.body !== "object") {
    return ctx.throw(ctx.response.status, { "error": ctx.response.message });
  }

  return next();
}

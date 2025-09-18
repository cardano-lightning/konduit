export function catch_all_as_json (ctx, next) {
  if (ctx.response.status >= 400) {
    const status = ctx.response.status;
    ctx.response.body = `"${ctx.response.body ?? ctx.response.message}"`;
    ctx.response.status = status;
  }

  return next();
}

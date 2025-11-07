import * as hono from 'hono/cors'

export default async function cors(ctx, next) {
  const middleware = hono.cors({
    origin: ctx.env.CORS_ORIGIN,
    allowHeaders: ["*"],
    allowMethods: ["HEAD", "GET", "POST", "OPTIONS"],
  });

  return middleware(ctx, next);
}

export default async function blockfrost(ctx, next) {
  const blockfrostId = await ctx.env.BLOCKFROST_PROJECT_ID.get();
  const network = blockfrostId.slice(0, 7);
  const baseUrl = `https://cardano-${network}.blockfrost.io/api/v0`;

  ctx.network = network;
  ctx.blockfrost = async (path, opts = {}) => {
    path = path.startsWith("/") ? path : `/${path}`;
    const res = await fetch(`${baseUrl}${path}`, {
      ...opts,
      headers: {
        ...opts.headers,
        project_id: blockfrostId,
      },
    });

    if (res.ok) {
      return res.json();
    }

    throw res
  };

  return next();
}

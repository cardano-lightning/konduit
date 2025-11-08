export default async function koios(ctx, next) {
  const baseUrl = ctx.network === "mainnet"
    ? "https://api.koios.rest/api/v1"
    : `https://${ctx.network}.koios.rest/api/v1`;

  ctx.koios = async (path, opts = {}) => {
    path = path.startsWith("/") ? path : `/${path}`;
    const res = await fetch(`${baseUrl}${path}`, opts);

    if (res.ok) {
      return res.json();
    }

    throw res
  };

  return next();
}

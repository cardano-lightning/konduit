export default async function endpoint(ctx, next) {
  /**
   * A wrapper with generic error handling for endpoints.
   * @param {any} [defaultValue] A value to be returned when the fetched resource is missing.
   * @param {function} callback The asynchronous callback for the endpoint. Expected to return JSON.
   */
  ctx.endpoint = async (defaultValue, callback) => {
    try {
      return ctx.json(await callback());
    } catch (res) {
      if (defaultValue !== undefined && res.status === 404) {
        return ctx.json(defaultValue);
      }

      if (res.status && res.statusText) {
        console.log(`${res.status} ${res.statusText}: ${await res.text()}`);
      } else {
        console.log(res);
      }

      throw "unexpected error";
    }
  };

  return next();
}

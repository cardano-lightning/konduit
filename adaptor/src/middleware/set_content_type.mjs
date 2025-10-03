import { HEADERS } from "../constants.mjs";

export function set_content_type(content_type) {
  return (ctx, next) => {
    ctx.set(HEADERS.CONTENT_TYPE, content_type);
    return next();
  };
}

import { nanoid } from 'nanoid'

export function request_span(logger) {
  return async (ctx, next) => {
    const timestamp = new Date();

    const id = nanoid();

    ctx.logger = logger.child({ id });

    let payload = {
      id,
      method: ctx.request.method,
      path: ctx.request.path,
    };

    try {
      await next();
    } catch (error) {
      const ms = Date.now() - timestamp.getTime();

      let stack = [];
      let node = error;
      while (node?.cause != null) {
        stack.push(node.cause.message ?? String(node.cause));
        node = node.cause;
      }

      logger.error({
        ...payload,
        status: error?.status ?? ctx.res.statusCode,
        error: error?.message ?? String(error),
        stack,
        ms,
      });

      ctx.status = error?.status ?? ctx.res.statusCode;
      ctx.body = `"${error?.message ?? String(error)}"`;
      return;
    }

    const ms = Date.now() - timestamp.getTime();
    logger.info({ ...payload, status: ctx.res.statusCode, ms });
  };
}

import Koa from 'koa';
import Router from '@koa/router';
import { MIME_TYPES } from './constants.mjs';
import { set_content_type, catch_all_as_json, effect } from './middleware/index.mjs';
import * as handlers from './handlers/index.mjs';
import config from './config.mjs';
import LightningClient from './lightning.mjs';

const lightning_client = new LightningClient(
  config.LN_BASE_URL,
  config.LN_TLS_CERT,
  config.LN_MACAROON,
);

const router = new Router();
router.get('/quote/:payload', handlers.get_quote(lightning_client))

const app = new Koa();
app
  .use(effect)
  .use(set_content_type(MIME_TYPES.JSON))
  .use(router.routes())
  .use(router.allowedMethods())
  .use(catch_all_as_json)
  .listen(config.LISTEN_PORT, () => {
    console.log(`listening on port ${config.LISTEN_PORT}`);
  });

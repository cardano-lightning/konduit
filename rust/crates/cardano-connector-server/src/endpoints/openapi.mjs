import OPENAPI_YAML from "../../openapi.yaml";

export const REDOC_HTML = `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Cardano Connector Server API</title>
    <style>
      html, body {
        margin: 0;
        padding: 0;
        height: 100%;
        background: #ffffff;
      }
      redoc {
        display: block;
        height: 100%;
      }
    </style>
  </head>
  <body>
    <redoc spec-url="/openapi.yaml"></redoc>
    <script src="https://cdn.jsdelivr.net/npm/redoc@next/bundles/redoc.standalone.js"></script>
  </body>
</html>
`;

export async function endpointOpenApi(ctx) {
  ctx.header("content-type", "application/yaml; charset=utf-8");
  return ctx.body(OPENAPI_YAML);
}

export async function endpointDocs(ctx) {
  return ctx.html(REDOC_HTML);
}

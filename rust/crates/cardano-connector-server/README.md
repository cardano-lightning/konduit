# Cardano Connector Server

Cloudflare Worker exposing a small Cardano connector API backed by Blockfrost and Koios.

## Pre-requisites

- `wrangler >= 4.46.0` or `npx >= 10.9.3`
- `jq >= 1.8.0`

## Getting started

#### Environments

This worker must be started or deployed with an explicit Wrangler environment.

Available environments in [`wrangler.jsonc`](/Users/ktorz/Documents/Projects/Konduit/rust/crates/cardano-connector-server/wrangler.jsonc):

- `preprod`
- `mainnet`

#### Setting up secrets

The worker requires a `BLOCKFROST_PROJECT_ID` secret for each environment.

In this repository, each environment binds that variable from a Cloudflare Secrets Store entry:

| environment | secret name          |
| ---         | ---                  |
| `preprod`   | `blockfrost-preprod` |
| `mainnet`   | `blockfrost-mainnet` |

Initialize those secrets before running or deploying the worker, for example:


```console
npx wrangler secrets-store secret create \
  $(jq ".env.preprod.secrets_store_secrets[0].store_id" wrangler.jsonc) \
  --name blockfrost-preprod \
  --scopes workers
```

This will prompt you for the Blockfrost project id.

#### Running

```console
npx wrangler dev --env preprod
```

## Documentation

HTML documentation is available at the root of the server `/`. It is based on an OpenAPI specification available under `/openapi.yaml`

## Deployment

```console
npx wrangler deploy --env preprod
```

## Testing

```console
npm test
```

> [!NOTE]
>
> Tests are running end-to-end against a local server running on preprod. The
> server must be started independently of the test.

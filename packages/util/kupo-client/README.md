# Kupo client

> Minimal HTTP client for [Kupo], the Cardano chain-index.

[Kupo]: https://cardanosolutions.github.io/kupo/

## Overview

This crate provides a first-pass Rust client for the [Kupo HTTP API]. It
targets the v2.11.0 surface and exposes the most commonly used endpoints:

- `GET /health`
- `GET /matches` and `GET /matches/{pattern}` (with optional filters)
- `DELETE /matches/{pattern}`
- `GET /datums/{datum-hash}`
- `GET /scripts/{script-hash}`
- `GET /patterns`, `PUT /patterns`, `PUT /patterns/{pattern}`,
  `DELETE /patterns/{pattern}`
- `GET /checkpoints`, `GET /checkpoints/{slot-no}`
- `GET /metadata/{slot-no}`

The client is intentionally minimal: it depends only on `anyhow`, `reqwest`,
`serde`, `serde_json`, and `thiserror`. It returns plain Rust types that mirror
the JSON shapes documented in the API reference; conversions to/from the
project's `cardano-sdk` types are left for downstream consumers.

[Kupo HTTP API]: https://cardanosolutions.github.io/kupo/

## Library usage

```rust
use kupo_client::{Client, MatchFilters, Pattern, Order};

let client = Client::new("http://localhost:1442")?;

// Health
let health = client.health().await?;

// All matches
let matches = client.all_matches(&MatchFilters::default()).await?;

// Matches for a specific address
let pattern = Pattern::address("addr_vk1...");
let matches = client.matches(&pattern, &MatchFilters::new()).await?;

// Add a pattern with a forced rollback
let rollback = ForcedRollback::to(RollbackTo::slot(0));
client.put_pattern(&Pattern::wildcard(), &rollback).await?;
```

## CLI (`kupo-print`)

The crate ships a `kupo-print` binary (gated behind the `cli` feature) that
queries a running Kupo server and pretty-prints matches as JSON. It's meant
for smoke-testing the library end-to-end against a real server.

Enable and run:

```bash
cargo run -p kupo-client --features cli --bin kupo-print -- \
    --host 127.0.0.1 --port 1442 --pattern "addr_vk1.../*" --limit 5
```

Available flags (every option also reads from the environment):

| Flag               | Env var               | Default     | Meaning                                           |
| ------------------ | --------------------- | ----------- | ------------------------------------------------- |
| `--host`           | `KUPO_HOST`           | `127.0.0.1` | Kupo server hostname or IP                        |
| `--port`           | `KUPO_PORT`           | `1442`      | Kupo server TCP port                              |
| `--pattern`        | `KUPO_PATTERN`        | _(none)_    | If given, only that pattern's matches are fetched |
| `--limit`          | `KUPO_LIMIT`          | `10`        | Cap on the number of matches printed              |
| `--resolve-hashes` | `KUPO_RESOLVE_HASHES` | `false`     | Resolve and inline datum/script references        |

A `.env` file in the working directory is also loaded at startup
(`dotenvy::dotenv().ok()`).

## Notes

Patterns that contain forward slashes (e.g. `addr_vk.../*`) are percent-encoded
when interpolated into path segments so they aren't mis-interpreted as path
separators by the URL parser.

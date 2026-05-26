# problem-details-server

Server integration for `problem-details-wire`. Provides transport-agnostic
helpers and optional framework bindings via feature flags.

## Features

| Feature | Enables                                                      |
| ------- | ------------------------------------------------------------ |
| `actix` | `Problem<E>` wrapper implementing `actix_web::ResponseError` |

## Usage

```toml
# konduit-server-actix/Cargo.toml
problem-details-server = { workspace = true, features = ["actix"] }
```

Without any feature flag, `into_parts()` is available for custom integrations:

```rust,ignore
let (status, content_type, body) = problem_details_server::into_parts(&err);
```

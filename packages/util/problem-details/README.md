# `problem-details`: Design & Usage

## Problem

In a multi-crate service, errors need to be:

- **Machine-readable** — clients parse and act on them, not just display them
- **Stable** — error identities don't change between deploys
- **Traceable** — a production error should lead a developer directly to the
  source definition, even without a running server
- **Consistent** — the same error from different features looks the same on the
  wire

Ad-hoc error responses (string messages, ad-hoc JSON shapes) fail all four.

## Standard

[RFC 9457](https://www.rfc-editor.org/rfc/rfc9457) defines _Problem Details_ — a
standard shape for HTTP error responses. We follow it with CBOR encoding
(`application/problem+cbor`, registered in RFC 9457 §8.1) rather than JSON,
since our transport is binary. JSON is also supported for mixed clients.

Wire shape:

```sample
{
  type:     URI identifying the error type      -- stable, points at docs/source
  title:    short human-readable summary        -- stable, same for all occurrences
  status:   HTTP status code                    -- u16
  detail:   explanation of this occurrence      -- optional, runtime context
  instance: URI identifying this occurrence     -- optional
}
```

## Design decisions

### `type` URI encodes crate identity

```sample
{PROBLEM_DETAIL_BASE_URL}/{crate_name}/{crate_version}/{slug}
# e.g.
https://konduit.channel/errors/konduit-auth/0.3.1/unauthorized
```

This means:

- The URI is stable within a crate version
- A developer can find the exact source without a running server: check out
  `konduit-auth` at `0.3.1`, search for `unauthorized`
- The static error site is organised by crate and version naturally

`PROBLEM_DETAIL_BASE_URL` is set once at the workspace level in
`.cargo/config.toml`. `crate_name` and `crate_version` resolve via `env!()` in
the consumer crate at compile time — they always reflect the crate that
_defines_ the error, not the crate that _serves_ it.

### HTTP status codes are transport-agnostic vocabulary

`http_status` is a `u16` on the trait, not an `http::StatusCode`. HTTP status
codes have become a universal error classification vocabulary — gRPC maps to
them (Google AIP-193), and they're well understood. Integration crates
(`problem-details-server`, a future `problem-details-grpc`) do the conversion to
their native type.

### `slug` is a first-class field

`slug` is exposed as a separate trait method, not just embedded in the URI. This
makes it directly available to transports that want a short stable identifier
without parsing a URI — gRPC's `ErrorInfo.reason` being the primary case.

### Runtime context via `WithDetail`

Error _types_ are static — slug, title, status, and type URI never change for a
given variant. Runtime context (which field failed, what the clock skew was,
which resource was missing) is added at the call site via `ProblemDetailExt`:

```rust,ignore
return Err(Error::Unauthorized.with_detail(format!("clock skew was {skew}s")));
```

This keeps the derive macro simple and ensures static properties stay static.

### Error hierarchies via `#[problem(delegate)]`

Features share common error vocabularies (e.g. `InsufficientFunds` is common to
all payment features). Rather than duplicating variants, a feature enum wraps a
common enum with a delegate variant:

```rust,ignore
#[derive(ProblemDetail)]
pub enum Error {
    #[problem(slug = "invalid-invoice", title = "Invalid Invoice", http_status = 400)]
    InvalidInvoice,

    #[problem(delegate)]
    Common(common::Error),  // delegates all trait methods to inner value
}
```

The `type` URI of a delegated variant points at the crate where it's _defined_
(e.g. `konduit-common`), not the crate doing the delegating. This is correct:
the error identity belongs to its definition.

### Static site manifest

Every type deriving `ProblemDetail` emits three associated constants:

```rust,ignore
Error::PROBLEM_DETAILS_MANIFEST      // JSON: [{slug, title, http_status, doc}]
Error::PROBLEM_DETAILS_CRATE_NAME    // e.g. "konduit-auth"
Error::PROBLEM_DETAILS_CRATE_VERSION // e.g. "0.3.1"
```

Doc comments on variants are captured into the manifest at compile time. A
`tools/collect-manifests` binary aggregates these across all wire crates; CI
runs it and feeds the output to a static site generator. The error reference
site requires no running service.

### VCS traceability

With the `vcs-info` feature (on by default), the server crate's `build.rs`
captures the git hash at build time and embeds it in every error response. A
non-technical user can forward an error to a developer who can immediately check
out the exact commit, without access to the production server.

## Crate structure

Users only add `problem-details` to `Cargo.toml`. The derive crate
(`problem-details-derive`) is an implementation detail, re-exported
transparently.

For now, we use feature flags.

## Usage

### 1. Set the base URL

`.cargo/config.toml` at workspace root:

```toml
[env]
PROBLEM_DETAIL_BASE_URL = "https://konduit.channel/errors"
```

### 2. Define errors

```rust,ignore
use problem_details::ProblemDetail;

#[derive(Debug, ProblemDetail)]
pub enum Error {
    /// No `konduit-hmac-token` header present.
    #[problem(slug = "missing-token", title = "Missing Token", http_status = 401)]
    MissingToken,

    /// MAC verification failed.
    #[problem(slug = "unauthorized", title = "Unauthorized", http_status = 401)]
    Unauthorized,

    /// Keytag is not a recognised patron (no channel on record).
    #[problem(slug = "not-a-patron", title = "Not a Patron", http_status = 403)]
    NotPatron,

    /// Wraps errors common to all features.
    #[problem(delegate)]
    Common(common::Error),
}
```

### 3. Add runtime context

```rust,ignore
use problem_details::ProblemDetailExt;

return Err(Error::Unauthorized.with_detail(format!("clock skew was {skew}s")));
return Err(Error::NotPatron.with_instance("keytag has no channel", "/requests/99"));
```

### 4. Add `build.rs` for VCS info

The `vcs-info` feature (on by default) embeds the git hash in every error
response. Add this to your server crate's `build.rs`:

```rust,ignore
fn main() {
    // Rerun only when HEAD or branch refs change
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");

    let hash = git_hash().unwrap_or_else(|| "unknown".to_owned());
    println!("cargo:rustc-env=GIT_HASH={hash}");
}

fn git_hash() -> Option<String> {
    let hash = run_git(&["rev-parse", "--short", "HEAD"])?;
    let dirty = run_git(&["status", "--porcelain"])
        .map(|out| !out.is_empty())
        .unwrap_or(false);

    if dirty { Some(format!("{hash}-dirty")) } else { Some(hash) }
}

fn run_git(args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .output()
        .ok()?;

    if output.status.success() {
        String::from_utf8(output.stdout)
            .ok()
            .map(|s| s.trim().to_owned())
    } else {
        None
    }
}
```

`GIT_HASH` is then available as `env!("GIT_HASH")` and is picked up by
`problem-details-server` when constructing the response body. To opt out,
disable the default feature:

```toml
problem-details = { workspace = true, default-features = false }
```

### 5. Serve with actix-web

```rust,ignore
use problem_details_server::Problem;

async fn handler() -> Result<HttpResponse, Problem<Error>> {
    Err(Problem(Error::Unauthorized))
}

// With runtime context:
async fn handler() -> Result<HttpResponse, Problem<WithDetail<Error>>> {
    Err(Problem(Error::Unauthorized.with_detail("clock skew was 5s")))
}
```

### 6. Decode on the client

```rust,ignore
use problem_details::ProblemDetailBody;

let body: ProblemDetailBody = minicbor::decode(&bytes)?;
// or
let body: ProblemDetailBody = serde_json::from_slice(&bytes)?;
```

### 7. Collect manifests for static site

This is boilerplate you write once in your server crate (or a dedicated
`tools/collect-manifests` binary). There is no automatic aggregation — the
server crate is the first point in the build graph where all feature wire crates
are simultaneously in scope, so it is the only place that can collect them.
Adding a new feature requires a conscious update here, which is intentional.

```rust,ignore
// konduit-server/src/bin/collect_manifests.rs
// or tools/collect-manifests/src/main.rs
fn main() {
    let entries = serde_json::json!([
        {
            "crate":   featurex_wire::Error::PROBLEM_DETAILS_CRATE_NAME,
            "version": featurex_wire::Error::PROBLEM_DETAILS_CRATE_VERSION,
            "errors":  serde_json::from_str::<serde_json::Value>(
                           featurex_wire::Error::PROBLEM_DETAILS_MANIFEST
                       ).unwrap(),
        },
        {
            "crate":   featurey_wire::Error::PROBLEM_DETAILS_CRATE_NAME,
            "version": featurey_wire::Error::PROBLEM_DETAILS_CRATE_VERSION,
            "errors":  serde_json::from_str::<serde_json::Value>(
                           featurey_wire::Error::PROBLEM_DETAILS_MANIFEST
                       ).unwrap(),
        },
    ]);
    println!("{}", serde_json::to_string_pretty(&entries).unwrap());
}
```

## Features

| Feature | Enables                                                      |
| ------- | ------------------------------------------------------------ |
| `actix` | `Problem<E>` wrapper implementing `actix_web::ResponseError` |

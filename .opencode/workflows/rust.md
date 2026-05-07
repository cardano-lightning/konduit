---
description: Konduit Rust development workflow
---

# Konduit Rust Workflow

Development workflow for the Konduit Rust workspace.

## Overview

Konduit is:

- a Rust workspace with shared Cardano, Lightning, transaction-building, connector, server, CLI, client, and wasm crates
- a project whose principal product purpose is to let Cardano users experience Bitcoin Lightning-style payments through Konduit
- a system with clear role boundaries between Consumer, Adaptor, Admin, and Dev
- a codebase that treats Cardano connectivity as a connector boundary, not a single hard-wired provider forever
- a project that supports both local/self-hosted adaptor deployment and client-side/mobile-facing product surfaces
- a workspace where connector, tx-building, data encoding, server, and client crates should stay composable instead of collapsing into one binary

When working on Konduit, optimize for:

1. payment and channel correctness
2. Cardano connector correctness
3. clear crate boundaries
4. deployment realism for adaptor/server work
5. good diagnostics and failure handling
6. maintainable Rust APIs and data mappings

---

## Rust Skill Usage

Konduit is Rust-first. Repo workflow guidance is necessary but not sufficient.

- Use `rust-router` first for all Konduit Rust implementation, design, debugging, code review, and best-practice work.
- Add targeted Rust skills when the task matches them.
- Use `coding-guidelines` for Rust naming, formatting, lint, and code-style questions.
- Use `unsafe-checker` for any unsafe, FFI, raw-pointer, soundness-sensitive, or wasm-boundary work.
- Use `m01-ownership`, `m02-resource`, and `m03-mutability` for ownership, resource-management, and borrow-conflict problems.
- Use `m04-zero-cost`, `m05-type-driven`, and `m09-domain` for API design, trait boundaries, domain modeling, and compile-time invariants.
- Use `m06-error-handling`, `m12-lifecycle`, and `m13-domain-error` for error-model and lifecycle design.
- Use `m07-concurrency` for async, task orchestration, locks, channels, and `Send`/`Sync` issues.
- Use `m10-performance` for hot-path transaction building, connector mapping, network calls, serialization, or wasm-sensitive performance work.
- Use `m11-ecosystem` when evaluating crates, features, or integration tradeoffs.
- Use `m15-anti-pattern` during review when checking whether a Rust design is fighting the language or drifting from idiomatic patterns.
- Use `domain-fintech` when a task materially affects payment semantics, channel semantics, financial value handling, or protocol-facing transaction behavior.
- Use `cardano-protocol-params` when the task touches Cardano protocol-parameter interpretation or mapping.

If Rust skill guidance and older local docs disagree, prefer the current Konduit design docs plus the Rust skills, then update the docs so later work inherits the corrected guidance.

---

## Project Location

```text
rust/
|- Cargo.toml
|- README.md
|- crates/
|  |- bln-client/                 # BLN-facing client bindings
|  |- bln-sdk/                    # BLN data/types
|  |- cardano-connector/          # Cardano connector trait boundary
|  |- cardano-connector-client/   # HTTP client for connector-server style use
|  |- cardano-connector-direct/   # Direct Blockfrost implementation
|  |- cardano-sdk/                # Cardano primitives and tx-building support
|  |- fx-client/                  # Price feed client surface
|  |- http-client/                # Isomorphic HTTP abstractions
|  |- http-client-native/         # Native HTTP implementation
|  |- http-client-wasm/           # WASM HTTP implementation
|  |- konduit-cli/                # CLI for admin/adaptor/consumer flows
|  |- konduit-client/             # Reusable client logic over connector/adaptor
|  |- konduit-data/               # Shared data encoding/decoding and protocol data
|  |- konduit-server/             # Adaptor-facing HTTP server
|  |- konduit-tx/                 # Konduit transaction-building logic
|  `- konduit-wasm/               # WASM-facing API surface
`- Cargo.lock
```

For the Dolos UTxO RPC effort, the workspace now includes:

- `cardano-connector-utxorpc/`

Key references:

- `rust/README.md`
- `docs/design/00_intro.md`
- `docs/design/11_roles.md`
- `docs/design/20_architecture.md`
- `docs/design/33_cardano_connector.md`
- `docs/adrs/03-cardano-connector-unification.md`
- `docs/adrs/06-dolos-utxorpc-adaptor-backend.md`
- `docs/design/36_dolos_utxorpc_implementation_prd.md`

---

## Quick Commands

Use `workdir=rust` instead of `cd rust && ...` when tooling supports it.

### Fast Feedback

```bash
cargo check --workspace
```

### Build

```bash
cargo build --workspace
```

### Build Release

```bash
cargo build --workspace --release
```

### Test

```bash
cargo test --workspace
```

### Targeted Tests

```bash
cargo test -p cardano-sdk
cargo test -p konduit-client
cargo test -p konduit-server
cargo test -p konduit-cli
cargo test -p cardano-connector-direct
```

For the current UTxO RPC crate:

```bash
cargo test -p cardano-connector-utxorpc
```

Current backend-truth note:

- `cardano-connector-utxorpc` is the production-targeted local-Dolos backend for
  this effort.
- the direct Blockfrost path still exists in parallel, but do not assume full
  runtime parity when docs or code show otherwise.

### Format Check

```bash
cargo fmt --all -- --check
```

### Format

```bash
cargo fmt --all
```

### Lint

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

If `--all-features` is not truthful for the touched crates because features are mutually exclusive or not fully wired, run the narrowest truthful clippy command and record exactly what was and was not verified.

### Docs Build

```bash
cargo doc --workspace --no-deps
```

### WASM Build

```bash
cargo build -p konduit-wasm
```

If `wasm-pack` is installed and the task actually touches the wasm package workflow:

```bash
wasm-pack build crates/konduit-wasm
```

Do not silently claim additional tooling standards that are not actually wired in this workspace. If `cargo-nextest`, `cargo-insta`, `cargo-mutants`, or `criterion` are not installed or not yet integrated here, call out the gap explicitly in verification notes and use the closest truthful fallback only when necessary.

---

## Standard Development Loop

1. Read the relevant design docs, ADRs, and affected crate boundaries before changing architecture.
2. Load `rust-router` first, then add targeted Rust skills when the problem matches them.
3. Keep logic in the appropriate workspace crate instead of pushing everything into `konduit-server`, `konduit-cli`, or a top-level binary by default.
4. Run `cargo fmt --all` after edits.
5. Run targeted `cargo test -p <crate>` first, then `cargo test --workspace` for broader changes.
6. Run `cargo clippy --workspace --all-targets --all-features -- -D warnings` before finishing substantial Rust work, or document the narrow truthful fallback you used.
7. Run `cargo doc --workspace --no-deps` when the change materially affects public crate APIs, trait contracts, or operator-facing interfaces.
8. If the task changes Cardano connector behavior, verify trait semantics, mapping behavior, configuration, and failure behavior explicitly.
9. If the task changes deployment/runtime behavior, verify the implementation against the current Konduit deployment docs instead of inventing a new runtime model.
10. If verification has to fall back from the canonical command set, document exactly what ran and why.

---

## Product Direction

Konduit is not just a generic Rust service collection.

Keep these product assumptions intact:

- the principal product purpose is Lightning-like payment UX for Cardano users
- Consumer, Adaptor, Admin, and Dev are distinct roles and should not collapse into one flat runtime model
- Cardano connectivity belongs behind a connector boundary
- multiple Cardano backends are a design goal; direct Blockfrost is not the only architecture
- the adaptor runtime should support self-hosted deployment with clear trust and exposure boundaries
- `konduit-server` is the public adaptor-facing service in the target deployment model
- local infrastructure boundaries matter for adaptor work:
- `nginx` is public
- `konduit-server`, `dolos`, and `lnd` are localhost-only in the target deployment profile
- Cardano transaction building and data mapping are first-class correctness concerns, not incidental plumbing
- the CLI is a serious runtime surface for admin/adaptor/consumer flows, not throwaway glue
- the wasm/client surfaces should not be broken casually by server- or CLI-centric changes
- diagnostics should make connector, transaction, and deployment failures actionable

Do not let convenience shortcuts in one crate redefine the architecture for the rest of the workspace.

---

## Architecture Guidance

### Layering

Konduit should continue moving toward these broad layers:

- connector boundary layer - Cardano provider abstraction and implementations
- protocol/data layer - shared domain data, serialization, and protocol representations
- transaction-building layer - Cardano and Konduit tx-building logic
- runtime/service layer - server, CLI, client, and deployment-facing runtime code
- platform integration layer - native HTTP, wasm HTTP, and deployment-specific integration surfaces

### Crate Responsibilities

- `cardano-connector` - trait boundary for Cardano access
- `cardano-connector-direct` - direct provider implementation(s), currently Blockfrost
- `cardano-connector-client` - remote connector client over HTTP
- `cardano-sdk` - Cardano types, addresses, transactions, protocol parameters, and builders
- `konduit-data` - shared Konduit protocol data and encoding/decoding
- `konduit-tx` - Konduit-specific tx building
- `konduit-client` - reusable client behavior over connector/adaptor surfaces
- `konduit-cli` - operator and test-facing command-line runtime
- `konduit-server` - adaptor service runtime
- `konduit-wasm` - wasm-facing API and web integration surface
- `http-client*` - transport abstractions and target-specific HTTP implementations

### Design Rules

- Prefer extending existing crate boundaries over inventing new top-level abstractions.
- Prefer small, explicit runtime selection models over large speculative framework layers.
- Keep provider-specific logic inside provider crates or provider-focused modules.
- Keep `konduit-server` startup validation close to the runtime boundary rather than leaking it everywhere.
- Keep connector trait semantics explicit and documented.
- Preserve the difference between direct provider implementations and remote connector-client implementations.
- Add diagnostics and actionable errors when a change affects runtime operations.
- Keep deployment assumptions truthful to the docs.
- Prefer minimal, elegant designs over sprawling abstractions.

---

## Rust Patterns

### Workspace-first development

Use the workspace as the unit of composition.

```rust
fn main() {
    // Compose config, connector selection, runtime services, and serving here.
}
```

Push domain logic down into crates and keep binaries focused on composition.

### Trait-boundary-first design

Prefer keeping provider-neutral logic behind traits and clear data contracts.

```rust
pub trait CardanoConnector {
    fn network(&self) -> cardano_sdk::Network;
    // ...
}
```

If runtime selection is needed and trait-object ergonomics are awkward, prefer a small enum wrapper at the runtime boundary over a speculative large refactor.

### Logging and failure clarity

Prefer actionable structured failures at runtime boundaries.

```rust
use anyhow::{Context, Result};

fn load_backend() -> Result<()> {
    do_the_thing().context("failed to initialize configured Cardano backend")?;
    Ok(())
}
```

Useful diagnostics questions:

- Which Cardano backend is selected?
- Which network is configured, and does it match live provider data?
- Is the connector reachable?
- Can required protocol parameters be derived?
- Can the reference script UTxO be resolved?
- Did UTxO mapping fail, and on which field/category?
- Did submission fail because of provider reachability, bad transaction data, or rejection from the backend?

Observability guardrails:

- Do not leak secrets, credentials, or unnecessary topology details in public-facing errors.
- Keep operator-facing errors actionable.
- Prefer runtime-boundary logs over duplicating noise in deep utility code.
- Keep diagnostics bounded and relevant to the actual failure mode.

---

## Connector And Mapping Expectations

### Connector semantics

For connector work, preserve and test the contract:

- `network()` returns the intended Cardano network
- `health()` reflects meaningful reachability/basic correctness
- `protocol_parameters()` returns what tx building actually needs
- `utxos_at(payment, Some(delegation))` means the exact payment/delegation pair
- `utxos_at(payment, None)` means any UTxO whose address shares the payment credential regardless of delegation
- `submit(tx)` returns success or actionable failure

Documented current-state caveat:

- `cardano-connector-utxorpc` now implements the intended `utxos_at` semantics
  through payment-credential paging plus local delegation filtering.
- the direct Blockfrost path still has narrower `utxos_at(payment, None)`
  behavior and static protocol-parameter fallback; when doing docs or review
  work, record that non-parity explicitly instead of smoothing it over.

### Mapping

When mapping provider responses into `cardano-sdk` or Konduit types:

- preserve the fields current tx-building and channel flows actually consume
- make unsupported or missing fields explicit
- prefer truthful failure over silent partial mapping
- add targeted tests for values, multi-assets, datums, script-related fields, and protocol parameters

### Runtime validation

For `konduit-server` backend startup work, keep the documented readiness rules intact:

- Dolos reachable
- configured network matches live data
- live protocol parameters derivable
- configured reference script UTxO resolvable

If those checks cannot be performed truthfully, document the gap instead of pretending startup validation is complete.

---

## Quality Expectations

Treat testing and verification as core requirements, not cleanup work.

- Add unit tests for domain logic, mapping logic, and boundary behavior.
- Use property-based testing where invariants matter and the crate already supports it or the invariant justifies adding it.
- Protect connector semantics with automated tests.
- Protect server and CLI backend selection/config parsing with automated tests.
- Add the smallest useful integrated smoke coverage when a new runtime path is introduced.
- Build docs when public contracts or interfaces change materially.
- Treat elegant code and measurable correctness as part of the definition of done.
- If the canonical toolchain is not yet installed or wired for the touched crate, record that gap explicitly instead of silently downgrading the standard.

---

## Common Commands By Scope

### Root workspace

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo doc --workspace --no-deps
```

### Single crate

```bash
cargo test -p konduit-server
cargo test -p konduit-cli
cargo test -p konduit-client
cargo test -p cardano-connector-direct
cargo test -p cardano-sdk
```

### New UTxO RPC crate

```bash
cargo check -p cardano-connector-utxorpc
cargo test -p cardano-connector-utxorpc
cargo clippy -p cardano-connector-utxorpc --all-targets -- -D warnings
```

### Fast feedback

```bash
cargo check --workspace
cargo fmt --all
```

### WASM-sensitive work

```bash
cargo build -p konduit-wasm
cargo test -p konduit-client
```

---

## Troubleshooting

### Connector behavior differs across runtimes

Check whether the drift comes from:

- trait contract ambiguity
- runtime wrapper selection
- CLI/server config parsing mismatch
- provider-specific mapping behavior
- startup validation happening in one runtime surface but not another

### New provider integration feels larger than expected

Re-center on the agreed scope:

- add a parallel implementation
- keep Blockfrost working
- touch only the Rust runtime surfaces that currently instantiate/configure Blockfrost
- avoid repo-wide migration fantasies

### Static protocol parameter assumptions leak back in

Re-check the current design docs and the task scope. For the UTxO RPC backend, static per-network presets are not the intended long-term behavior. If the live data is insufficient, document the gap explicitly.

### Errors are too vague to debug

Add or improve runtime-boundary diagnostics before adding more abstraction.

### Change landed in the wrong crate

Move it toward the correct layer:

- provider-specific logic -> provider crate
- shared Cardano types -> `cardano-sdk`
- shared Konduit protocol data -> `konduit-data`
- tx building -> `konduit-tx`
- runtime bootstrap/config -> `konduit-server` or `konduit-cli`

### Canonical Rust tools are unavailable

Call out exactly which tool is missing, what fallback you used, and what remains unverified. Do not claim the Konduit Rust standard was satisfied if you only ran a partial fallback.

---

## Done Checklist

Before considering Konduit Rust work complete:

- code is in the right crate boundary
- `rust-router` and any relevant targeted Rust skills were used for the Rust work
- formatting passes
- relevant tests pass, or any truthful fallback is documented
- clippy passes for substantial Rust changes, or the exact gap is documented
- docs are updated if the task changed public contracts, configuration, or runtime expectations
- diagnostics are good enough to observe the changed behavior
- connector semantics remain explicit and tested where touched
- runtime startup/readiness behavior remains truthful to the docs where touched
- verification notes clearly state any remaining toolchain or environment gaps

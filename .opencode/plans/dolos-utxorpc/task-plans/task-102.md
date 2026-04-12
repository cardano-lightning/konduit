# Task Plan: task-102 - Add autonomous integrated smoke coverage and run workspace verification

- task id: `task-102`
- title: `Add autonomous integrated smoke coverage and run workspace verification`
- planning status: `approved`
- build status: `completed`
- interaction mode: `autonomous`
- review-log paths:
- `.opencode/plans/dolos-utxorpc/task-plans/task-102-plan-review.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-102-impl-review.md`

## Why This Task Was Chosen Now

`task-102` is the next lowest-ID unblocked pending task on the remaining critical path after `task-101`. The connector core and runtime-surface unit coverage are already in place, so the next truthful step is autonomous integrated smoke coverage plus broader verification across the touched Rust runtime surfaces.

## Interaction Mode

- mode: `autonomous`
- reason: this task can be completed with agent-executable Rust test additions and local verification commands only; it does not require a live Dolos instance or operator-owned infrastructure because the required smoke coverage is explicitly autonomous and non-live.
- required user inputs: none
- required manual test steps: none
- evidence needed back from the user: none
- can implementation proceed before user interaction: yes

## Scope

This task is limited to the smallest useful autonomous integrated smoke coverage for the UTxO RPC runtime path and the truthful verification needed for the touched Rust surfaces.

In scope:

- add integrated smoke-style automated coverage that exercises real runtime composition for the UTxO RPC path without depending on live Dolos
- keep the smoke coverage inside the scoped Rust runtime surfaces: `cardano-connector-utxorpc`, `konduit-server`, and `konduit-cli`
- fix repo-local Rust issues inside those same runtime surfaces when they block truthful verification of the integrated path
- run the narrowest truthful build, test, lint, and docs commands needed to verify the touched surfaces after the smoke coverage lands
- record any remaining verification gaps precisely if a canonical workspace command is still blocked by unrelated repo issues after reasonable task-local fixes

## Non-Goals

- no live Dolos validation, operator evidence capture, or networked submission checks; that belongs to `task-103`
- no documentation-finalization sweep beyond task-local updates required to keep tracking and research consistent; that belongs to `task-104`
- no expansion into unrelated crates such as `cardano-connector-server` or other repository subprojects
- no speculative harness framework or broad fake-provider architecture if a small in-test seam is sufficient

## Relevant Dependencies

- direct task dependencies: `task-100`, `task-101`
- upstream code dependencies already completed: `task-002`, `task-003`
- downstream task unlocked by this task: `task-103`
- durable research output expected from this task: `.opencode/plans/dolos-utxorpc/research/task-102.md`

## Research Consulted

- `.opencode/plans/dolos-utxorpc/research/task-002.md`
- `.opencode/plans/dolos-utxorpc/research/task-003.md`
- `.opencode/plans/dolos-utxorpc/research/task-100.md`
- `.opencode/plans/dolos-utxorpc/research/task-101.md`

Key carry-forward findings from prior research:

- runtime backend selection remains a small enum wrapper because `CardanoConnector` is not object-safe
- truthful runtime readiness is split across `konduit-server/src/cardano/args.rs` and `konduit-server/src/admin/service.rs`
- the current `konduit-server` binary verification gap is a `Send` failure at `src/main.rs:63` when `tokio::spawn` captures the current admin-sync future
- the connector core already has focused mapping and parameter tests, so this task should add integrated runtime smoke coverage instead of duplicating lower-level tests

## Docs, Crate Files, External References, And Skills Consulted

- `.opencode/plans/dolos-utxorpc/dolos-utxorpc-tasks.json`
- `.opencode/plans/dolos-utxorpc/research/task-002.md`
- `.opencode/plans/dolos-utxorpc/research/task-003.md`
- `.opencode/plans/dolos-utxorpc/research/task-100.md`
- `.opencode/plans/dolos-utxorpc/research/task-101.md`
- `docs/adrs/06-dolos-utxorpc-adaptor-backend.md`
- `docs/design/33_cardano_connector.md`
- `docs/design/35_adaptor_deployment.md`
- `docs/design/36_dolos_utxorpc_implementation_prd.md`
- `docs/design/37_adaptor_deployment_prd.md`
- `.opencode/workflows/rust.md`
- `rust/README.md`
- `rust/Cargo.toml`
- `rust/crates/cardano-connector/src/connector.rs`
- `rust/crates/cardano-connector-utxorpc/src/lib.rs`
- `rust/crates/konduit-server/src/cardano.rs`
- `rust/crates/konduit-server/src/cardano/args.rs`
- `rust/crates/konduit-server/src/admin/service.rs`
- `rust/crates/konduit-server/src/main.rs`
- `rust/crates/konduit-cli/src/connector.rs`
- `rust/crates/konduit-cli/src/config/connector.rs`
- `rust/crates/konduit-cli/src/env/connector.rs`
- `rust/crates/konduit-cli/src/tip.rs`
- `rust/crates/konduit-cli/src/cmd/admin/tx.rs`
- `rust/crates/konduit-cli/src/cmd/adaptor/tx.rs`
- external reference: `https://github.com/utxorpc/rust-sdk`
- external reference: `https://github.com/txpipe/dolos`
- Rust skills consulted: `rust-router`, `coding-guidelines`, `m06-error-handling`, `m07-concurrency`, `m11-ecosystem`, `m15-anti-pattern`, `domain-fintech`

## Files Expected To Change

- `rust/crates/konduit-server/src/main.rs`
- `rust/crates/konduit-server/src/admin.rs`
- `rust/crates/konduit-server/src/admin/service.rs`
- `rust/crates/konduit-cli/src/tip.rs`
- `rust/crates/konduit-cli/src/cmd/admin/tx.rs`
- `rust/crates/konduit-cli/src/cmd/adaptor/tx.rs`

Possible low-risk support changes if the smallest smoke seam needs them:

- `rust/crates/konduit-server/src/cardano.rs`
- `rust/crates/konduit-cli/src/connector.rs`
- `rust/crates/cardano-connector-utxorpc/src/lib.rs`

## Implementation Approach

1. Add one minimal integrated smoke path at the CLI runtime boundary.

Use a narrow in-test fake connector in `konduit-cli` to exercise a real tx-oriented runtime path in `konduit-cli/src/cmd/admin/tx.rs` or `konduit-cli/src/cmd/adaptor/tx.rs`. The accepted smoke seam must prove connector network usage, UTxO lookup, protocol-parameter retrieval, transaction building, and submit wiring together. A tip-only path is not sufficient for this task because it would miss the distinctive integrated behavior added by the UTxO RPC backend.

2. Add one minimal integrated smoke path at the server runtime boundary.

Use the existing `admin::Service::new(...)` startup path and, if needed, one subsequent `sync()`-related path to prove the server-side UTxO RPC runtime composition still works as a whole with a fake connector or narrow mock dependencies. Reuse local in-test fakes rather than introducing production abstractions.

3. Treat the existing `konduit-server` `Send` failure as part of this task if it blocks truthful verification.

Because this task explicitly requires relevant runtime targets to build and test cleanly, the current `tokio::spawn` versus `#[async_trait(?Send)]` mismatch in `konduit-server` is in scope if it still prevents truthful crate-level verification after the smoke tests land. The acceptable repair scope is the runtime boundary around `konduit-server/src/main.rs` and `konduit-server/src/admin.rs` sendness. Do not refactor `CardanoConnector`, add broader provider abstractions, or move logic across crate boundaries just to force `Send`.

4. Keep the smoke coverage autonomous and repeatable.

Do not add tests that need a real Dolos instance, network access, or operator-managed secrets. The smoke coverage should be fast, deterministic, and runnable in CI-style local execution.

5. Run broader verification for the touched surfaces and then the workspace.

After implementation, run formatting, targeted crate builds, targeted tests, targeted clippy, and truthful docs generation for the touched surfaces. If the `konduit-server` binary build becomes clean after the task-local fix, follow those targeted checks with at least `cargo check --workspace` and `cargo test --workspace` as the broader truthful workspace pass required by this task's title and the Rust workflow.

## Acceptance Criteria

- there is automated smoke coverage for the integrated UTxO RPC runtime path
- the relevant Rust workspace targets build and test cleanly for the touched surfaces
- the integrated smoke coverage remains autonomous and suitable for repeat execution

## Verification Plan

Planned implementation-time verification:

- `cargo fmt --all`
- `cargo fmt --all -- --check`
- `cargo check -p cardano-connector-utxorpc`
- `cargo test -p cardano-connector-utxorpc`
- `cargo clippy -p cardano-connector-utxorpc --all-targets -- -D warnings`
- `cargo check -p konduit-cli`
- `cargo test -p konduit-cli`
- `cargo clippy -p konduit-cli --all-targets -- -D warnings`
- `cargo check -p konduit-server`
- `cargo test -p konduit-server`
- `cargo clippy -p konduit-server --all-targets -- -D warnings`
- `cargo doc -p cardano-connector-utxorpc --no-deps`
- `cargo doc -p konduit-cli --no-deps`
- `cargo doc -p konduit-server --no-deps`
- `cargo check --workspace`
- `cargo test --workspace`

Truthful fallback only if a remaining failure is proven unrelated to the touched surfaces and not reasonably fixable within this task's scoped runtime crates.

## Risks / Open Questions

- the existing `SyncApi` `?Send` design may conflict with `tokio::spawn` in `konduit-server/src/main.rs`; if so, review should confirm whether making that future `Send` is the smallest correct fix or whether the spawn pattern itself should change
- integrated smoke coverage can bloat quickly if it duplicates unit-level fakes across CLI and server; the implementation should prefer small local helpers and reuse existing test fixtures where possible
- if a chosen smoke path only proves one runtime branch, review should check that it still exercises the distinctive UTxO RPC composition requirements rather than generic connector behavior already covered elsewhere

## Required Docs / Tracking / Research Updates

- append planning critique results to `.opencode/plans/dolos-utxorpc/task-plans/task-102-plan-review.md`
- during implementation, create and maintain `.opencode/plans/dolos-utxorpc/task-plans/task-102-impl-review.md`
- during or after implementation, write durable findings to `.opencode/plans/dolos-utxorpc/research/task-102.md`, including the accepted smoke-test seam, any `Send` or runtime-boundary fixes needed for truthful verification, and any remaining verification limits
- update `.opencode/plans/dolos-utxorpc/dolos-utxorpc-tasks.json` when the task outcome is final

## Plan Summary

Implement `task-102` by adding the smallest truthful autonomous smoke coverage for the integrated UTxO RPC runtime path in `konduit-cli` and `konduit-server`, fixing the current `konduit-server` runtime-boundary `Send` issue if it blocks crate-level verification, and then running targeted build, test, lint, and docs commands for the touched Rust surfaces so the integrated path is validated beyond isolated unit tests.

## Implementation Progress

- implementation status: `iteration_1_complete`
- scope completed: added the planned tx-oriented CLI smoke test, added the planned successful server-startup smoke test, applied the bounded runtime-local fix for the `konduit-server` background-sync compile blocker, and completed targeted plus workspace verification
- durable research recorded at `.opencode/plans/dolos-utxorpc/research/task-102.md`

## Current Outcome

- current implementation outcome: `approved`
- implementation review log: `.opencode/plans/dolos-utxorpc/task-plans/task-102-impl-review.md`
- no user handoff is required for this task because all implementation and verification steps were agent-executable

## Final Outcome

- final accepted outcome: `task-102` is complete and approved
- `konduit-cli` now has an autonomous tx-oriented smoke test that proves integrated runtime composition across address derivation, UTxO lookup, protocol-parameter retrieval, tx building, signing, and submission
- `konduit-server` now has a successful startup smoke test for the documented readiness composition, and the previously blocked admin background-sync runtime path now compiles cleanly in the current repository state
- targeted touched-surface verification and broader workspace verification both pass truthfully after the bounded runtime-local `Send` fix
- final review sources:
  - plan review: `.opencode/plans/dolos-utxorpc/task-plans/task-102-plan-review.md`
  - implementation review: `.opencode/plans/dolos-utxorpc/task-plans/task-102-impl-review.md`

## Verification Actually Run

- `cargo fmt --all`
- `cargo fmt --all -- --check`
- `cargo check -p cardano-connector-utxorpc`
- `cargo test -p cardano-connector-utxorpc`
- `cargo clippy -p cardano-connector-utxorpc --all-targets -- -D warnings`
- `cargo doc -p cardano-connector-utxorpc --no-deps`
- `cargo check -p konduit-cli`
- `cargo test -p konduit-cli`
- `cargo clippy -p konduit-cli --all-targets -- -D warnings`
- `cargo doc -p konduit-cli --no-deps`
- `cargo check -p konduit-server`
- `cargo test -p konduit-server`
- `cargo clippy -p konduit-server --all-targets -- -D warnings`
- `cargo doc -p konduit-server --no-deps`
- `cargo check --workspace`
- `cargo test --workspace`

## Final Build Status

- status: `completed`
- meaning for this task: the integrated autonomous smoke coverage is in place, the touched runtime surfaces verify cleanly, the broader workspace build and test pass, the review loop is approved, and the required research plus tracking artifacts are synchronized

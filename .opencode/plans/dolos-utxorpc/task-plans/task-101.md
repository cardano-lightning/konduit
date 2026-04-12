# Task Plan: task-101 - Add server and CLI tests for backend selection and readiness checks

- task id: `task-101`
- title: `Add server and CLI tests for backend selection and readiness checks`
- planning status: `approved`
- build status: `completed`
- interaction mode: `autonomous`
- review-log paths:
- `.opencode/plans/dolos-utxorpc/task-plans/task-101-plan-review.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-101-impl-review.md`

## Why This Task Was Chosen Now

`task-101` is the next lowest-ID unblocked pending task on the remaining critical path after `task-100`. It must happen now because the UTxO RPC runtime path is already wired into `konduit-server` and `konduit-cli`, but the broader regression coverage promised for those runtime surfaces is still missing.

## Interaction Mode

- mode: `autonomous`
- reason: this task is limited to agent-executable Rust test additions and small helper refactors in already-touched runtime crates; no live Dolos instance, operator-owned infrastructure, or user-observed behavior is required to reach a truthful completion point.
- required user inputs: none
- required manual test steps: none
- evidence needed back from the user: none
- can implementation proceed before user interaction: yes

## Scope

This task is limited to automated regression coverage for the server and CLI runtime surfaces that gained explicit `blockfrost` or `utxorpc` backend selection in `task-003`.

In scope:

- add server tests for backend selection and per-backend config validation in `konduit-server`
- add focused server startup-readiness tests in `konduit-server/src/admin/service.rs` for the non-live startup blockers enforced before traffic can be served
- add CLI tests for backend selection, config parsing, env fill behavior, and runnable-config validation in `konduit-cli`
- add at least one representative higher-level CLI env consumer test where `fill()` and `connector.network_id()` drive default wallet or host-address derivation
- add automated coverage for UTxO RPC readiness-related failures that can be exercised truthfully without a live Dolos instance
- add automated coverage that protects the existing Blockfrost path from regression where the new backend-selection model could drift
- make only the smallest runtime-local refactors needed to expose truthful test seams for validation logic

## Non-Goals

- no live Dolos validation or operator evidence collection; that belongs to `task-103`
- no broad integrated smoke harness beyond the smallest runtime-local coverage needed here; that belongs to `task-102`
- no connector-core mapping or protocol-parameter unit work; that belongs to `task-100`
- no repo-wide backend migration or changes outside `konduit-server` and `konduit-cli`
- no redesign of the `CardanoConnector` trait or runtime architecture unless an existing validation path is literally untestable without a tiny local extraction

## Relevant Dependencies

- direct task dependency: `task-003`
- practical coverage dependency: `task-100`, because connector-core behavior already has focused tests and should not be re-tested here at the wrong layer
- downstream task unlocked by this task:
- `task-102`
- durable research output expected from this task: `.opencode/plans/dolos-utxorpc/research/task-101.md`

## Research Consulted

- `.opencode/plans/dolos-utxorpc/research/task-003.md`
- `.opencode/plans/dolos-utxorpc/research/task-100.md`

Key carry-forward findings from prior research:

- runtime backend selection stays as small enum wrappers because the shared `CardanoConnector` trait is not object-safe
- server readiness is split truthfully between `konduit-server/src/cardano/args.rs` for reachability and live-network validation, and `konduit-server/src/admin/service.rs` for live protocol-parameter derivation plus reference-script resolution before traffic can be served
- CLI truthfulness depends on keeping explicit UTxO RPC network requirements separate from Blockfrost inference and ensuring stale Blockfrost env state does not rewrite UTxO RPC config
- connector-core semantics and parameter mapping already have task-local coverage, so this task should focus on the runtime-layer selection and validation boundaries rather than duplicating lower-level tests

## Docs, Crate Files, External References, And Skills Consulted

- `.opencode/plans/dolos-utxorpc/dolos-utxorpc-tasks.json`
- `.opencode/plans/dolos-utxorpc/research/task-003.md`
- `.opencode/plans/dolos-utxorpc/research/task-100.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-003.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-100.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-100-plan-review.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-100-impl-review.md`
- `docs/adrs/06-dolos-utxorpc-adaptor-backend.md`
- `docs/design/33_cardano_connector.md`
- `docs/design/35_adaptor_deployment.md`
- `docs/design/36_dolos_utxorpc_implementation_prd.md`
- `docs/design/37_adaptor_deployment_prd.md`
- `.opencode/workflows/rust.md`
- `rust/README.md`
- `rust/Cargo.toml`
- `rust/crates/cardano-connector/src/connector.rs`
- `rust/crates/cardano-connector-direct/src/blockfrost.rs`
- `rust/crates/konduit-server/src/cardano.rs`
- `rust/crates/konduit-server/src/cardano/args.rs`
- `rust/crates/konduit-server/src/admin/service.rs`
- `rust/crates/konduit-cli/src/connector.rs`
- `rust/crates/konduit-cli/src/config/connector.rs`
- `rust/crates/konduit-cli/src/env/connector.rs`
- `rust/crates/konduit-cli/src/env/admin.rs`
- `rust/crates/konduit-cli/src/env/adaptor.rs`
- `rust/crates/konduit-cli/src/env/consumer.rs`
- external reference: `https://github.com/utxorpc/rust-sdk`
- external reference: `https://github.com/txpipe/dolos`
- Rust skills consulted: `rust-router`, `coding-guidelines`, `m06-error-handling`, `m07-concurrency`, `m15-anti-pattern`, `domain-fintech`, `domain-cli`

## Files Expected To Change

- `rust/crates/konduit-server/src/cardano/args.rs`
- `rust/crates/konduit-server/src/admin/service.rs`
- `rust/crates/konduit-cli/src/connector.rs`
- `rust/crates/konduit-cli/src/config/connector.rs`
- `rust/crates/konduit-cli/src/env/connector.rs`
- `rust/crates/konduit-cli/src/env/admin.rs`

Possible low-risk support changes if test seams require them:

- `rust/crates/konduit-server/src/cardano.rs`
- `rust/crates/konduit-cli/src/env/adaptor.rs`
- `rust/crates/konduit-cli/src/env/consumer.rs`

Likely no-change surfaces for this task:

- `rust/crates/cardano-connector-utxorpc/*`
- `rust/crates/cardano-connector-direct/*`

## Implementation Approach

1. Expand `konduit-server` tests around the actual backend-selection boundary.

Add coverage in `konduit-server/src/cardano/args.rs` for both backend kinds, including success-path config construction for Blockfrost and UTxO RPC, missing per-backend fields, and the existing shared network-mismatch rejection helper. If the current tests only prove individual error strings, add minimal assertions that the built config preserves the selected backend and explicit network or endpoint values before any live connection is attempted.

2. Add focused unit coverage for the server startup blockers that do not require live Dolos.

Add tests in `konduit-server/src/admin/service.rs` around `Service::new(...)` using the smallest in-test fake `CardanoConnector` needed to drive the startup path. At minimum, cover failure when `protocol_parameters()` returns an error and failure when the configured host address lookup does not resolve the reference script UTxO. Keep this seam local to the test module rather than introducing a new shared abstraction, and treat these tests as protection for the documented startup blockers that remain on the mandatory boot path before `server.run()`.

3. Lock down CLI config and env truthfulness across both backends.

Add tests in `konduit-cli/src/config/connector.rs` and `konduit-cli/src/env/connector.rs` covering explicit backend selection, `network()` and `network_id()` behavior for both variants, Blockfrost inference remaining scoped to Blockfrost only, and UTxO RPC continuing to require explicit network plus URI for runnable config while still allowing display or address-derivation fill behavior where intended.

4. Add one representative higher-level CLI env consumer regression test.

Add a focused test in `konduit-cli/src/env/admin.rs` or another single representative `env/*` consumer proving that `fill()` plus `connector.network_id()` still produce the correct derived defaults for both backends at the real caller boundary, and explicitly that UTxO RPC does not inherit Blockfrost-only fallback behavior. Keep this minimal and rely on the shared `ConnectorEnv` path already exercised in lower-level tests rather than duplicating all env modules.

5. Add CLI runnable-config validation coverage at the shared connector construction boundary.

Extend `konduit-cli/src/connector.rs` tests to cover successful Blockfrost connector construction, failure on missing UTxO RPC URI, failure on Blockfrost network mismatch, and any additional pre-runtime validation that is currently reachable without live Dolos. If the code lacks a narrow pure helper for asserting backend-specific validation inputs before runtime creation, extract only that helper rather than introducing a fake connector abstraction.

6. Protect runtime-surface readiness semantics without inventing fake live integration.

Because live Dolos reachability and network-match checks remain in `cardano/args.rs`, and live protocol-parameter derivation plus reference-script resolution remain in `admin::Service::new`, this task should cover the non-live regression-prone failure paths at both boundaries without pretending to replace integrated or manual validation. The remaining live behavior claims still belong to later smoke and operator-validation tasks.

7. Keep Blockfrost parity explicit.

Every new test group should include at least one Blockfrost-path assertion so the new UTxO RPC work does not silently break the existing backend. Prefer small focused tests over broad duplicated suites.

## Acceptance Criteria

- automated tests cover backend selection for both server and CLI
- automated tests cover invalid UTxO RPC configuration and the readiness-related failures that are truthfully testable without live Dolos, including the non-live server startup blockers in `admin::Service::new` and representative CLI env-consumer default-derivation behavior
- automated tests protect the existing Blockfrost path from regression

## Verification Plan

Planned implementation-time verification:

- `cargo fmt --all -- --check`
- `cargo check -p konduit-server`
- `cargo check -p konduit-cli`
- `cargo test -p konduit-server`
- `cargo test -p konduit-cli`
- `cargo clippy -p konduit-server --all-targets -- -D warnings`
- `cargo clippy -p konduit-cli --all-targets -- -D warnings`

Truthful optional broader verification if the change surface warrants it:

- `cargo doc -p konduit-server --no-deps`
- `cargo doc -p konduit-cli --no-deps`

Verification explicitly out of scope for this task:

- no claim that live Dolos connectivity was exercised
- no claim that startup-time protocol-parameter derivation or reference-script resolution was validated against a live service
- no claim that end-to-end submission was exercised outside future tasks

## Risks / Open Questions

- `admin::Service::new` currently enforces protocol-parameter and reference-script startup blockers on the actual boot path, so the in-test fake connector for those cases must stay narrow and should not evolve into a broader production abstraction
- CLI connector construction currently creates a Tokio runtime in-process for UTxO RPC validation; the plan should avoid adding broad test scaffolding around live async work and instead focus on pre-runtime validation seams that can be checked truthfully
- env fill and runnable config remain intentionally different concepts in the CLI; the representative higher-level env test must protect this distinction without duplicating every env module
- Blockfrost parity can regress quietly if new assertions only target UTxO RPC, so review should stress-test whether each touched runtime surface still has at least one Blockfrost-path regression check

## Required Docs / Tracking / Research Updates

- append planning critique results to `.opencode/plans/dolos-utxorpc/task-plans/task-101-plan-review.md`
- during implementation, create and maintain `.opencode/plans/dolos-utxorpc/task-plans/task-101-impl-review.md`
- during or after implementation, write durable findings to `.opencode/plans/dolos-utxorpc/research/task-101.md`, including accepted runtime-layer test seams, any validation helpers extracted, and any remaining live-only readiness gaps
- update `.opencode/plans/dolos-utxorpc/dolos-utxorpc-tasks.json` when the task outcome is final

## Plan Summary

Implement `task-101` by adding focused regression tests in `konduit-server` and `konduit-cli` for explicit backend selection, per-backend config truthfulness, server startup blockers that are testable without live Dolos, representative higher-level CLI env-consumer behavior, shared pre-runtime validation failures, and Blockfrost parity, while keeping any new test seams narrow, runtime-local, and honest about the live Dolos checks that still belong to later tasks.

## Implementation Progress

- implementation status: `iteration_1_complete`
- scope completed: added the planned runtime-surface regression tests in `konduit-server` and `konduit-cli`, including server config-boundary coverage, `admin::Service::new(...)` startup-blocker tests, CLI config and env truthfulness tests, one representative higher-level CLI env-consumer test, and a narrow Blockfrost validation helper in the CLI connector for direct runnable-config test coverage
- durable research recorded at `.opencode/plans/dolos-utxorpc/research/task-101.md`

## Current Outcome

- current implementation outcome: `approved`
- implementation review log: `.opencode/plans/dolos-utxorpc/task-plans/task-101-impl-review.md`
- no user handoff is required for this task because all implementation and verification steps were agent-executable and no live Dolos checkpoint was claimed

## Final Outcome

- final accepted outcome: `task-101` is complete and approved
- `konduit-server` now has runtime-layer regression coverage for backend-selection config validation plus the non-live startup blockers enforced by `admin::Service::new(...)`
- `konduit-cli` now has broader regression coverage for explicit backend display and selection, env fill truthfulness, representative higher-level default-address derivation behavior, and Blockfrost runnable-config validation without widening runtime architecture
- final review sources:
  - plan review: `.opencode/plans/dolos-utxorpc/task-plans/task-101-plan-review.md`
  - implementation review: `.opencode/plans/dolos-utxorpc/task-plans/task-101-impl-review.md`

## Verification Actually Run

- `cargo fmt --all`
- `cargo fmt --all -- --check`
- `cargo check -p konduit-cli`
- `cargo test -p konduit-cli`
- `cargo clippy -p konduit-cli --all-targets -- -D warnings`
- `cargo test -p konduit-server --lib`
- `cargo clippy -p konduit-server --lib -- -D warnings`

Verification gaps recorded truthfully for this task:

- `cargo check -p konduit-server --lib --tests` currently fails because Cargo still builds the `konduit-server` binary target, which hits an existing `Send` error at `rust/crates/konduit-server/src/main.rs:63` unrelated to task-101's test-only changes
- `cargo clippy -p konduit-server --lib --tests -- -D warnings` hits the same existing binary-target `Send` failure, so the truthful fallback for the touched server surfaces was `cargo clippy -p konduit-server --lib -- -D warnings`

## Final Build Status

- status: `completed`
- meaning for this task: the touched CLI crate passes full targeted checks, the touched `konduit-server` library or test surfaces pass targeted tests and library-only clippy, the review loop is approved, and the remaining binary-target `Send` failure is documented as a pre-existing repo-level blocker outside this task's test-only scope

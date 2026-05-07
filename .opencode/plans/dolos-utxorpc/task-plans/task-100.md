# Task Plan: task-100 - Add connector unit tests for mapping, parameters, semantics, and submit errors

- task id: `task-100`
- title: `Add connector unit tests for mapping, parameters, semantics, and submit errors`
- planning status: `approved`
- build status: `completed`
- interaction mode: `autonomous`
- review-log paths:
- `.opencode/plans/dolos-utxorpc/task-plans/task-100-plan-review.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-100-impl-review.md`

## Why This Task Was Chosen Now

`task-100` is the next unblocked pending task on the remaining critical path after `task-003`. It must happen now because phase 2 starts with strengthening automated confidence in the new connector core before broader runtime-surface regression coverage in `task-101` and integrated smoke validation in `task-102`.

## Interaction Mode

- mode: `autonomous`
- reason: this task is limited to agent-executable Rust test and small helper changes inside `cardano-connector-utxorpc`; no live Dolos instance, operator-owned infrastructure, or subjective user validation is required to reach a truthful completion point.
- required user inputs: none
- required manual test steps: none
- evidence needed back from the user: none
- can implementation proceed before user interaction: yes

## Scope

This task is limited to adding focused automated coverage for the UTxO RPC connector crate and any small connector-crate-local refactors needed to make that coverage truthful and maintainable.

In scope:

- add connector-crate unit tests covering successful and failing UTxO mapping behavior
- add automated tests for protocol-parameter derivation success and error paths
- add explicit tests for `utxos_at(payment, None)` semantics at the connector-core level
- add tests covering submit-path error translation behavior
- make only the smallest connector-crate-local code changes needed to expose testable seams without widening runtime scope

## Non-Goals

- no new server or CLI backend-selection tests in this task; those belong to `task-101`
- no live Dolos validation or operator handoff; that belongs to `task-103`
- no repo-wide integration harness or broad workspace smoke path; that belongs to `task-102`
- no connector-trait redesign unless existing connector-core code proves fundamentally untestable without it
- no behavior changes to Blockfrost or unrelated crates

## Relevant Dependencies

- direct task dependency: `task-002`
- practical implementation context from `task-003`, because shared UTxO RPC network and validation helpers now exist in the connector crate
- downstream tasks unlocked by this task:
- `task-102`
- durable research output expected from this task: `.opencode/plans/dolos-utxorpc/research/task-100.md`

## Research Consulted

- `.opencode/plans/dolos-utxorpc/research/task-002.md`
- `.opencode/plans/dolos-utxorpc/research/task-003.md`

Key carry-forward findings from prior research:

- connector-core mapping already has targeted tests for a few high-risk helper paths, but broader success and failure coverage remains explicitly reserved for `task-100`
- parsed fallback mapping must reject unsupported native reference scripts and oversized integer quantities rather than silently truncating or partially mapping data
- protocol-parameter derivation currently depends on `read_params` plus `read_era_summary`, with `start_time` reconstructed from the Shelley boundary and the ledger-fixed Byron slot length because parsed genesis config is not exposed through the reviewed helper path
- runtime selection is already handled outside this crate, so this task can stay tightly focused on connector-core correctness

## Docs, Crate Files, External References, And Skills Consulted

- `.opencode/plans/dolos-utxorpc/dolos-utxorpc-tasks.json`
- `.opencode/plans/dolos-utxorpc/research/task-002.md`
- `.opencode/plans/dolos-utxorpc/research/task-003.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-002.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-003.md`
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
- `rust/crates/cardano-connector-utxorpc/Cargo.toml`
- `rust/crates/cardano-connector-utxorpc/src/lib.rs`
- `rust/crates/cardano-connector-utxorpc/src/mapping.rs`
- `rust/crates/cardano-connector-utxorpc/src/params.rs`
- `rust/crates/cardano-connector-utxorpc/src/config.rs`
- external reference: `https://github.com/utxorpc/rust-sdk`
- external reference: `https://github.com/txpipe/dolos`
- Rust skills consulted: `rust-router`, `coding-guidelines`, `m06-error-handling`, `m11-ecosystem`, `m15-anti-pattern`, `domain-fintech`, `cardano-protocol-params`

## Files Expected To Change

- `rust/crates/cardano-connector-utxorpc/src/lib.rs`
- `rust/crates/cardano-connector-utxorpc/src/mapping.rs`
- `rust/crates/cardano-connector-utxorpc/src/params.rs`

Possible low-risk support changes if test seams require them:

- `rust/crates/cardano-connector-utxorpc/Cargo.toml`

Likely no-change surfaces for this task:

- all server and CLI crates
- `rust/crates/cardano-connector/src/connector.rs`
- `rust/crates/cardano-connector-utxorpc/src/config.rs`

## Implementation Approach

1. Expand mapping tests around both success and rejection behavior.

Add coverage for native-bytes and parsed-output mapping where truthful fixtures are practical, including datum handling, multi-asset mapping, unsupported script cases, and malformed or incomplete payload errors. Require an explicit precedence test proving native transaction-output bytes win over parsed fallback when both are present, plus a separate fallback test proving parsed mapping is used only when native bytes are absent. Prefer exercising existing pure helpers instead of adding broad mocking layers.

2. Expand protocol-parameter tests around the actual derivation helpers.

Add tests for successful `ProtocolParameters` derivation from UTxO RPC-style inputs and for key failure paths such as missing Shelley era data, missing execution prices, missing cost model, negative or overflowed numeric values, invalid rational denominators, and computed negative chain start. The success cases must explicitly assert the current Shelley-boundary-based derivation of `start_time` and `first_shelley_slot` so the connector cannot silently drift back toward static presets.

3. Make payment-only semantics explicit in tests.

Preserve the requirement that `utxos_at(payment, None)` means any address sharing the payment credential regardless of delegation. Add a truthful unit-test seam for the full paginated accumulation path currently implemented in `load_utxos()`, so multi-page results, post-page local payment filtering, and mapping-error behavior are exercised together rather than inferred only from separate predicate and matcher helper tests. Keep that seam narrow and local to this crate instead of adding a broad fake-client abstraction.

4. Add submit-path error translation coverage without inventing an integration harness.

If the current `submit()` implementation cannot be tested truthfully without live transport, extract only the narrow error-wrapping logic into a small helper that can be unit-tested with representative upstream error values. Avoid speculative test abstractions around the whole gRPC client if a small seam is enough.

5. Lock down remaining connector-core network assumptions and keep code changes minimal.

Retain connector-core coverage for `network_from_genesis()` edge cases, including unsupported `network_magic`, inconsistent `network_id` versus magic, and the accepted empty `network_id` case already encoded in the crate. If a helper must become `pub(crate)` or a small pure function must be extracted for testability, do that instead of adding large fake client traits or broad dependency inversion just for tests.

## Acceptance Criteria

- automated tests cover successful and failing Cardano data mapping cases
- automated tests cover live protocol parameter derivation and error paths
- automated tests explicitly cover `utxos_at(payment, None)` semantics
- automated tests cover submit-path error translation

## Verification Plan

Planned implementation-time verification:

- `cargo fmt --all -- --check`
- `cargo check -p cardano-connector-utxorpc`
- `cargo test -p cardano-connector-utxorpc`
- `cargo clippy -p cardano-connector-utxorpc --all-targets -- -D warnings`

Truthful optional broader verification if the change surface warrants it:

- `cargo doc -p cardano-connector-utxorpc --no-deps`

Verification explicitly out of scope for this task:

- no claim that live Dolos connectivity was exercised
- no claim that server or CLI runtime surfaces were revalidated beyond connector-crate compilation impacts

## Risks / Open Questions

- some connector behavior currently sits behind concrete `utxorpc` clients, so critique should stress-test whether submit-path coverage needs a small helper extraction to stay unit-testable without over-abstracting the crate
- parsed-output success fixtures may be verbose because UTxO RPC protobuf structures are nested; critique should stress-test whether the proposed success cases stay minimal and maintainable
- protocol-parameter success tests must not accidentally bake in static per-network presets; they should assert values derived from explicit synthetic UTxO RPC payloads only, including the currently documented Shelley-boundary timing reconstruction
- if native-byte fixture construction becomes disproportionately heavy, the implementation should prefer a smaller pure-helper test seam over introducing a sprawling fixture factory
- the paginated `utxos_at` test seam must stay minimal; the implementation should avoid inventing a new client abstraction layer if a small page-processing helper or similarly local seam is enough

## Required Docs / Tracking / Research Updates

- append planning critique results to `.opencode/plans/dolos-utxorpc/task-plans/task-100-plan-review.md`
- during implementation, create and maintain `.opencode/plans/dolos-utxorpc/task-plans/task-100-impl-review.md`
- during or after implementation, write durable findings to `.opencode/plans/dolos-utxorpc/research/task-100.md`, including test seam decisions, fixture constraints, new edge cases covered, and any connector-core limitations still deferred
- update `.opencode/plans/dolos-utxorpc/dolos-utxorpc-tasks.json` when the task outcome is final

## Plan Summary

Implement `task-100` entirely inside `cardano-connector-utxorpc`: expand unit coverage for mapping success and failure cases, protocol-parameter derivation and error paths, payment-only `utxos_at` semantics, and submit-path error translation, while keeping any testability refactors small, local, and faithful to the existing connector design.

## Implementation Progress

- implementation status: `iteration_2_complete`
- scope completed: expanded connector-crate unit coverage across `lib.rs`, `mapping.rs`, and `params.rs`, added a narrow stateful page-fetch seam to exercise the real paginated `utxos_at` accumulation loop, and recorded durable findings in `.opencode/plans/dolos-utxorpc/research/task-100.md`
- review-driven follow-up completed: iteration 2 replaced helper-only pagination coverage with full loop-level coverage and added the missing durable research note before the final code-review pass

## Current Outcome

- current implementation outcome: `approved`
- implementation review log: `.opencode/plans/dolos-utxorpc/task-plans/task-100-impl-review.md`
- no user handoff is required for this task because all implementation and verification steps were agent-executable and no live Dolos checkpoint was claimed

## Final Outcome

- final accepted outcome: `task-100` is complete and approved
- `cardano-connector-utxorpc` now carries focused automated coverage for mapping success and rejection cases, native-bytes precedence, Shelley-boundary-based protocol-parameter derivation and failure paths, payment-only paginated `utxos_at` semantics, submit error translation, and connector-core network mapping edge cases
- final review sources:
  - plan review: `.opencode/plans/dolos-utxorpc/task-plans/task-100-plan-review.md`
  - implementation review: `.opencode/plans/dolos-utxorpc/task-plans/task-100-impl-review.md`

## Verification Actually Run

- `cargo fmt --all`
- `cargo fmt --all -- --check`
- `cargo check -p cardano-connector-utxorpc`
- `cargo test -p cardano-connector-utxorpc`
- `cargo clippy -p cardano-connector-utxorpc --all-targets -- -D warnings`
- `cargo doc -p cardano-connector-utxorpc --no-deps`

## Final Build Status

- status: `completed`
- meaning for this task: the touched connector crate compiles cleanly, passes clippy and docs build, and now carries 23 passing unit tests that lock down the core UTxO RPC mapping, parameter, semantics, and submit-path behaviors planned for this task

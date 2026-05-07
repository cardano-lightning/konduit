# Task Plan: task-001 - Add the UTxO RPC connector crate and workspace wiring

- task id: `task-001`
- title: `Add the UTxO RPC connector crate and workspace wiring`
- planning status: `approved`
- implementation status: `approved`
- completion status: `completed`
- final review result: `approved`
- build status: `verified`
- review-log paths:
  - `.opencode/plans/dolos-utxorpc/task-plans/task-001-plan-review.md`
  - `.opencode/plans/dolos-utxorpc/task-plans/task-001-impl-review.md`
- research note: `.opencode/plans/dolos-utxorpc/research/task-001.md`

## Why This Task Was Chosen Now

`task-001` is the first task in `phase-1`, has no dependencies, and is the first
step on the task graph critical path:

`task-001 -> task-002 -> task-003 -> task-100 -> task-101 -> task-102 -> task-103 -> task-104`

This task must happen first because it creates the workspace and crate boundary
that `task-002` needs for the actual UTxO RPC connector implementation, while
preserving the existing Blockfrost path in parallel.

## Interaction Mode

- mode: `autonomous`
- reason: this planning pass only needs repository context, task-graph ordering,
  and existing design constraints. No operator action, implementation-time
  validation, or design decision from the user is required to produce the task
  plan.

## Scope

Fixed scope for `task-001`:

- add a new `cardano-connector-utxorpc` crate to the Rust workspace
- add the minimal workspace dependency wiring required for that crate to exist
  beside `cardano-connector-direct`
- create the minimal crate structure needed for later Dolos UTxO RPC
  implementation work
- preserve the existing Blockfrost connector in parallel

## Non-Goals

- no UTxO RPC client implementation yet
- no health, network, protocol-parameter, UTxO, or submit logic yet
- no server or CLI backend-selection wiring yet
- no startup-readiness checks yet
- no live Dolos validation claims
- no repo-wide backend migration outside the Rust runtime surfaces already
  identified in the task graph

## Relevant Dependencies

- direct task dependencies: none
- downstream tasks unlocked by this task:
  - `task-002` depends on `task-001`
  - the rest of the critical path depends transitively on `task-001`
- crate boundary dependency:
  - `cardano-connector-utxorpc` should depend on `cardano-connector` and
    `cardano-sdk` so it can later satisfy the existing trait contract from
    `rust/crates/cardano-connector/src/connector.rs`

## Research Consulted

- Task-specific research directory: `.opencode/plans/dolos-utxorpc/research/`
- Planning-time result: no relevant research files existed before this task
- Completion-time result: durable findings from `task-001` are now recorded in
  `.opencode/plans/dolos-utxorpc/research/task-001.md`

## Docs, Crate Files, External References, Workflows, And Skills Consulted

- task graph:
  - `.opencode/plans/dolos-utxorpc/dolos-utxorpc-tasks.json`
- design / ADR anchors:
  - `docs/adrs/06-dolos-utxorpc-adaptor-backend.md`
  - `docs/design/33_cardano_connector.md`
  - `docs/design/35_adaptor_deployment.md`
  - `docs/design/36_dolos_utxorpc_implementation_prd.md`
  - `docs/design/37_adaptor_deployment_prd.md`
- workspace / workflow references:
  - `rust/README.md`
  - `.opencode/workflows/rust.md`
  - `rust/Cargo.toml`
- affected Rust crate surfaces reviewed:
  - `rust/crates/cardano-connector/src/connector.rs`
  - `rust/crates/cardano-connector-direct/Cargo.toml`
  - `rust/crates/cardano-connector-direct/src/blockfrost.rs`
  - `rust/crates/cardano-connector-direct/src/lib.rs`
- external references consulted:
  - none
- Rust skills consulted:
  - `rust-router`
  - `coding-guidelines`
  - `m11-ecosystem`
  - `m04-zero-cost`
  - `m06-error-handling`
  - `m15-anti-pattern`
  - `domain-fintech`

## Files Expected To Change

Implementation for this task should stay limited to:

- `rust/Cargo.toml`
- `rust/crates/cardano-connector-utxorpc/Cargo.toml`
- `rust/crates/cardano-connector-utxorpc/src/lib.rs`

Possible follow-up-only files, but not part of this task's minimal acceptance
target:

- none required if the crate skeleton is kept minimal

## Implementation Approach

1. Update `rust/Cargo.toml`. Add `crates/cardano-connector-utxorpc` to
   `[workspace].members` and add a workspace path dependency entry for
   `cardano-connector-utxorpc` if later runtime crates will need a shared
   workspace alias.

2. Create `rust/crates/cardano-connector-utxorpc/Cargo.toml`. Mirror the
   workspace-inherited package metadata style used by
   `cardano-connector-direct`, keep the dependency set minimal, and align with
   repo-validated workspace values instead of introducing crate-local version
   drift.

3. Create `rust/crates/cardano-connector-utxorpc/src/lib.rs`. Add only the
   minimal library surface needed to compile cleanly now and host the future
   implementation. Prefer a small exported type or module placeholder that does
   not pretend the connector is implemented yet.

4. Keep Blockfrost parallelism explicit. Do not touch `cardano-connector-direct`
   behavior during this task. The new crate should exist beside it, not replace
   it.

5. Avoid speculative abstraction. Do not change `CardanoConnector` for
   `task-001`. Trait sufficiency questions belong to later implementation or
   critique if the UTxO RPC data model proves insufficient.

## Acceptance Criteria

- the Rust workspace contains a new `cardano-connector-utxorpc` crate
- the new crate builds in the workspace with truthful, minimal wiring
- the existing Blockfrost connector remains available in parallel

## Verification Plan

Planned implementation-time verification commands:

- `cargo check -p cardano-connector-utxorpc`
- `cargo check --workspace`
- `cargo test -p cardano-connector-utxorpc`

Optional broader verification if the crate skeleton exposes public API or
triggers lint issues:

- `cargo fmt --all -- --check`
- `cargo clippy -p cardano-connector-utxorpc --all-targets -- -D warnings`

Planning note:

- this planning pass does not claim that any build or test has been run for the
  not-yet-created crate

## Implementation Progress

- implementation status: `iteration_1_complete`
- scope completed: added the new workspace member, a minimal
  `cardano-connector-utxorpc` crate manifest, and a compile-safe placeholder
  library surface without backend wiring or connector-trait implementation
- build verification passed:
  - `cargo check -p cardano-connector-utxorpc`
  - `cargo check --workspace`
  - `cargo test -p cardano-connector-utxorpc`
- Blockfrost remains available in parallel because `cardano-connector-direct`
  was left untouched

## Final Outcome

- Final accepted outcome: `task-001` is complete and approved
- Scope landed exactly as a crate-and-workspace-wiring step only
- No trait changes, backend-selection wiring, readiness checks, or Dolos runtime
  behavior were added in this task
- Final review sources:
  - plan review:
    `.opencode/plans/dolos-utxorpc/task-plans/task-001-plan-review.md`
  - implementation review:
    `.opencode/plans/dolos-utxorpc/task-plans/task-001-impl-review.md`

## Verification Actually Run

Commands actually run and re-confirmed in the implementation review log:

- `cargo check -p cardano-connector-utxorpc`
- `cargo test -p cardano-connector-utxorpc`
- `cargo check --workspace`

Verification not claimed for this task:

- `cargo fmt --all -- --check` was not recorded as run
- `cargo clippy -p cardano-connector-utxorpc --all-targets -- -D warnings` was
  not recorded as run
- `cargo doc --workspace --no-deps` was not recorded as run

## Final Build Status

- Status: `verified`
- Meaning for this task: the new crate skeleton and updated workspace passed the
  targeted `cargo check` and `cargo test` commands that were actually run;
  broader formatting, clippy, and docs verification remain unclaimed

## Risks / Open Questions

- workspace dependency aliasing: `task-001` acceptance does not strictly require
  a `[workspace.dependencies]` alias for `cardano-connector-utxorpc`;
  implementation should keep this minimal unless a downstream crate immediately
  needs that alias
- placeholder shape: the crate skeleton should compile without implying
  completed backend behavior; a minimal placeholder API is safer than a fake
  `CardanoConnector` impl
- future UTxO RPC crate choice is still open at planning time; `m11-ecosystem`
  review suggests deferring any dependency expansion until `task-002` clarifies
  the actual client surface needed

Documented source conflicts:

- `rust/README.md` uses older crate names such as `cardano-connect` and
  `cardano-connect-blockfrost`, while `rust/Cargo.toml` and the actual crate
  paths use `cardano-connector` and `cardano-connector-direct`. Repo-validated
  workspace paths should win for this task.
- `rust-router` default project settings mention `rust-version = "1.85"`, but
  this workspace is explicitly pinned to `rust-version = "1.94.0"` in
  `rust/Cargo.toml`. The new crate should inherit the workspace value rather
  than introducing a conflicting crate-local override.

Current accepted guidance for `task-001`:

- the workspace manifest and current Rust workflow docs were treated as
  canonical over older historical naming in `rust/README.md`
- the workspace `rust-version = "1.94.0"` was treated as canonical over the
  skill's generic default guidance
- `.opencode/workflows/rust.md` materially affected execution by requiring
  `rust-router` first, targeted Rust skills, and truthful verification reporting
  instead of claiming unrun tools

## Required Docs / Tracking / Research Updates

Updates required from later phases or subagents:

- `task-002` should update this plan if the chosen UTxO RPC client crate or
  trait-fit analysis changes the expected crate shape
- `task-002` or a critique pass should record any connector-trait insufficiency
  discovered while implementing real network, health, parameter, UTxO, or submit
  behavior
- `task-003` should update planning/tracking docs with the final
  backend-selection config surface for `konduit-server` and `konduit-cli`
- `task-103` should append truthful live Dolos validation evidence rather than
  inferred results
- `task-104` should update the runtime-facing docs listed in the task graph once
  the implementation is complete
- if future research is needed on UTxO RPC Rust crates, it should be added under
  `.opencode/plans/dolos-utxorpc/research/` and referenced back into this plan

## Plan Summary

Implement `task-001` as a narrow crate-and-workspace-wiring change only: add
`cardano-connector-utxorpc` beside the existing direct Blockfrost connector,
keep dependencies minimal, avoid trait or runtime-surface changes, and verify
the new crate compiles cleanly without claiming any Dolos functionality yet.

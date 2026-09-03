# Task Plan: task-103 - Run live Dolos validation and capture operator evidence

- task id: `task-103`
- title: `Run live Dolos validation and capture operator evidence`
- planning status: `approved`
- build status: `completed`
- interaction mode: `manual_execution`
- review-log paths:
- `.opencode/plans/dolos-utxorpc/task-plans/task-103-plan-review.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-103-impl-review.md`

## Why This Task Was Chosen Now

`task-103` was the next lowest-ID unblocked pending task on the remaining
critical path after `task-102`. The code-path implementation and autonomous
smoke coverage were already complete, so the required next step was live
Dolos-backed validation and operator evidence capture before final documentation
cleanup in `task-104`.

## Interaction Mode

- mode: `manual_execution`
- reason: this task's acceptance criteria require a real Dolos-backed
  environment, live readiness checks, and at least one live submission path or a
  documented blocker. Those steps depend on operator-managed infrastructure,
  environment-specific connectivity, and human-observed results that the agent
  cannot truthfully produce alone in the current workspace.
- required user inputs: environment-specific runtime values needed to execute
  the live checks, plus the resulting command outputs or observed outcomes
- required manual test steps: start or identify the target Dolos-backed
  environment, run the server and CLI validation commands against it, and
  capture the outputs or blockers
- evidence needed back from the user: command transcripts or equivalent output
  for reachability, network match, live protocol-parameter derivation,
  reference-script resolution, and at least one live transaction submission path
  or a clearly explained blocker
- can implementation proceed before user interaction: yes, for operator-handoff
  preparation only; truthful task completion cannot proceed past the handoff
  without user-provided evidence

## Scope

This task is limited to preparing and then truthfully executing or handing off
live Dolos validation for the Rust runtime path already implemented in scope.

In scope:

- reconcile the documented live validation requirements with the current
  repository behavior
- define the smallest truthful operator-run validation sequence covering the
  required readiness and submission checks
- identify the exact runtime surfaces and configuration needed for live
  validation in `konduit-server` and `konduit-cli`
- provide a concise operator handoff with commands, expected results, failure
  interpretation, rollback notes, and required evidence capture
- record the live-validation outcome or blocker once the operator responds
- record the accepted live-validation evidence or blocker in the task-local
  tracked surfaces before closing the task

## Non-Goals

- no speculative new runtime features, connector behavior changes, or deployment
  automation unless live validation exposes a concrete defect after operator
  evidence is returned
- no fake or mocked completion of live checks
- no repo-wide deployment work outside the scoped Rust runtime surfaces and
  documentation needed to capture truthful validation results
- no final documentation cleanup sweep beyond task-local plan, review-log, and
  research updates; that belongs to `task-104`

## Relevant Dependencies

- direct task dependency: `task-102`
- upstream code dependencies already completed: `task-001`, `task-002`,
  `task-003`, `task-100`, `task-101`, `task-102`
- downstream task unlocked by this task: `task-104`
- durable research output expected from this task:
  `.opencode/plans/dolos-utxorpc/research/task-103.md`

## Research Consulted

- `.opencode/plans/dolos-utxorpc/research/task-002.md`
- `.opencode/plans/dolos-utxorpc/research/task-003.md`
- `.opencode/plans/dolos-utxorpc/research/task-101.md`
- `.opencode/plans/dolos-utxorpc/research/task-102.md`

Key carry-forward findings from prior research:

- `konduit-server` startup truthfulness is split between backend construction in
  `src/cardano/args.rs` and readiness composition in `src/admin/service.rs`
- the smallest truthful server startup proof is successful
  `admin::Service::new(...)` with live protocol parameters and a resolvable
  reference script UTxO
- the smallest truthful integrated submit proof already identified for
  autonomous smoke coverage is the CLI admin `send` path because it composes
  network-derived address selection, UTxO lookup, protocol parameters, tx
  building, signing, and submit
- workspace verification is already green in the current repository state, so
  this task should focus on live-environment evidence rather than more local
  autonomous verification

## Docs, Crate Files, External References, And Skills Consulted

- `.opencode/plans/dolos-utxorpc/dolos-utxorpc-tasks.json`
- `.opencode/plans/dolos-utxorpc/research/task-002.md`
- `.opencode/plans/dolos-utxorpc/research/task-003.md`
- `.opencode/plans/dolos-utxorpc/research/task-101.md`
- `.opencode/plans/dolos-utxorpc/research/task-102.md`
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
- `rust/crates/konduit-server/src/main.rs`
- `rust/crates/konduit-server/src/cardano.rs`
- `rust/crates/konduit-server/src/cardano/args.rs`
- `rust/crates/konduit-server/src/admin/service.rs`
- `rust/crates/konduit-cli/src/connector.rs`
- `rust/crates/konduit-cli/src/config/connector.rs`
- `rust/crates/konduit-cli/src/env/connector.rs`
- `rust/crates/konduit-cli/src/cmd/admin/tx.rs`
- `rust/crates/konduit-cli/src/tip.rs`
- external reference: `https://github.com/utxorpc/rust-sdk`
- external reference: `https://github.com/txpipe/dolos`
- Rust skills consulted: `rust-router`, `m09-domain`, `m11-ecosystem`,
  `m13-domain-error`, `domain-fintech`, `domain-cli`, `cardano-protocol-params`

## Files Expected To Change

- `.opencode/plans/dolos-utxorpc/task-plans/task-103.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-103-plan-review.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-103-impl-review.md`
- `.opencode/plans/dolos-utxorpc/research/task-103.md`

Possible follow-up documentation updates only after live evidence is returned:

- `.opencode/plans/dolos-utxorpc/dolos-utxorpc-tasks.json`
- `docs/design/33_cardano_connector.md`
- `docs/design/36_dolos_utxorpc_implementation_prd.md`

## Implementation Approach

1. Keep the task focused on truthful live validation rather than more local
   automation.

Use the implemented runtime entry points as they exist today. Do not invent new
harnesses or attempt to simulate Dolos. The implementation work for this task is
to prepare the operator-facing validation procedure and capture results.

2. Validate the required startup and runtime checks through the existing server
   and CLI surfaces.

The handoff should cover:

- backend reachability through UTxO RPC
- configured network versus live-network match
- live protocol-parameter derivation
- configured reference-script resolution during server startup or equivalent
  CLI/server path
- at least one live submission path using an existing runtime flow, with the CLI
  admin `send` path as the preferred minimal submit seam

3. Require concrete evidence capture, not narrative confirmation.

The handoff should ask for exact command outputs, relevant logs, and any
transaction identifier or backend rejection details. If the operator cannot or
should not run a live submission, the task can still progress only if the
blocker is concrete, environment-specific, and documented with evidence.

4. Map each acceptance criterion to a concrete evidence artifact.

The handoff and the final task record must use this evidence matrix:

- reachability: server startup output or CLI connector initialization output
  showing the UTxO RPC backend at the chosen `KONDUIT_UTXORPC_URI` was contacted
  successfully, or the exact reachability failure
- configured network match: startup or CLI output showing the configured
  `KONDUIT_NETWORK` matched the live network reported by Dolos, or the exact
  mismatch error
- live protocol-parameter derivation: successful `konduit-server` startup beyond
  `cardano/args.rs` into `admin::Service::new(...)` or successful CLI tx-path
  execution far enough to fetch protocol parameters, or the exact
  protocol-parameter derivation failure
- reference-script resolution: successful `konduit-server` startup beyond
  `admin::Service::new(...)` with the configured host address or an equivalent
  CLI/runtime output proving the reference script UTxO was resolved, or the
  exact missing-script failure
- live submission path: CLI `admin send` output showing backend submit
  acceptance together with the emitted transaction id, or the exact backend
  rejection/transport failure; this proves backend acceptance only and must not
  be described as chain confirmation unless a separate follow-up confirmation
  check is also captured

5. Keep rollback and operator safety explicit.

Because this is a live-environment task, the handoff must include rollback notes
and a reminder to use a controlled environment, small funds, or a non-production
profile when submitting a transaction.

## Acceptance Criteria

- a real Dolos-backed validation run is performed or handed off truthfully to
  the operator
- validation covers reachability, configured network match, live protocol
  parameter derivation, and reference script resolution
- validation includes at least one live transaction submission path showing
  backend acceptance and transaction id or a clearly documented blocker

## Verification Plan

Agent-executable verification for this task before handoff:

- confirm the selected live validation seam against the current code paths in
  `konduit-server` and `konduit-cli`
- confirm the required config surface from the implemented env or CLI parsing
  paths
- confirm the repository's current autonomous verification baseline from
  `task-102`

Manual/operator verification required for truthful completion:

- start or identify a live Dolos-backed environment reachable from the target
  Konduit runtime
- run a server startup against `backend=utxorpc` with the same explicit
  `KONDUIT_UTXORPC_URI` and `KONDUIT_NETWORK` values that will be used for the
  CLI validation path
- capture evidence that startup passed reachability, live-network match,
  protocol-parameter derivation, and reference-script resolution, or capture the
  exact blocker for each item using the evidence matrix above
- run at least one live CLI submission path against the same backend and capture
  the emitted transaction id on backend acceptance, or capture the exact blocker
  preventing safe submission
- if desired or available, separately capture whether the submitted transaction
  later appears on chain, but do not treat that extra confirmation as part of
  the base `submit()` contract unless explicit evidence is returned

## Risks / Open Questions

- the workspace does not expose the operator's live Dolos endpoint, wallet
  material, reference-script host address, or target network from the current
  agent context, so the handoff must avoid guessing environment values
- live submission may require funded test credentials or an operator-approved
  environment; if absent, the blocker must be recorded rather than papered over
- if the operator's Dolos deployment differs materially from the documented
  localhost-only target profile, the task must capture that drift for `task-104`
  docs cleanup rather than silently normalizing it

## Required Docs / Tracking / Research Updates

- append planning critique results to
  `.opencode/plans/dolos-utxorpc/task-plans/task-103-plan-review.md`
- during implementation, create and maintain
  `.opencode/plans/dolos-utxorpc/task-plans/task-103-impl-review.md`
- write durable findings to
  `.opencode/plans/dolos-utxorpc/research/task-103.md`, including the chosen
  live-validation seam, operator prerequisites, observed blockers, and final
  evidence state; if no new durable finding exists before user response, record
  `no new research`
- after operator evidence returns, update this task's canonical plan doc and
  implementation review log with the exact evidence or blocker summary before
  closing the task
- update `.opencode/plans/dolos-utxorpc/dolos-utxorpc-tasks.json` only after
  operator evidence is returned and the task outcome is truthful
- if the returned evidence exposes a durable behavior or blocker that materially
  changes current operational truth, update the task-targeted docs for this task
  rather than relying only on `task-104` to backfill it later

## Plan Summary

Implement `task-103` as a `manual_execution` task by preparing the smallest
truthful live-validation handoff for the already-built UTxO RPC runtime path,
centered on server startup readiness checks and an existing CLI submission path,
with an explicit evidence matrix for reachability, network match,
protocol-parameter derivation, reference-script resolution, and backend submit
acceptance, then pause for operator-run evidence before any completion or
downstream task selection.

## Implementation Progress

- implementation status: `completed`
- scope completed: prepared the operator-facing live-validation handoff,
  incorporated returned live-environment evidence, applied connector fixes for
  Dolos `network_id` casing plus payment-and-delegation UTxO lookup semantics,
  refactored `konduit-cli` onto a single Tokio runtime so UTxO RPC validation
  and later gRPC calls share one runtime, corrected the inverted
  `admin tx deploy` `spend_all` filter, and completed the live Dolos-backed
  reference-script deploy plus server startup validation
- durable research recorded at
  `.opencode/plans/dolos-utxorpc/research/task-103.md`

## Current Outcome

- current implementation outcome: `completed`
- implementation review log:
  `.opencode/plans/dolos-utxorpc/task-plans/task-103-impl-review.md`
- final review result: `approved`

## Verification Actually Run

- verified the live-validation seam against the current runtime sources in
  `rust/crates/konduit-server/src/cardano/args.rs`,
  `rust/crates/konduit-server/src/admin/service.rs`,
  `rust/crates/konduit-cli/src/connector.rs`, and
  `rust/crates/konduit-cli/src/cmd/admin/tx.rs`
- verified the current autonomous baseline from `task-102` and the current task
  tracker state
- verified that truthful completion of this task remains blocked on operator-run
  live Dolos evidence rather than additional local automation
- verified the connector compatibility fix with `cargo fmt --all`,
  `cargo test -p cardano-connector-utxorpc network_from_genesis`, and
  `cargo check -p cardano-connector-utxorpc`
- verified the CLI runtime-lifecycle and deploy-filter fix with
  `cargo fmt --all`, `cargo check -p konduit-cli`, and
  `cargo test -p konduit-cli`
- verified the final payment-and-delegation UTxO lookup fix with
  `cargo test -p cardano-connector-utxorpc`,
  `cargo test -p konduit-server new_fails_when_reference_script_is_missing`, and
  fresh sequential
  `cargo build -p konduit-cli -p konduit-server && ./target/debug/konduit-server`

## Final Outcome

- interaction completed with operator-provided live Dolos evidence
- live transaction submission evidence captured through
  `./target/debug/konduit-cli admin tx deploy` with tx id
  `b090e09ae05b947e2818f807dba874a205acacf1ffc4a3c5a53b8bc1cfe5c0ab`
- live startup evidence captured through fresh sequential
  `cargo build -p konduit-cli -p konduit-server && ./target/debug/konduit-server`,
  which started the server successfully and reached steady state
- the post-startup
  `Admin sync failed: insufficient total gain: preferences.min_total = 1000000, gain = 0`
  log was observed after readiness and reflects the background sync loop finding
  no actionable gain rather than a startup failure
- the implementation review loop closed cleanly with `Code Review: Iteration 5`
  approval in `.opencode/plans/dolos-utxorpc/task-plans/task-103-impl-review.md`

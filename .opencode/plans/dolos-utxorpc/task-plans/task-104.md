# Task Plan: task-104 - Finalize Rust-runtime documentation and implementation cleanup

- task id: `task-104`
- title: `Finalize Rust-runtime documentation and implementation cleanup`
- planning status: `approved`
- build status: `completed`
- interaction mode: `autonomous`
- review-log paths:
- `.opencode/plans/dolos-utxorpc/task-plans/task-104-plan-review.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-104-impl-review.md`

## Why This Task Was Chosen Now

`task-104` is the next lowest-ID unblocked pending task on the remaining critical path after `task-103`. The implementation, automated verification, and live Dolos validation are already complete, so the remaining work is to reconcile the final repository state with the Rust-runtime-facing docs, capture the accepted operational model, and land any small cleanup needed to leave the scoped surfaces in a reviewable state.

## Interaction Mode

- mode: `autonomous`
- reason: the task is limited to repository-local documentation and cleanup work that can be completed truthfully from the current codebase, task tracker, prior live-validation evidence, and existing docs without additional user decisions or operator-run checks
- required user inputs: none
- required manual test steps: none beyond the already captured `task-103` live validation evidence
- evidence needed back from the user: none
- can implementation proceed before user interaction: yes; no user interaction is required for truthful completion

## Scope

This task is limited to final Rust-runtime-facing documentation alignment and any small repository-local cleanup needed to reflect the implemented UTxO RPC backend truthfully.

In scope:

- update the governing ADR and design docs to describe the final implemented runtime behavior rather than pre-implementation assumptions
- document the explicit backend-selection and UTxO RPC configuration surfaces used by `konduit-server` and `konduit-cli`
- document the final startup and readiness model, including the split between backend initialization and server startup blockers
- capture the live-validation findings from `task-103` where they materially change operator or developer guidance
- preserve truthful documentation for both supported backends, including the current Blockfrost defaults and validation behavior that remain available in parallel with UTxO RPC
- update `rust/README.md` and `rust/crates/konduit-cli/README.md` where they materially misdescribe the current runtime surfaces
- update `.opencode/workflows/rust.md` where it still describes the UTxO RPC crate or verification guidance as future work instead of current workspace reality
- make the smallest additional cleanup needed if current implementation-facing docs still drift from the repository state discovered during this task

## Non-Goals

- no new Cardano backend features or behavior changes unless a repository-local docs contradiction reveals a concrete cleanup that is required for correctness
- no new live validation, deployment execution, or environment-specific operator work; `task-103` already captured that evidence
- no repo-wide documentation rewrite outside the targeted runtime surfaces and task-local tracking artifacts
- no speculative guidance for unrelated repository subprojects such as `cardano-connector-server`

## Relevant Dependencies

- direct task dependency: `task-103`
- upstream implementation and validation dependencies already completed: `task-001`, `task-002`, `task-003`, `task-100`, `task-101`, `task-102`, `task-103`
- this is the final task on the recorded critical path
- durable research output expected from this task: `.opencode/plans/dolos-utxorpc/research/task-104.md`

## Research Consulted

- `.opencode/plans/dolos-utxorpc/research/task-002.md`
- `.opencode/plans/dolos-utxorpc/research/task-003.md`
- `.opencode/plans/dolos-utxorpc/research/task-102.md`
- `.opencode/plans/dolos-utxorpc/research/task-103.md`

Key carry-forward findings from prior research:

- the UTxO RPC connector derives live protocol parameters from Dolos rather than static presets and treats Dolos as the authoritative runtime source for health, network, UTxO, and submit data
- server readiness is split between backend construction in `konduit-server/src/cardano/args.rs` and startup blockers in `konduit-server/src/admin/service.rs`
- CLI execution now runs under a single top-level Tokio runtime, which is relevant to final runtime documentation for the UTxO RPC path
- live validation in `task-103` confirmed successful localhost Dolos submission and startup, while also surfacing accepted implementation details around case-insensitive Dolos `network_id` handling, payment-only UTxO lookup plus local delegation filtering, and the non-blocking nature of the later `insufficient total gain` background sync log
- live validation in `task-103` also established a concrete Dolos compatibility prerequisite for this backend: Konduit startup and network validation depend on Dolos serving `read_genesis`, and the observed `dolos 1.0.3` environment required an operator-side patch before validation could pass

## Docs, Crate Files, External References, And Skills Consulted

- `.opencode/plans/dolos-utxorpc/dolos-utxorpc-tasks.json`
- `.opencode/plans/dolos-utxorpc/research/task-002.md`
- `.opencode/plans/dolos-utxorpc/research/task-003.md`
- `.opencode/plans/dolos-utxorpc/research/task-102.md`
- `.opencode/plans/dolos-utxorpc/research/task-103.md`
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
- `rust/crates/cardano-connector-utxorpc/src/lib.rs`
- `rust/crates/konduit-server/src/cardano.rs`
- `rust/crates/konduit-server/src/cardano/args.rs`
- `rust/crates/konduit-server/src/env.rs`
- `rust/crates/konduit-server/src/admin/service.rs`
- `rust/crates/konduit-cli/src/main.rs`
- `rust/crates/konduit-cli/src/cmd.rs`
- `rust/crates/konduit-cli/src/cmd/admin.rs`
- `rust/crates/konduit-cli/src/cmd/adaptor.rs`
- `rust/crates/konduit-cli/src/cmd/consumer.rs`
- `rust/crates/konduit-cli/src/connector.rs`
- `rust/crates/konduit-cli/src/config/connector.rs`
- `rust/crates/konduit-cli/src/env/base.rs`
- `rust/crates/konduit-cli/src/env/admin.rs`
- `rust/crates/konduit-cli/src/env/adaptor.rs`
- `rust/crates/konduit-cli/src/env/consumer.rs`
- `rust/crates/konduit-cli/src/env/connector.rs`
- `rust/crates/konduit-cli/src/cmd/admin/tx.rs`
- `rust/crates/konduit-cli/src/shared/setup.rs`
- `rust/crates/konduit-cli/README.md`
- Rust skills consulted: `rust-router`, `domain-cli`, `domain-fintech`, `cardano-protocol-params`

## Files Expected To Change

- `.opencode/plans/dolos-utxorpc/task-plans/task-104.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-104-plan-review.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-104-impl-review.md`
- `.opencode/plans/dolos-utxorpc/research/task-104.md`
- `.opencode/plans/dolos-utxorpc/dolos-utxorpc-tasks.json`
- `.opencode/workflows/rust.md`
- `docs/adrs/06-dolos-utxorpc-adaptor-backend.md`
- `docs/design/33_cardano_connector.md`
- `docs/design/35_adaptor_deployment.md`
- `docs/design/36_dolos_utxorpc_implementation_prd.md`
- `docs/design/37_adaptor_deployment_prd.md`
- `rust/crates/konduit-cli/README.md`
- `rust/README.md`

Potential code cleanup only if a concrete docs-to-code contradiction requires it:

- `rust/crates/konduit-cli/src/*`
- `rust/crates/konduit-server/src/*`
- `rust/crates/cardano-connector-utxorpc/src/*`

## Implementation Approach

1. Reconcile the final runtime truth against the docs, not the other way around.

Use the current repository state plus the recorded `task-103` live evidence as the source for final runtime behavior. Any doc that still says the env var surface or readiness model is unknown should be updated to the implemented truth.

2. Keep documentation changes targeted to the scoped Rust runtime surfaces.

Focus on the ADR, connector design, deployment design, implementation PRD, deployment PRD, workspace overview, and CLI README. Avoid broad edits outside those surfaces unless a linked reference must be adjusted for consistency.

Before changing CLI-facing docs, verify the actual clap command surfaces and setup output path so README examples match the current runtime shape for `setup`, `.env.<role>` loading, and the role command boundaries.

Verify the env/config loaders as well, because the truth for `KONDUIT_CARDANO_BACKEND`, `KONDUIT_NETWORK`, `KONDUIT_UTXORPC_URI`, Blockfrost network inference, and role-local dotenv defaults comes from the server env constants and the CLI env modules, not just the clap command shells.

3. Record final backend-selection and readiness behavior concretely.

The updated docs should capture at least:

- explicit backend selection between `blockfrost` and `utxorpc`
- the still-supported Blockfrost behavior that differs from UTxO RPC, including backend defaults and where network inference or defaults remain valid only for Blockfrost
- explicit `KONDUIT_CARDANO_BACKEND`, `KONDUIT_UTXORPC_URI`, and `KONDUIT_NETWORK` requirements for the UTxO RPC backend
- the fact that Dolos is authoritative for live protocol parameters, UTxO data, and transaction submission on this backend
- that `konduit-server` startup blocks on reachability, live-network match, protocol-parameter derivation, and reference-script resolution
- that the CLI only performs live reachability and network validation during connector construction for the UTxO RPC backend, while the Blockfrost path validates project-id presence and network-prefix consistency but otherwise fails later on API use; config-derived address and config display remain live-connector-independent
- the accepted `utxos_at` semantics for the UTxO RPC backend, including payment-only lookup when delegation is absent and local enforcement of the exact payment-and-delegation pair when delegation is present after payment-credential paging
- the residual Blockfrost differences that remain in parallel, including its static per-network protocol-parameter fallback and the concrete `utxos_at(payment, None)` limitation where the current direct Blockfrost path still queries one constructed address instead of the broader payment-credential-wide contract

4. Fold durable live-validation findings into operator-facing guidance.

Document the observed live submission and startup evidence from `task-103`, note that Dolos `network_id` casing is tolerated while `network_magic` remains authoritative, and clarify that the post-startup `insufficient total gain` log is a background admin-sync condition rather than a startup readiness failure.

Document the observed Dolos compatibility prerequisite from live validation as well: Konduit's UTxO RPC startup path depends on Dolos successfully serving `read_genesis`, so operators need a Dolos build or version that supports that call before expecting backend initialization and network validation to succeed.

Record the accepted UTxO RPC delegated-lookup implementation detail and watchpoint explicitly: current correctness comes from payment-credential paging plus local delegation filtering, not from a guaranteed Dolos-side delegation index, so future readers should treat that behavior as backend-specific and potentially relevant to scale.

5. Correct stale secret-handling and operator guidance while keeping local-dev ergonomics explicit.

If README examples still suggest storing runtime secrets or long-lived operator configuration inside the repository checkout, update them to match the deployment docs: secrets belong outside the repo for real deployments. It is acceptable to keep local-dev examples, but they must be framed clearly as local/test convenience rather than production guidance, the docs should stay truthful to the current `setup` commands plus `.env`, `.env.admin`, `.env.adaptor`, and `.env.consumer` loading behavior, and they should state plainly that `setup` prints sensitive config material to stdout and must be handled accordingly.

6. Keep cleanup minimal and verification truthful.

Prefer docs-only changes. If a small implementation cleanup is still required to make the docs truthful, keep it narrowly scoped and rerun only the relevant verification for the touched surfaces.

## Acceptance Criteria

- relevant docs describe the final Rust runtime behavior rather than Blockfrost-only or pre-implementation assumptions
- the UTxO RPC config and readiness model are documented clearly for `konduit-server` and `konduit-cli`
- the touched implementation and documentation surfaces are left in a clean, reviewable state

## Verification Plan

Required verification:

- re-read the affected docs after editing to confirm they match the current runtime surfaces and `task-103` evidence
- verify the CLI README against the actual clap command and setup surfaces in `src/cmd.rs`, the role command modules, and `src/shared/setup.rs` so local-dev guidance and production caveats both remain truthful
- verify the final config wording against `konduit-server/src/env.rs` plus the CLI env modules so backend selection, dotenv precedence, role-local defaults, and explicit UTxO RPC requirements are described exactly as implemented
- run targeted repository checks appropriate to the actual edits; at minimum this includes verifying the task tracker, task-plan docs, and research notes stay synchronized
- if the task remains docs-only, no new Rust build or test commands are required beyond consistency checks; if code cleanup is needed, run the narrowest truthful cargo verification for the touched crates and record it exactly

## Risks / Open Questions

- some older docs, especially `rust/README.md` and the CLI README, predate the current crate names and backend-selection model, so the task should correct factual drift without overpromising broader modernization outside scope
- the deployment docs still contain future-tense language from before implementation; updates should distinguish between implemented runtime truth and still-open operational details such as exact `systemd` unit files or tuned rate limits
- if the docs reveal a residual mismatch between implementation and accepted design, that conflict must be documented in the task artifacts and resolved with the smallest truthful repo-local change rather than silently choosing one source

## Required Docs / Tracking / Research Updates

- append planning critique results to `.opencode/plans/dolos-utxorpc/task-plans/task-104-plan-review.md`
- during implementation, create and maintain `.opencode/plans/dolos-utxorpc/task-plans/task-104-impl-review.md`
- write durable findings to `.opencode/plans/dolos-utxorpc/research/task-104.md`, including the final docs drift corrected, any residual limitations left explicit, whether code cleanup was or was not required, the current UTxO RPC delegated-lookup performance watchpoint, the live Dolos `read_genesis` compatibility prerequisite, and any remaining Blockfrost connector-contract limitation left explicit rather than silently normalized
- record whether the final docs pass updated `.opencode/workflows/rust.md`, preserved truthful Blockfrost parity notes and explicit non-parity where needed, and corrected any stale repo-local secret-handling guidance in README surfaces
- update `.opencode/plans/dolos-utxorpc/dolos-utxorpc-tasks.json` after the task is approved and completed
- keep this canonical plan doc synchronized with the final approved plan, implementation outcome, review results, and verification summary

## Plan Summary

Implement `task-104` as an `autonomous` documentation-alignment task that updates the ADR, design docs, deployment docs, Rust workflow guide, workspace overview, CLI README, and task tracking artifacts to match the implemented Cardano-backend truth from the current repository and `task-103` live evidence, including both UTxO RPC and the still-supported Blockfrost path, while keeping any cleanup minimal and limited to concrete repo-local drift.

## Implementation Progress

- implementation status: `completed`
- scope completed: updated the ADR, connector design, deployment docs, implementation PRD, deployment PRD, Rust workflow guide, workspace overview, and CLI README to reflect the implemented backend-selection/config truth, the UTxO RPC startup/readiness model, the live Dolos `read_genesis` prerequisite, the UTxO RPC delegated-lookup watchpoint, the remaining direct Blockfrost non-parity points, the current clap ordering for role-level connector flags in CLI `setup` examples, and the parsed-config versus live-connector split for UTxO RPC CLI commands
- durable research recorded at `.opencode/plans/dolos-utxorpc/research/task-104.md`

## Current Outcome

- current implementation outcome: `completed`
- implementation review log: `.opencode/plans/dolos-utxorpc/task-plans/task-104-impl-review.md`
- final review result: `approved`

## Verification Actually Run

- read the task tracker, relevant research notes, governing docs, workflow, workspace manifest, and affected crate surfaces for the current runtime truth
- searched the targeted docs and README surfaces for stale Blockfrost-only language, missing UTxO RPC configuration details, and deployment text still phrased as pre-implementation future work
- re-read the CLI clap surfaces, env loaders, config display paths, server env constants, and the current runtime bootstrap files so the final docs reflect actual command behavior, dotenv precedence, backend validation differences, and config-derived versus live-connector-dependent commands
- performed iterative docs verification against implementation-review findings until the current repository state closed cleanly with approved recovery entries in the planning and implementation logs

## Final Outcome

- planning loop reached the max-iteration guard after `Critiquer: Iteration 5`
- user selected the strict truth-first documentation option, and the implementation pass completed with approved review under that scope

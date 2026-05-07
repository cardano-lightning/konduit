# Task Plan: task-003 - Wire backend selection into the Rust runtime surfaces

- task id: `task-003`
- title: `Wire backend selection into the Rust runtime surfaces`
- planning status: `approved`
- build status: `completed`
- interaction mode: `autonomous`
- review-log paths:
- `.opencode/plans/dolos-utxorpc/task-plans/task-003-plan-review.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-003-impl-review.md`

## Why This Task Was Chosen Now

`task-003` is the next unblocked task on the recorded critical path after `task-002` completed the UTxO RPC connector core. It must happen now because the new backend is not yet selectable from the server or CLI runtime surfaces, so the connector exists but cannot truthfully be used in the scoped Konduit runtime paths.

## Interaction Mode

- mode: `autonomous`
- reason: the locked ADR, design docs, completed `task-002` connector work, and current runtime crate surfaces are sufficient to implement explicit backend selection and startup-equivalent readiness logic without requiring a user architecture choice or live operator checkpoint in this task.
- required user inputs: none
- required manual test steps: none during this task
- evidence needed back from the user: none
- can implementation proceed before user interaction: yes, because live Dolos validation remains a later phase-2 task and this task can be completed truthfully with code changes plus agent-executable verification only

## Scope

This task is limited to the Rust runtime surfaces that currently instantiate or configure the direct Blockfrost connector.

In scope:

- add explicit backend selection between `blockfrost` and `utxorpc` in `konduit-server`
- add explicit backend selection between `blockfrost` and `utxorpc` in `konduit-cli`
- add explicit UTxO RPC URI and Cardano network configuration surfaces for both runtime crates
- keep Blockfrost available in parallel rather than replacing it
- add startup/readiness validation for the server UTxO RPC backend covering reachability, network match, live protocol-parameter derivation, and configured reference-script UTxO resolution
- make current CLI runtime flows construct and use either backend through the existing config path
- keep backend-specific logic layered inside connector-focused runtime modules rather than spreading provider logic through unrelated crates

## Non-Goals

- no repo-wide backend migration outside `konduit-server`, `konduit-cli`, and shared connector implementation layers already in scope
- no changes to `cardano-connector-server` or unrelated repository subprojects
- no fake claim that live Dolos validation or end-to-end submission against operator infrastructure has been performed here
- no removal of existing Blockfrost env/config behavior unless replaced by an explicit equivalent in the new selection model
- no fallback to static per-network protocol parameters, local genesis files, or `cardano-node` artifacts for the UTxO RPC backend
- no broad connector-trait redesign unless the current runtime layering proves it unavoidable

## Relevant Dependencies

- direct task dependency: `task-002`
- downstream tasks unlocked by this task:
- `task-101`
- current connector trait boundary: `rust/crates/cardano-connector/src/connector.rs`
- new backend implementation: `rust/crates/cardano-connector-utxorpc/src/lib.rs`
- current direct-provider behavior reference: `rust/crates/cardano-connector-direct/src/blockfrost.rs`
- server runtime consumer that depends on ready Cardano state: `rust/crates/konduit-server/src/admin/service.rs`

## Research Consulted

- `.opencode/plans/dolos-utxorpc/research/task-001.md`
- `.opencode/plans/dolos-utxorpc/research/task-002.md`

Key carry-forward findings from prior research:

- `task-002` established the truthful UTxO RPC connector shape, including explicit endpoint/network config, paginated payment-only UTxO lookup semantics, live protocol-parameter derivation, and endpoint-contextual error handling
- `rust/Cargo.toml` and live crate paths remain more trustworthy than older historical naming in `rust/README.md`
- the current UTxO RPC connector already supports localhost `http://` Dolos endpoints and exposes the health/params/utxo/submit surface needed by runtime selection work

## Docs, Crate Files, External References, And Skills Consulted

- `.opencode/plans/dolos-utxorpc/dolos-utxorpc-tasks.json`
- `.opencode/plans/dolos-utxorpc/research/task-001.md`
- `.opencode/plans/dolos-utxorpc/research/task-002.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-002.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-002-plan-review.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-002-impl-review.md`
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
- `rust/crates/cardano-connector-utxorpc/src/config.rs`
- `rust/crates/cardano-connector-utxorpc/src/mapping.rs`
- `rust/crates/cardano-connector-utxorpc/src/params.rs`
- `rust/crates/konduit-server/Cargo.toml`
- `rust/crates/konduit-server/src/main.rs`
- `rust/crates/konduit-server/src/args.rs`
- `rust/crates/konduit-server/src/cardano.rs`
- `rust/crates/konduit-server/src/cardano/args.rs`
- `rust/crates/konduit-server/src/admin/service.rs`
- `rust/crates/konduit-server/src/admin/config.rs`
- `rust/crates/konduit-server/src/env.rs`
- `rust/crates/konduit-cli/Cargo.toml`
- `rust/crates/konduit-cli/src/connector.rs`
- `rust/crates/konduit-cli/src/config/connector.rs`
- `rust/crates/konduit-cli/src/config/admin.rs`
- `rust/crates/konduit-cli/src/config/adaptor.rs`
- `rust/crates/konduit-cli/src/config/consumer.rs`
- `rust/crates/konduit-cli/src/env/connector.rs`
- `rust/crates/konduit-cli/src/env/admin.rs`
- `rust/crates/konduit-cli/src/env/adaptor.rs`
- `rust/crates/konduit-cli/src/env/consumer.rs`
- `rust/crates/konduit-cli/src/cmd/admin/show.rs`
- `rust/crates/konduit-cli/src/cmd/admin/tx.rs`
- `rust/crates/konduit-cli/src/cmd/adaptor/show.rs`
- `rust/crates/konduit-cli/src/cmd/adaptor/tx.rs`
- `rust/crates/konduit-cli/src/cmd/consumer/show.rs`
- `rust/crates/konduit-cli/src/cmd/consumer/tx.rs`
- `rust/crates/konduit-cli/src/tip.rs`
- external reference: `https://github.com/utxorpc/rust-sdk`
- external reference: `https://github.com/txpipe/dolos`
- external references reviewed via GitHub README surfaces for SDK and Dolos deployment/runtime claims
- Rust skills consulted: `rust-router`, `coding-guidelines`, `m06-error-handling`, `m09-domain`, `m11-ecosystem`, `domain-fintech`

## Files Expected To Change

- `rust/crates/konduit-server/Cargo.toml`
- `rust/crates/konduit-server/src/cardano.rs`
- `rust/crates/konduit-server/src/cardano/args.rs`
- `rust/crates/konduit-server/src/env.rs`
- `rust/crates/konduit-cli/Cargo.toml`
- `rust/crates/konduit-cli/src/connector.rs`
- `rust/crates/konduit-cli/src/config/connector.rs`
- `rust/crates/konduit-cli/src/env/connector.rs`
- `rust/crates/konduit-cli/src/env/admin.rs`
- `rust/crates/konduit-cli/src/env/adaptor.rs`
- `rust/crates/konduit-cli/src/env/consumer.rs`
- `rust/crates/konduit-cli/src/cmd/admin/tx.rs`
- `rust/crates/konduit-cli/src/cmd/adaptor/tx.rs`
- `rust/crates/konduit-cli/src/tip.rs`

Possible no-change surfaces to verify during implementation:

- `rust/crates/konduit-server/src/admin/service.rs` if startup validation can stay fully inside server cardano bootstrap
- `rust/crates/konduit-cli/src/cmd/admin/show.rs`
- `rust/crates/konduit-cli/src/cmd/adaptor/show.rs`
- `rust/crates/konduit-cli/src/cmd/consumer/show.rs`
- `rust/crates/konduit-cli/src/cmd/consumer/tx.rs`

## Implementation Approach

1. Introduce a small runtime backend enum in each affected runtime boundary rather than widening the shared connector trait.

For `konduit-server`, replace the current `type Cardano = Blockfrost` alias with a small enum wrapper that implements `CardanoConnector` by delegating to `Blockfrost` or `cardano_connector_utxorpc::UtxoRpc`. For `konduit-cli`, extend the existing wrapper enum similarly instead of inventing a new abstraction layer.

2. Make backend selection explicit in server args and env names.

Add a backend-kind flag/env, keep Blockfrost project id support for the `blockfrost` path, and add UTxO RPC endpoint plus explicit Cardano network config for the `utxorpc` path. Validate required arguments per backend and return actionable configuration errors rather than silently guessing.

3. Keep server UTxO RPC readiness checks on the mandatory boot path, close to the runtime boundary.

`konduit-server` startup must fail for the UTxO RPC backend unless Dolos is reachable, the configured network matches live data, live protocol parameters can be derived, and the configured reference script UTxO can be resolved. The current boot path in `src/main.rs` already builds Cardano before constructing `admin::Service::new`, and the server does not start serving until `admin::Service::new(...).await?` succeeds. The implementation should make that boundary explicit and stable: perform backend-specific reachability and network-match validation in `cardano/args.rs`, then keep live-parameter and reference-script readiness in a mandatory startup helper or in `admin::Service::new` only if the boot path continues to require its success before `server.run()`.

4. Preserve explicit network semantics for UTxO RPC and existing inference only where Blockfrost already depends on it.

For Blockfrost, keep current project-id inference behavior unless the runtime surface now requires explicit backend-kind disambiguation. For UTxO RPC, require explicit network configuration and cross-check it against `connector.network()` immediately after building the connector.

5. Extend CLI connector config as a tagged runtime selection model shared by admin, adaptor, and consumer env flows.

Update `config/connector.rs` and `env/connector.rs` so config can represent either backend explicitly. Keep the existing `connector()` constructor and `network_id()` helper, but make them truthful for both backends. `fill()` should still support default-address generation and config display without fabricating a usable runtime connector for missing required backend fields.

6. Add an explicit CLI-side validation point for the UTxO RPC backend.

`konduit-cli` should fail clearly before tip or transaction flows proceed when the selected backend config is incomplete or when the configured UTxO RPC network does not match live data. The minimal likely shape is a shared connector-construction helper that builds the selected backend and, for `utxorpc`, runs a live network cross-check immediately after connection. Reference-script availability can remain a per-command runtime failure where those flows already resolve it through existing command or tip helpers; the plan does not require a CLI-wide startup phase, only truthful command-time validation.

7. Keep CLI flow changes minimal and construction-centric.

Most admin/adaptor/consumer command modules should continue calling `config.connector.connector()?`. The main code changes should be in connector construction and config parsing. Only touch command modules where a type signature or helper needs adjusting for the updated connector enum or network handling.

8. Keep diagnostics actionable and bounded.

Configuration or readiness failures should identify the selected backend and missing or mismatched requirement without leaking unnecessary internals. For UTxO RPC startup, errors should distinguish invalid config, reachability failure, network mismatch, protocol-parameter failure, and missing reference script UTxO.

9. Add small task-local tests for backend-selection and config truthfulness before broader phase-2 coverage.

Because this task changes the operator-facing selection boundary in both runtime crates, add the smallest focused tests now rather than deferring all coverage to `task-101`. At minimum, cover backend enum or config parsing, missing required per-backend fields, UTxO RPC network mismatch failure behavior at the shared validation point, and CLI fill behavior that may derive display defaults but must not fabricate a runnable connector configuration.

10. Avoid widening scope into live validation or broad test backfill, but run truthful targeted verification now.

Because `task-101` owns the broader automated coverage for these runtime surfaces, this task should still run build-oriented verification plus the small focused tests above, without pretending that live Dolos or operator deployment validation has already happened.

## Acceptance Criteria

- `konduit-server` supports explicit backend selection between `blockfrost` and `utxorpc`
- `konduit-cli` can select either `blockfrost` or `utxorpc`
- server and CLI config parsing support explicit UTxO RPC URI and network values
- current CLI runtime flows construct and validate the UTxO RPC backend truthfully, including clear failure on missing config or live network mismatch before tx or tip flows proceed
- the server fails startup when Dolos is unreachable, on the wrong network, missing live parameters, or missing the configured reference script UTxO
- the existing Blockfrost server and CLI paths remain available in parallel

## Verification Plan

Planned implementation-time verification:

- `cargo fmt --all -- --check`
- `cargo check -p konduit-server`
- `cargo check -p konduit-cli`
- `cargo test -p konduit-server`
- `cargo test -p konduit-cli`
- `cargo clippy -p konduit-server --all-targets -- -D warnings`
- `cargo clippy -p konduit-cli --all-targets -- -D warnings`

Required task-local coverage focus:

- server backend-selection and per-backend config validation
- CLI backend-selection and per-backend config validation
- UTxO RPC network-mismatch failure behavior at the shared runtime validation point
- CLI fill or default behavior proving display/address derivation does not imply a runnable connector when required fields are absent

Truthful broader verification if the touched code or dependency graph justifies it:

- `cargo check --workspace`
- `cargo doc -p konduit-server --no-deps`
- `cargo doc -p konduit-cli --no-deps`

Verification explicitly out of scope for this task:

- no claim that a live Dolos instance was used
- no claim that operator-managed localhost deployment or nginx topology was validated
- no claim that end-to-end live transaction submission has been exercised outside future manual validation work

## Risks / Open Questions

- dyn-compatibility limitation: the shared `CardanoConnector` trait currently returns `impl Future`, so runtime selection likely remains an enum wrapper rather than a trait object. Critique should stress-test whether that approach stays minimal across both runtime crates.
- startup check placement: the current server startup path already derives protocol parameters and resolves the reference script in `admin::Service::new`. The implementation must preserve the invariant that this readiness work happens before serving traffic, or centralize it in a single bootstrap helper to avoid drift.
- CLI config truthfulness: the current env fill path fabricates placeholder Blockfrost ids to derive addresses. The implementation must keep display and address-derivation convenience separate from runnable backend validation, especially for UTxO RPC where endpoint and network are distinct requirements.
- config drift risk: server and CLI must expose explicit backend selection and UTxO RPC network config without drifting on naming or semantics. Critique should stress-test whether one runtime could accidentally remain Blockfrost-defaulted while the other becomes explicit.
- acceptance wording vs autonomous verification: the CLI acceptance says flows can execute against UTxO RPC, but this task cannot truthfully prove live execution without Dolos. The implementation should satisfy this by wiring construction and command-path compatibility, while task docs and later validation work make the live-runtime gap explicit.

## Required Docs / Tracking / Research Updates

- append planning critique results to `.opencode/plans/dolos-utxorpc/task-plans/task-003-plan-review.md`
- during implementation, create `.opencode/plans/dolos-utxorpc/task-plans/task-003-impl-review.md` and record the full Implementation/Code Review transcript
- during implementation, write durable findings to `.opencode/plans/dolos-utxorpc/research/task-003.md`, including backend-selection shape, startup-check placement decisions, config/env naming decisions, and any truthfulness constraints discovered around CLI execution or server readiness
- update `.opencode/plans/dolos-utxorpc/dolos-utxorpc-tasks.json` when the task outcome is final
- update runtime-facing docs only if implementation materially changes documented config or readiness expectations earlier than `task-104`

## Implementation Progress

- implementation status: `iteration_2_complete`
- scope completed: added explicit `blockfrost` or `utxorpc` selection to the scoped server and CLI runtime surfaces, wired explicit UTxO RPC URI plus explicit network config, preserved Blockfrost in parallel, added shared live-network validation using Dolos genesis data, and kept server readiness on the mandatory boot path before `server.run()`
- review-driven follow-up completed: iteration 2 removed the accidental CLI implicit-mainnet fallback for `utxorpc`, restricted Blockfrost network inference to the Blockfrost backend only, and added focused regression tests for both cases
- durable research recorded at `.opencode/plans/dolos-utxorpc/research/task-003.md`

## Current Outcome

- current implementation outcome: `approved`
- implementation review log: `.opencode/plans/dolos-utxorpc/task-plans/task-003-impl-review.md`
- no user handoff is required for this task because all completed verification was agent-executable and no live Dolos operator checkpoint is claimed here

## Final Outcome

- final accepted outcome: `task-003` is complete and approved
- `konduit-server` now supports explicit `blockfrost` or `utxorpc` backend selection with UTxO RPC reachability and live-network validation at bootstrap, while protocol-parameter derivation and reference-script resolution remain startup blockers before traffic can be served
- `konduit-cli` now supports explicit `blockfrost` or `utxorpc` selection, requires explicit network plus URI for runnable UTxO RPC use, validates live UTxO RPC network before tip or tx flows proceed, and no longer allows stale Blockfrost env inference to rewrite UTxO RPC config state
- final review sources:
  - plan review: `.opencode/plans/dolos-utxorpc/task-plans/task-003-plan-review.md`
  - implementation review: `.opencode/plans/dolos-utxorpc/task-plans/task-003-impl-review.md`

## Verification Actually Run

- `cargo fmt --all -- --check`
- `cargo check -p konduit-server`
- `cargo check -p konduit-cli`
- `cargo test -p konduit-server`
- `cargo test -p konduit-cli`
- `cargo clippy -p konduit-server --all-targets -- -D warnings`
- `cargo clippy -p konduit-cli --all-targets -- -D warnings`

## Final Build Status

- status: `completed`
- meaning for this task: the touched server and CLI runtime surfaces compile cleanly, pass targeted clippy, and now carry focused tests covering backend selection, required per-backend config, network mismatch handling, and CLI fill truthfulness

## Plan Summary

Implement `task-003` by adding explicit `blockfrost`/`utxorpc` backend selection to `konduit-server` and `konduit-cli`, wiring UTxO RPC endpoint plus explicit network config into both runtime surfaces, preserving Blockfrost in parallel, enforcing server UTxO RPC readiness on the actual boot path before traffic can be served, and adding a shared CLI-side validation point plus small task-local tests so backend-selection truthfulness does not drift before later phase-2 coverage.

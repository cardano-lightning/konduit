# Task Plan: task-002 - Implement the UTxO RPC connector core

- task id: `task-002`
- title: `Implement the UTxO RPC connector core`
- planning status: `approved`
- build status: `completed`
- interaction mode: `autonomous`
- review-log paths:
- `.opencode/plans/dolos-utxorpc/task-plans/task-002-plan-review.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-002-impl-review.md`

## Why This Task Was Chosen Now

`task-002` is the next unlocked task on the critical path after `task-001` completed the workspace and crate skeleton. It must happen now because `task-003` backend selection work depends on a real `cardano-connector-utxorpc` implementation existing behind the current `CardanoConnector` trait.

## Interaction Mode

- mode: `autonomous`
- reason: the locked ADRs, PRDs, workflow docs, existing connector trait, and prior `task-001` research are sufficient to produce a truthful implementation plan without asking the user for a design choice, runtime confirmation, or manual operator action.

## Scope

This task is limited to the connector-core implementation inside `rust/crates/cardano-connector-utxorpc`.

In scope:

- implement UTxO RPC client construction with explicit Konduit network configuration
- implement the `CardanoConnector` surface required by current runtime code: `network`, `health`, `protocol_parameters`, `utxos_at`, and `submit`
- derive protocol parameters from UTxO RPC data rather than static `Network::{mainnet,preprod,preview}` presets
- map UTxO RPC responses into current `cardano-sdk` `Input`, `Output`, and related data structures
- implement `utxos_at(payment, None)` semantics as payment-credential matching regardless of delegation
- return actionable submission failures from the Dolos path

## Non-Goals

- no CLI or server backend-selection wiring in this task
- no `konduit-server` or `konduit-cli` startup/readiness enforcement in this task
- no repo-wide backend migration outside `cardano-connector-utxorpc`
- no fake claims about live Dolos validation, startup success, or end-to-end submission against a real deployment
- no fallback to Blockfrost-specific assumptions, local genesis files, `cardano-node` artifacts, or static per-network protocol-parameter presets for the UTxO RPC backend
- no connector-trait expansion unless implementation proves the current trait is insufficient

## Relevant Dependencies

- direct task dependency: `task-001`
- durable research output expected from this task: `.opencode/plans/dolos-utxorpc/research/task-002.md`
- downstream tasks unlocked by this task:
- `task-003`
- `task-100`
- trait boundary dependency: `rust/crates/cardano-connector/src/connector.rs`
- behavior reference for current connector expectations: `rust/crates/cardano-connector-direct/src/blockfrost.rs`
- external client dependency expected for implementation: `utxorpc` Rust SDK over `tonic`

## Research Consulted

- `.opencode/plans/dolos-utxorpc/research/task-001.md`

Key carry-forward findings from prior research:

- `task-001` intentionally stopped at crate wiring and placeholder code, so `task-002` is the first task allowed to add real connector behavior
- `rust/Cargo.toml` and the live workspace crate paths are more trustworthy than the older historical names in `rust/README.md`
- the workspace `rust-version = "1.94.0"` is canonical for this crate

## Docs, Crate Files, External References, And Skills Consulted

- `.opencode/plans/dolos-utxorpc/dolos-utxorpc-tasks.json`
- `.opencode/plans/dolos-utxorpc/research/task-001.md`
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
- `rust/crates/cardano-connector-direct/src/lib.rs`
- `rust/crates/cardano-connector-utxorpc/Cargo.toml`
- `rust/crates/cardano-connector-utxorpc/src/lib.rs`
- `rust/crates/cardano-sdk/src/cardano/network.rs`
- `rust/crates/cardano-sdk/src/cardano/input.rs`
- `rust/crates/cardano-sdk/src/cardano/output.rs`
- `rust/crates/cardano-sdk/src/cardano/protocol_parameters.rs`
- external reference: `https://github.com/utxorpc/rust-sdk`
- external reference: `https://github.com/txpipe/dolos`
- external SDK surface reviewed: `utxorpc/rust-sdk` `Cargo.toml`, `src/lib.rs`, and example listing via `gh api`
- Rust skills consulted: `rust-router`, `coding-guidelines`, `m11-ecosystem`, `m06-error-handling`, `m09-domain`, `m15-anti-pattern`, `domain-fintech`, `cardano-protocol-params`

## Files Expected To Change

- `rust/crates/cardano-connector-utxorpc/Cargo.toml`
- `rust/crates/cardano-connector-utxorpc/src/lib.rs`
- `rust/crates/cardano-connector-utxorpc/src/config.rs`
- `rust/crates/cardano-connector-utxorpc/src/mapping.rs`
- `rust/crates/cardano-connector-utxorpc/src/params.rs`

Likely no-change surfaces for this task:

- `rust/crates/cardano-connector/src/connector.rs`
- `rust/crates/cardano-connector-direct/src/blockfrost.rs`
- all CLI and server crates

## Implementation Approach

1. Replace the placeholder crate surface with a real connector type and a minimal config module.

Define a small `UtxoRpc` connector struct plus explicit config carrying at least the endpoint URI and configured `cardano_sdk::Network`. Keep provider-specific logic inside this crate and avoid leaking runtime-selection concerns into CLI or server code here.

2. Add only the crate dependencies needed for truthful UTxO RPC integration.

Expect to add `anyhow`, `tokio`, `utxorpc`, `thiserror` only if a typed local error layer is justified, and any narrow utility dependencies required by mapping. Do not add speculative framework layers or unrelated transport crates if the SDK already covers the gRPC surface.

3. Build UTxO RPC clients around the SDK, but keep startup assumptions explicit.

Use the current `utxorpc` SDK client builder directly for the documented localhost Dolos path, including `http://127.0.0.1` style endpoints supported by the reviewed SDK examples. Do not plan a custom transport fallback unless implementation proves a real SDK gap.

4. Implement `health()` as a meaningful reachability and correctness probe.

Use a lightweight UTxO RPC read path that proves Dolos is reachable and serving Cardano data, and include enough information in the returned string to aid operator debugging. Keep the method honest: it should not claim startup validation or reference-script readiness that belongs to later runtime work.

5. Implement live protocol-parameter derivation in `params.rs`.

Use UTxO RPC query modules to derive the fields Konduit actually needs for `cardano_sdk::ProtocolParameters`, including fee coefficients, execution prices, collateral percentage, reference-script fee inputs, and Plutus V3 cost model. Derive `start_time` and `first_shelley_slot` from UTxO RPC chain data such as era summary or genesis data rather than static per-network presets. If `cardano_sdk::ProtocolParameters` contains non-provider fields that are currently ledger-hard-wired rather than live protocol parameters, keep those defaults explicit and justified instead of smuggling in per-network presets.

6. Implement UTxO and output mapping in `mapping.rs`.

Prefer the most faithful mapping source available from the SDK. If parsed protobuf fields fully preserve addresses, value, datum, and reference-script data, map from them directly. If fidelity is incomplete, prefer decoding `native_bytes` into Cardano-native structures and then reuse existing `cardano-sdk` conversions such as `TryFrom<pallas::TransactionOutput> for Output` rather than inventing a partial duplicate mapping path.

7. Implement `utxos_at` semantics without Blockfrost-era shortcuts.

For `Some(delegation)`, match the exact payment and delegation pair. For `None`, ensure the query returns any UTxO whose address shares the same payment credential regardless of stake credential. Because `search_utxos` is paginated, the connector must page until exhaustion of `next_token`, or another exact stopping condition justified by the remote predicate shape, before applying any final local payment-credential filter. Local filtering must therefore stay bounded by an explicit remote predicate and complete pagination, not by an accidental first-page or open-ended scan approximation.

8. Implement `submit()` with actionable failures.

Serialize the ready transaction to CBOR, submit through Dolos, and attach endpoint and gRPC status context to failures so callers can distinguish transport errors, invalid requests, and backend rejection. Keep the success contract aligned with the existing trait: accepted submission is enough here unless richer stage tracking is required to avoid misleading results.

9. Keep trait fit under review, but do not widen scope prematurely.

If the current `CardanoConnector` trait proves insufficient for truthful network or submit behavior, capture that explicitly in task tracking and critique notes first. Do not hide a trait-shape problem behind silent approximation.

## Acceptance Criteria

- the new connector implements `network`, `health`, `protocol_parameters`, `utxos_at`, and `submit`
- protocol parameters are derived from UTxO RPC data rather than static per-network presets
- `utxos_at(payment, None)` returns UTxOs for any address sharing the payment credential regardless of delegation
- transaction submission works through Dolos and returns actionable failures

## Verification Plan

Planned implementation-time verification:

- `cargo fmt --all -- --check`
- `cargo check -p cardano-connector-utxorpc`
- `cargo test -p cardano-connector-utxorpc`
- `cargo clippy -p cardano-connector-utxorpc --all-targets -- -D warnings`
- `cargo doc -p cardano-connector-utxorpc --no-deps`

Truthful fallback or optional broader checks:

- `cargo check --workspace` if dependency changes or public exports affect the wider workspace
- targeted local tests for mapping and parameter derivation may be added during implementation, but comprehensive coverage remains the main purpose of `task-100`

Verification explicitly out of scope for this planning pass and this task alone:

- no claim that live Dolos validation has been run
- no claim that server or CLI startup-readiness rules have been exercised end to end
- no claim that real reference-script resolution has been validated outside later runtime tasks

## Risks / Open Questions

- payment-only UTxO lookup semantics: it is not yet proven from the reviewed SDK surface alone that the query predicate can express payment credential matching independent of delegation, so critique should stress-test the narrowest truthful remote predicate and the completeness of paging to exhaustion before local filtering.
- protocol-parameter completeness: `cardano_sdk::ProtocolParameters` needs `start_time` and `first_shelley_slot` in addition to live fee and cost data. Critique should stress-test which UTxO RPC modules provide these values truthfully.
- output mapping source of truth: critique should stress-test whether parsed protobuf output fields are sufficient or whether `native_bytes` decoding is the safer canonical mapping path for datums and reference scripts.
- submit success semantics: the current trait returns `Result<()>`, while UTxO RPC also exposes staged submission events. Critique should stress-test whether immediate `submit_tx` is truthful enough or whether some stage observation is required for actionable failure behavior.
- source conflict: `rust/README.md` still contains older crate names, so implementation should continue treating `rust/Cargo.toml` and actual crate paths as canonical.

## Required Docs / Tracking / Research Updates

- append critique results to `.opencode/plans/dolos-utxorpc/task-plans/task-002-plan-review.md`
- during implementation, write a durable research note at `.opencode/plans/dolos-utxorpc/research/task-002.md` capturing the concrete UTxO RPC and Dolos API findings that shaped the final connector behavior, including transport usage, pagination semantics, protocol-parameter derivation sources, mapping source-of-truth decisions, and submit behavior findings
- if implementation reveals the current `CardanoConnector` trait cannot express truthful UTxO RPC behavior, record that explicitly in task tracking before `task-003` starts
- do not update locked design docs during this planning-only step

## Implementation Progress

- implementation status: `iteration_1_complete`
- scope completed: replaced the placeholder crate with a real `UtxoRpc` connector, added explicit config plus mapping and params modules, implemented `network`, `health`, `protocol_parameters`, `utxos_at`, and `submit`, and kept all work inside `cardano-connector-utxorpc`
- build verification passed:
  - `cargo check -p cardano-connector-utxorpc`
  - `cargo test -p cardano-connector-utxorpc`
  - `cargo doc -p cardano-connector-utxorpc --no-deps`
  - `cargo fmt --all -- --check`
  - `cargo clippy -p cardano-connector-utxorpc --all-targets -- -D warnings`
- review-driven follow-up completed: iteration 2 fixed `BigUInt` overflow truncation, strengthened health-check endpoint diagnostics, and added 5 focused connector-core unit tests before the final code-review pass
- durable research recorded at `.opencode/plans/dolos-utxorpc/research/task-002.md`

## Current Outcome

- current implementation outcome: `approved`
- implementation review log: `.opencode/plans/dolos-utxorpc/task-plans/task-002-impl-review.md`
- no user handoff is currently required for this task because all verification run so far is agent-executable and no live Dolos operator checkpoint is claimed here

## Final Outcome

- final accepted outcome: `task-002` is complete and approved
- the connector core now provides explicit UTxO RPC endpoint/network configuration, live health and protocol-parameter reads, paginated `utxos_at` lookup with payment-only semantics, and Dolos-backed transaction submission inside `cardano-connector-utxorpc`
- Blockfrost remains available in parallel because no existing direct connector, CLI, or server runtime surfaces were changed in this task
- final review sources:
  - plan review: `.opencode/plans/dolos-utxorpc/task-plans/task-002-plan-review.md`
  - implementation review: `.opencode/plans/dolos-utxorpc/task-plans/task-002-impl-review.md`

## Verification Actually Run

- `cargo fmt --all -- --check`
- `cargo check -p cardano-connector-utxorpc`
- `cargo test -p cardano-connector-utxorpc`
- `cargo clippy -p cardano-connector-utxorpc --all-targets -- -D warnings`
- `cargo doc -p cardano-connector-utxorpc --no-deps`

## Final Build Status

- status: `completed`
- meaning for this task: the connector-core crate compiled, passed clippy and docs build, and now carries 5 focused unit tests covering the highest-risk mapping and parameter-helper behavior landed in this task

## Plan Summary

Implement `task-002` entirely inside `cardano-connector-utxorpc`: add the real Dolos UTxO RPC connector, derive live protocol parameters from UTxO RPC modules, map UTxO data into `cardano-sdk` types with truthful handling of datum and script fields, preserve the documented `utxos_at` semantics, and return actionable submit errors without drifting into CLI/server wiring or fake live-validation claims.

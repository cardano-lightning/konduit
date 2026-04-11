Implementation: Iteration 1
Timestamp: 2026-04-11T23:58:00Z
Outcome: completed

Changes made:
- replaced the hard-wired Blockfrost-only server runtime binding with a small `Cardano` enum wrapper that delegates the `CardanoConnector` trait to either `Blockfrost` or `UtxoRpc`
- extended `konduit-server` cardano args and env surface with explicit backend selection, `KONDUIT_UTXORPC_URI`, and explicit `KONDUIT_NETWORK`, while preserving Blockfrost in parallel
- added server-side UTxO RPC startup validation for reachability and live network match in `cardano/args.rs`, while keeping protocol-parameter and reference-script readiness on the mandatory boot path through `admin::Service::new(...).await?` before `server.run()`
- added a small live-network helper to `cardano-connector-utxorpc` that derives the network from Dolos genesis data and exposes a shared `ensure_network_matches` helper used by both server and CLI runtime validation
- refactored CLI connector config into explicit backend-tagged `Blockfrost` or `UtxoRpc` config variants with truthful explicit network storage for both paths
- replaced the CLI `None` connector branch with real dual-backend construction and validation, including UTxO RPC reachability plus live network cross-check before tip or tx flows proceed
- adjusted CLI `show address` commands to derive addresses from config network directly so config display and address derivation remain usable without fabricating a runnable connector configuration
- added focused task-local tests for backend selection requirements, missing per-backend config, network mismatch handling, and CLI fill behavior that leaves missing runtime-only fields unset

Files touched:
- `rust/crates/cardano-connector-utxorpc/src/lib.rs`
- `rust/crates/konduit-server/Cargo.toml`
- `rust/crates/konduit-server/src/cardano.rs`
- `rust/crates/konduit-server/src/cardano/args.rs`
- `rust/crates/konduit-server/src/env.rs`
- `rust/crates/konduit-cli/Cargo.toml`
- `rust/crates/konduit-cli/src/connector.rs`
- `rust/crates/konduit-cli/src/config/connector.rs`
- `rust/crates/konduit-cli/src/env/connector.rs`
- `rust/crates/konduit-cli/src/cmd/admin/show.rs`
- `rust/crates/konduit-cli/src/cmd/adaptor/show.rs`
- `rust/crates/konduit-cli/src/cmd/consumer/show.rs`

Verification run:
- `cargo fmt --all -- --check` ✅
- `cargo check -p konduit-server` ✅
- `cargo check -p konduit-cli` ✅
- `cargo test -p konduit-server` ✅ (`3 passed`)
- `cargo test -p konduit-cli` ✅ (`7 passed`)
- `cargo clippy -p konduit-server --all-targets -- -D warnings` ✅
- `cargo clippy -p konduit-cli --all-targets -- -D warnings` ✅

Deviations from the approved plan:
- the CLI `show address` paths now derive addresses from explicit config network instead of constructing a live connector first; this keeps config-display and address-generation flows truthful when runtime-only backend fields are absent while still requiring connector validation for tip and tx flows
- the shared live-network check was added to `cardano-connector-utxorpc` rather than duplicated in server and CLI runtime crates, because the existing `utxorpc` helper only exposed raw genesis bytes and the smallest clean solution was to centralize parsed genesis-based network mapping once

User interaction required now: no

Code Review: Iteration 2
Timestamp: 2026-04-11T23:40:29Z
Outcome: approved

Findings:
- none

Verification notes:
- Reviewed the required task, design, ADR, and scoped Rust runtime files with focus on the prior iteration-1 findings.
- Confirmed `rust/crates/konduit-cli/src/env/connector.rs:49-53`, `rust/crates/konduit-cli/src/env/connector.rs:88-109`, and `rust/crates/konduit-cli/src/config/connector.rs:94-104` now keep UTxO RPC network selection explicit and reject runnable `utxorpc` config when `KONDUIT_NETWORK` is missing.
- Confirmed stale Blockfrost env drift is no longer applied to the UTxO RPC path because Blockfrost network inference is scoped to the `blockfrost` branch in `rust/crates/konduit-cli/src/env/connector.rs:66-97`.
- Confirmed regression coverage for both prior findings in `rust/crates/konduit-cli/src/env/connector.rs:176-205`, and the implementation log records fresh `cargo check`, `cargo test`, and `cargo clippy` passes for both `konduit-cli` and `konduit-server`.

Decision: approved

Code Review: Iteration 1
Timestamp: 2026-04-11T23:37:01Z
Outcome: requires_changes

Findings:
- high: `rust/crates/konduit-cli/src/env/connector.rs:22-24`, `rust/crates/konduit-cli/src/config/connector.rs:95-98`, and `rust/crates/konduit-cli/src/connector.rs:51-63` no longer enforce an explicit Cardano network for the `utxorpc` backend. `ConnectorEnv.network` always defaults to `Network::Mainnet`, and `UtxoRpc` stores that non-optional value, so `KONDUIT_CARDANO_BACKEND=utxorpc` plus only `KONDUIT_UTXORPC_URI` produces a runnable config instead of failing as incomplete. The approved plan and task acceptance require explicit UTxO RPC network selection and truthful refusal to fabricate runnable config from defaults.
- medium: `rust/crates/konduit-cli/src/env/connector.rs:55-80` still applies Blockfrost project-id inference even when `backend=utxorpc`. If an operator keeps an old `KONDUIT_BLOCKFROST_PROJECT_ID` in the environment while switching to UTxO RPC, `fill()` rewrites the configured network from the stale Blockfrost value and `setup` emits the wrong network/address pair. That is cross-backend config drift in the exact area task-003 was supposed to make explicit.

Verification notes:
- Reviewed the task plan, implementation log, scoped docs, affected runtime surfaces, and the live git diff for task-003.
- Did not rerun the recorded cargo verification commands during review.
- The existing added tests do not cover the two regressions above: there is no CLI test proving `utxorpc` rejects a missing explicit network, and no test proving stale Blockfrost variables are ignored when the selected backend is `utxorpc`.

Decision: requires_changes

Implementation: Iteration 2
Timestamp: 2026-04-12T00:06:00Z
Outcome: completed

Changes made:
- changed CLI connector env parsing so `utxorpc` requires an explicit `KONDUIT_NETWORK` instead of inheriting the old `mainnet` default silently
- restricted Blockfrost project-id network inference to the `blockfrost` backend only, so stale Blockfrost env values no longer rewrite UTxO RPC network selection during `fill()` or `setup`
- updated CLI env helpers to treat network selection as optional at parse time but required for runnable UTxO RPC config, while keeping Blockfrost setup/display defaults usable
- added focused regression tests covering missing explicit UTxO RPC network and stale Blockfrost env values being ignored for the selected UTxO RPC backend

Files touched:
- `rust/crates/konduit-cli/src/env/connector.rs`
- `rust/crates/konduit-cli/src/env/admin.rs`
- `rust/crates/konduit-cli/src/env/adaptor.rs`
- `rust/crates/konduit-cli/src/env/consumer.rs`

Verification run:
- `cargo fmt --all -- --check` ✅
- `cargo check -p konduit-cli` ✅
- `cargo test -p konduit-cli` ✅ (`9 passed`)
- `cargo clippy -p konduit-cli --all-targets -- -D warnings` ✅
- `cargo check -p konduit-server` ✅
- `cargo test -p konduit-server` ✅ (`3 passed`)
- `cargo clippy -p konduit-server --all-targets -- -D warnings` ✅

Deviations from the approved plan:
- none beyond the iteration-1 implementation notes; this iteration only tightened CLI explicit-network truthfulness and added the missing task-local regression coverage required by review

User interaction required now: no

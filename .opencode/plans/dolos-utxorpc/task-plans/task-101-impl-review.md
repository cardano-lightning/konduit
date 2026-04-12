Implementation: Iteration 1
Timestamp: 2026-04-12T00:20:00Z
Outcome: completed

Changes made:
- added focused `konduit-server/src/cardano/args.rs` tests for explicit backend identity, preserved UTxO RPC config values, trimmed Blockfrost project ids, and the missing-network failure after URI selection
- added `konduit-server/src/admin/service.rs` startup-blocker tests using a narrow in-test fake connector plus mock BLN/DB dependencies to cover protocol-parameter failure and missing reference-script failure on the real `Service::new(...)` boot path
- added CLI regression tests in `konduit-cli/src/config/connector.rs`, `konduit-cli/src/env/connector.rs`, and `konduit-cli/src/env/admin.rs` for explicit backend display, network or network-id behavior, Blockfrost inference, and representative higher-level `fill()` plus `connector.network_id()` address derivation without UTxO RPC inheriting Blockfrost defaults
- extracted a tiny `validated_blockfrost_project_id` helper in `konduit-cli/src/connector.rs` so Blockfrost runnable-config validation can be tested without introducing a broader connector abstraction or live runtime path

Files touched:
- `rust/crates/konduit-server/src/cardano/args.rs`
- `rust/crates/konduit-server/src/admin/service.rs`
- `rust/crates/konduit-cli/src/connector.rs`
- `rust/crates/konduit-cli/src/config/connector.rs`
- `rust/crates/konduit-cli/src/env/connector.rs`
- `rust/crates/konduit-cli/src/env/admin.rs`

Verification run:
- `cargo fmt --all` ✅
- `cargo fmt --all -- --check` ✅
- `cargo check -p konduit-cli` ✅
- `cargo test -p konduit-cli` ✅ (`16 passed`)
- `cargo clippy -p konduit-cli --all-targets -- -D warnings` ✅
- `cargo test -p konduit-server --lib` ✅ (`9 passed`)
- `cargo clippy -p konduit-server --lib -- -D warnings` ✅
- `cargo check -p konduit-server --lib --tests` ⚠️ blocked by an existing repo-level `Send` failure in `src/main.rs` when the binary target is built with the current UTxO RPC admin-sync spawn path; the touched library or test surfaces compile and pass
- `cargo clippy -p konduit-server --lib --tests -- -D warnings` ⚠️ same existing binary-target `Send` failure when Cargo includes test binaries; narrowed truthful fallback was `cargo clippy -p konduit-server --lib -- -D warnings`

Deviations from the approved plan:
- none on scope or behavior; the only code-shape adjustment was the tiny `validated_blockfrost_project_id` extraction to make the shared Blockfrost runnable-config validation path testable without live runtime construction

User interaction required now: no

Code Review: Iteration 1
Timestamp: 2026-04-12T00:25:22Z
Outcome: approved

Findings:
- No blocking findings. The implementation matches the approved minimal plan: `konduit-server/src/cardano/args.rs` now covers both backend-selection and backend-specific config validation, `konduit-server/src/admin/service.rs` adds the required non-live startup-blocker tests on the real `Service::new(...)` path, and the CLI changes cover config parsing, env fill truthfulness, one real higher-level env consumer boundary, and Blockfrost regression checks without widening crate boundaries.
- The small `validated_blockfrost_project_id` extraction in `rust/crates/konduit-cli/src/connector.rs` is acceptable for this task. It keeps the only nontrivial Blockfrost runnable-config validation in one place, makes that validation directly testable, and does not introduce the broader abstraction drift the canonical plan explicitly avoided.
- The narrowed `konduit-server` verification fallback is acceptable here. Re-running `cargo check -p konduit-server --lib --tests` and `cargo clippy -p konduit-server --lib --tests -- -D warnings` still fails on the existing binary-target `Send` issue at `rust/crates/konduit-server/src/main.rs:63`, where `tokio::spawn` requires a `Send` future but the current UTxO RPC admin sync path is not `Send`. That blocker is recorded truthfully rather than being misrepresented as task-local fallout, and the touched library or test surfaces still compile and pass.

Verification notes:
- Reviewed the live diff for the six touched Rust files against the approved `task-101` plan and confirmed no scope drift, no doc drift requirement for this task, and no unnecessary production abstraction beyond the narrow CLI helper extraction.
- Re-ran `cargo test -p konduit-cli` and `cargo test -p konduit-server --lib`; both pass with the expected new coverage (`16 passed` and `9 passed`).
- Re-ran `cargo check -p konduit-server --lib --tests` and `cargo clippy -p konduit-server --lib --tests -- -D warnings`; both reproduce the same pre-existing binary-target `Send` failure in `src/main.rs`, confirming the implementation log handled that limitation honestly.

Decision: approved

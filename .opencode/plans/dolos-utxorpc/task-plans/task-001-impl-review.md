Implementation: Iteration 1
Timestamp: 2026-04-11T19:51:18Z
Changes made: Added `cardano-connector-utxorpc` to the Rust workspace, created the new crate manifest with minimal inherited workspace metadata and connector-facing dependencies, added a compile-safe placeholder `src/lib.rs`, and updated the canonical task plan doc to reflect approved planning state and successful implementation verification.
Files touched:
- `rust/Cargo.toml`
- `rust/crates/cardano-connector-utxorpc/Cargo.toml`
- `rust/crates/cardano-connector-utxorpc/src/lib.rs`
- `.opencode/plans/dolos-utxorpc/task-plans/task-001.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-001-impl-review.md`
Verification run:
- `cargo check -p cardano-connector-utxorpc`
- `cargo check --workspace`
- `cargo test -p cardano-connector-utxorpc`
Deviations from the approved plan: None.
User interaction required: No.

Code Review: Iteration 1
Timestamp: 2026-04-11T19:53:05Z
Outcome: Approved. The implementation stays within the approved `task-001` scope: `rust/Cargo.toml` adds the new workspace member and workspace dependency alias, `rust/crates/cardano-connector-utxorpc/Cargo.toml` mirrors the existing workspace-inherited crate metadata style with only the minimal connector-facing dependencies, and `rust/crates/cardano-connector-utxorpc/src/lib.rs` provides a truthful placeholder type rather than a fake `CardanoConnector` implementation. `cardano-connector-direct` and the Blockfrost path remain untouched, so the existing backend remains available in parallel. I also re-ran the claimed verification successfully with `cargo check -p cardano-connector-utxorpc`, `cargo test -p cardano-connector-utxorpc`, and `cargo check --workspace`. One non-blocking tracking note: the actual task diff also updates `rust/Cargo.lock` to register the new crate, but that file was omitted from the implementation entry's `Files touched` list.
Decision: approved

Planner: Iteration 1 Timestamp: 2026-04-11T19:47:19Z Outcome: Created the
initial planning draft for `task-001` with autonomous interaction mode,
task-graph justification, strict scope limited to workspace/crate wiring,
expected file changes, verification commands, and documented source conflicts to
preserve critique visibility.

Plan summary for critique: Add a new `cardano-connector-utxorpc` workspace crate
beside `cardano-connector-direct`, limit changes to `rust/Cargo.toml`, the new
crate manifest, and `src/lib.rs`, avoid implementing any UTxO RPC behavior or
runtime wiring yet, and verify the new crate builds without disturbing existing
Blockfrost support.

Critiquer: Iteration 1 Timestamp: 2026-04-11T19:49:05Z Outcome: Approved. The
plan stays within the intended crate boundary for `task-001`, keeps CLI/server
selection and startup-readiness work deferred to later tasks, and correctly
avoids pulling UTxO RPC client assumptions or Blockfrost-era trait changes into
the crate-skeleton step. The direct connector remains parallel by design because
`cardano-connector-utxorpc` is introduced beside `cardano-connector-direct` and
the verification plan includes `cargo check --workspace`, which exercises the
existing Blockfrost crate in the updated workspace. One small improvement would
be to treat `cargo test -p cardano-connector-utxorpc` as opportunistic rather
than required if the initial skeleton contains no tests, but that is a
verification-shape detail, not a scope defect. No additional docs or research
updates are required for this task beyond append-only planning logs because the
task does not change runtime behavior, configuration, or dependency policy yet.
Decision: approved

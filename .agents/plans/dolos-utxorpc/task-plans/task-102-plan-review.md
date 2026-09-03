Planner: Iteration 1 Timestamp: 2026-04-12T00:30:31Z Outcome: Created the
initial planning draft for `task-102` with autonomous interaction mode, scope
limited to minimal integrated UTxO RPC smoke coverage plus truthful
touched-surface verification, and an explicit plan to absorb the current
`konduit-server` binary `Send` failure if it blocks crate-level verification.

Plan summary for critique: Add one minimal CLI runtime smoke path and one
minimal server runtime smoke path for the UTxO RPC integration using narrow
in-test fakes, keep the coverage autonomous and repeatable, and fix the current
`konduit-server` admin-sync `Send` mismatch if it still prevents
`cargo check/test/clippy -p konduit-server` from passing truthfully for this
task.

Planner: Iteration 2 Timestamp: 2026-04-12T00:33:29Z Outcome: Revised the
`task-102` plan to require a tx-oriented CLI smoke path that proves
protocol-parameter and submit wiring, to require broader
`cargo check --workspace` and `cargo test --workspace` after targeted
verification, and to constrain any `Send` fix to the `konduit-server` runtime
boundary rather than broader trait or crate refactors.

Plan summary for critique: Keep `task-102` autonomous and minimal, but require
one tx-command CLI smoke test plus one server runtime smoke test, absorb the
current `konduit-server` `Send` blocker only through a small `main.rs` or
`admin.rs` runtime-boundary repair if needed, and follow targeted crate
verification with workspace-level `cargo check --workspace` and
`cargo test --workspace` when the touched runtime path builds truthfully.

Critiquer: Iteration 2 Timestamp: 2026-04-12T00:49:00Z Outcome: Reviewed the
revised `task-102` plan against the task spec, design docs, research notes, and
current Rust runtime boundaries. The previous blockers are addressed: the CLI
smoke seam is now explicitly tx-oriented, broader `cargo check --workspace` and
`cargo test --workspace` are required, and any `Send` repair is constrained to
the `konduit-server` runtime boundary rather than broader trait or crate
refactors.

Findings:

- No blocking findings. The plan is now minimal, truthful, and aligned with the
  current crate boundaries and design docs. It keeps autonomous smoke coverage
  inside the scoped Rust runtime surfaces, preserves the documented Dolos
  readiness model, avoids abstraction bloat, and only absorbs the existing
  `konduit-server` `Send` blocker to the extent needed for honest verification.

Residual watchpoints:

- During implementation, keep the server smoke focused on
  `admin::Service::new(...)` plus only as much `sync()` coverage as needed to
  prove integrated submission wiring; broader BLN or DB harness expansion would
  be unnecessary scope growth.
- If `cargo check --workspace` or `cargo test --workspace` still fail after the
  bounded runtime-boundary fix, the implementation notes must distinguish
  touched-surface failures from unrelated workspace breakage exactly as required
  by `.opencode/workflows/rust.md`.

Decision: approved

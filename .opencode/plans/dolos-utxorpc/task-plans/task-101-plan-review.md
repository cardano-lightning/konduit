Planner: Iteration 1 Timestamp: 2026-04-12T00:00:00Z Outcome: Created the
initial planning draft for `task-101` with autonomous interaction mode, scope
limited to automated regression coverage for the runtime surfaces changed by
`task-003`, and a minimal-refactor approach centered on truthful server and CLI
validation seams rather than live Dolos simulation.

Plan summary for critique: Add focused tests in `konduit-server` and
`konduit-cli` covering explicit `blockfrost` or `utxorpc` selection, per-backend
config truthfulness, pre-runtime validation failures that are reachable without
live Dolos, and Blockfrost regression protection, while keeping any testability
refactors small and local to the runtime modules already changed by `task-003`.

Critiquer: Iteration 1 Timestamp: 2026-04-12T00:11:25Z Outcome: requires_changes

Findings:

- The plan understates the server-side readiness surface by treating
  `rust/crates/konduit-server/src/admin/service.rs` as a likely no-change file
  and by explicitly declining to cover the startup blockers that actually fail
  server boot after `CardanoArgs::build()` succeeds. The reviewed runtime shape
  still awaits `admin::Service::new(...).await?` before serving traffic, and
  that constructor is where live protocol-parameter fetch failure and missing
  reference-script UTxO resolution are enforced. Without at least unit-level
  coverage for that boundary using a narrow fake `CardanoConnector`, `task-101`
  would miss two documented startup blockers from the ADR and PRDs and leave the
  most regression-prone server readiness path unprotected until later smoke or
  manual tasks.

Required changes:

- Expand the task scope and file expectations to include focused
  `konduit-server/src/admin/service.rs` tests that exercise the non-live startup
  blockers reachable without Dolos: failure when `protocol_parameters()` returns
  an error and failure when `utxos_at(host payment, host delegation)` cannot
  resolve the configured reference script. Keep this as minimal unit coverage
  with a tiny in-test fake connector rather than a broader runtime abstraction,
  and update the implementation approach plus verification narrative so
  `task-101` explicitly protects those server readiness rules while still
  deferring live Dolos reachability and network checks to the existing
  `cardano/args.rs` coverage and later integrated/manual tasks.

Decision: requires_changes

Planner: Iteration 2 Timestamp: 2026-04-12T00:12:30Z Outcome: Revised the
`task-101` plan to include focused `konduit-server/src/admin/service.rs`
startup-blocker tests, updated file expectations accordingly, and clarified that
the task will cover non-live readiness failures at both the server bootstrap
boundary and the CLI/runtime config boundary without pretending to replace live
Dolos validation.

Plan summary for critique: Keep `task-101` focused on runtime-surface regression
coverage, but require tests in `konduit-server/src/admin/service.rs` for
protocol-parameter and reference-script startup failures alongside the existing
server and CLI backend-selection, config-truthfulness, pre-runtime validation,
and Blockfrost parity coverage, using only narrow runtime-local test seams.

Critiquer: Iteration 2 Timestamp: 2026-04-12T00:13:36Z Outcome: requires_changes

Findings:

- The revised plan now covers the missing server startup blockers, but it still
  leaves a CLI drift gap around the actual consumers of
  `ConnectorEnv::network_id()` and `fill()`. The current plan talks about
  preserving display and address-derivation behavior, yet its concrete file list
  and implementation steps only guarantee tests in
  `konduit-cli/src/config/connector.rs`, `konduit-cli/src/env/connector.rs`, and
  `konduit-cli/src/connector.rs`. The reviewed CLI shape uses
  `connector.network_id()?` inside `rust/crates/konduit-cli/src/env/admin.rs`,
  `env/adaptor.rs`, and `env/consumer.rs` to derive default wallet and host
  addresses, so a regression where `utxorpc` incorrectly defaults, or where
  stale Blockfrost state leaks into higher-level fill behavior, would slip past
  the currently planned tests. That would directly undermine the task-003
  finding that UTxO RPC must keep explicit-network requirements distinct from
  Blockfrost inference in runnable CLI flows.

Required changes:

- Expand the plan just enough to require at least one focused higher-level CLI
  env test at the real consumer boundary, preferably in
  `rust/crates/konduit-cli/src/env/admin.rs` or another representative `env/*`
  module, proving that `fill()` plus `connector.network_id()` still behave
  correctly for both backends where default wallet or host-address derivation is
  used. Keep this minimal: one representative regression test is enough if it
  exercises the shared behavior and explicitly protects against UTxO RPC
  accidentally inheriting Blockfrost defaults.

Decision: requires_changes

Planner: Iteration 3 Timestamp: 2026-04-12T00:14:40Z Outcome: Revised the
`task-101` plan to require one representative higher-level CLI env-consumer
regression test at the real `fill()` plus `connector.network_id()` call
boundary, updated the expected file list to include
`konduit-cli/src/env/admin.rs`, and clarified that this coverage exists to catch
UTxO RPC drift from Blockfrost-only defaults without duplicating every env
module.

Plan summary for critique: Keep `task-101` tightly scoped, but require focused
tests in `konduit-server/src/cardano/args.rs`,
`konduit-server/src/admin/service.rs`, `konduit-cli/src/config/connector.rs`,
`konduit-cli/src/env/connector.rs`, one representative CLI env consumer such as
`konduit-cli/src/env/admin.rs`, and `konduit-cli/src/connector.rs` so backend
selection, non-live readiness blockers, higher-level CLI default derivation,
runnable-config validation, and Blockfrost parity are all protected with minimal
runtime-local seams.

Critiquer: Iteration 3 Timestamp: 2026-04-12T00:15:22Z Outcome: approved

Findings:

- The revised plan now covers the full non-live runtime surface that the current
  code and docs actually make regression-sensitive for `task-101`: server
  reachability and live-network validation in
  `konduit-server/src/cardano/args.rs`, startup-time protocol-parameter and
  reference-script blockers in `konduit-server/src/admin/service.rs`,
  backend-selection and config truthfulness in the CLI connector and env layers,
  and one real higher-level CLI env consumer boundary where `fill()` plus
  `connector.network_id()` can drift between Blockfrost and UTxO RPC behavior.
- The plan stays within the documented crate boundaries and avoids the two main
  review risks from this code shape: inventing a broader abstraction just for
  tests, or pretending this task can replace later live Dolos smoke and operator
  validation. Its verification scope is appropriately targeted to the touched
  runtime crates, and the required research/tracking updates are explicit.

Decision: approved

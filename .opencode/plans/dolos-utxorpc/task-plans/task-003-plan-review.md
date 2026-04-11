Planner: Iteration 1
Timestamp: 2026-04-11T23:10:00Z
Outcome: Created the initial planning draft for `task-003` with autonomous interaction mode, scope limited to `konduit-server`, `konduit-cli`, and the already-in-scope shared connector layer, an enum-wrapper backend-selection approach for both runtimes, explicit UTxO RPC endpoint and network config, and a startup-readiness plan that keeps Dolos reachability and network-match validation near server bootstrap while relying on existing admin-service startup flow for live protocol-parameter and reference-script resolution.

Plan summary for critique: Replace the current hard-wired Blockfrost runtime bindings with explicit `blockfrost` or `utxorpc` selection in `konduit-server` and `konduit-cli`, add truthful UTxO RPC URI plus network config surfaces, preserve existing Blockfrost behavior in parallel, keep runtime selection minimal through small enum wrappers rather than trait redesign, and make server startup fail for the UTxO RPC backend when reachability, network, live-parameter, or reference-script requirements are not satisfied.

Critiquer: Iteration 1
Timestamp: 2026-04-11T23:20:09Z
Outcome: requires_changes

Findings:
- The CLI side is still under-specified for UTxO RPC readiness and network truthfulness. The plan says current CLI runtime flows should execute against either backend, but the concrete approach only guarantees construction parity and explicitly concentrates readiness checks on the server. That leaves a real drift risk for `konduit-cli` because its current flows in `src/tip.rs`, `src/cmd/admin/tx.rs`, and `src/cmd/adaptor/tx.rs` derive addresses and build transactions from `connector.network()` and `protocol_parameters()` without a stated central check that the configured UTxO RPC network matches live Dolos data before those flows run. The PRD for Workstream 4b requires CLI runtime flows to fail clearly when startup-equivalent backend validation cannot be satisfied, so the task plan needs an explicit CLI validation point for at least backend-specific config completeness and live network mismatch handling.
- The server startup-blocker story is still too implicit. Relying on `admin::Service::new` for live protocol-parameter derivation and reference-script resolution is reasonable, but the plan currently treats that as an assumption rather than a verified invariant of the server bootstrap path. Given the acceptance criterion says server startup must fail when live parameters or the configured reference script UTxO are unavailable, the plan needs to state that `konduit-server` will not start serving traffic before `admin::Service::new` succeeds for the selected backend, or else move those checks into a shared bootstrap helper. Without that, the plan leaves a gap where reachability and network mismatch are checked in `cardano/args.rs` but deeper Dolos readiness could drift behind the actual ready-to-serve boundary.
- The verification section is too light for a task that changes operator-facing config parsing in both runtime crates. Deferring all real coverage to `task-101` is risky here because this task changes `konduit-server` args/env parsing and `konduit-cli` tagged connector/env fill behavior at the exact boundary where Blockfrost parity, UTxO RPC explicitness, and placeholder/default-generation truthfulness can easily drift. The current CLI env code already fabricates a placeholder Blockfrost id for `fill()`, and the plan discusses that risk but does not elevate it into a required implementation check. This task should require at least small focused tests now for backend selection parsing, invalid/missing per-backend config, UTxO RPC network mismatch handling, and CLI fill behavior that must not fabricate a runnable connector configuration.

Required changes:
- Add an explicit CLI-side validation requirement to the implementation approach and acceptance notes: define where `konduit-cli` validates selected backend config, and for UTxO RPC require a live network cross-check before tx/tip flows proceed. Also define whether reference-script lookup failures remain per-command runtime errors or are surfaced through a shared readiness helper.
- Tighten the server readiness section so it explicitly proves the startup boundary. Either document that `admin::Service::new` is on the mandatory boot path before serving traffic, or move protocol-parameter and reference-script readiness into a single bootstrap helper used by startup so the acceptance criterion is mechanically satisfied.
- Strengthen the task-local verification plan with minimal targeted tests for server/CLI backend selection and config truthfulness instead of relying entirely on later test tasks. At minimum cover enum/config parsing, missing required UTxO RPC fields, network mismatch failure behavior, and non-runnable placeholder/default fill semantics.

Decision: requires_changes

Planner: Iteration 2
Timestamp: 2026-04-11T23:24:30Z
Outcome: Revised the `task-003` plan to make CLI UTxO RPC validation explicit, tie server readiness to the actual startup boundary before `server.run()`, and require a small set of task-local backend-selection and config-truthfulness tests instead of deferring all safety to `task-101`.

Plan summary for critique: Keep the enum-wrapper backend-selection approach, but explicitly require `konduit-cli` to validate selected backend config and cross-check live UTxO RPC network data before tip or tx flows proceed, preserve the invariant that server protocol-parameter and reference-script readiness is enforced on the mandatory boot path before serving traffic, and add minimal server/CLI tests now for backend selection, missing per-backend config, network mismatch failure, and non-runnable CLI default-fill semantics.

Critiquer: Iteration 2
Timestamp: 2026-04-11T23:25:00Z
Outcome: approved

Findings:
- The revised CLI section now closes the main truthfulness gap from iteration 1. It no longer treats UTxO RPC support as mere construction parity; it explicitly requires a shared validation point that checks backend-specific completeness and cross-checks configured network versus live Dolos data before `tip` or tx flows proceed. That matches the Workstream 4b requirement in `docs/design/36_dolos_utxorpc_implementation_prd.md` that runtime flows fail clearly when startup-equivalent backend validation cannot be satisfied.
- The server readiness story is now mechanically tied to the real boot boundary. The plan explicitly calls out that `src/main.rs` builds Cardano and awaits `admin::Service::new(...).await?` before `server.run()`, and it requires either preserving that invariant or centralizing the checks in one bootstrap helper. That is specific enough to satisfy the ADR and design-doc requirement that Dolos reachability, network match, live protocol parameters, and reference-script resolution remain startup blockers rather than post-start drift.
- The verification plan is now appropriately minimal and local to this task. Requiring focused tests for backend selection, per-backend missing config, UTxO RPC network mismatch at the shared validation point, and CLI fill semantics is enough to protect the operator-facing selection boundary now without collapsing task scope into the broader coverage already reserved for `task-101` and later integration validation work.

Decision: approved

Implementation: Iteration 1
Timestamp: 2026-04-12T17:28:09Z
Outcome: Updated the task-104 target docs and tracking artifacts with a strict truth-first pass that documents the implemented UTxO RPC path, the actual CLI/server config surfaces, the live Dolos `read_genesis` prerequisite, the UTxO RPC delegated-lookup watchpoint, and the remaining direct Blockfrost non-parity points instead of smoothing them over.

Changes made:
- updated `docs/adrs/06-dolos-utxorpc-adaptor-backend.md` to scope the modern `utxos_at(payment, None)` statement to the UTxO RPC backend and record the implemented `read_genesis` prerequisite plus remaining Blockfrost differences
- updated `docs/design/33_cardano_connector.md` with explicit current-state notes for UTxO RPC semantics, Blockfrost `utxos_at(payment, None)` limitations, concrete env/config surfaces, startup/readiness split, and the weaker current Blockfrost CLI validation path
- updated `docs/design/35_adaptor_deployment.md` and `docs/design/37_adaptor_deployment_prd.md` to record the implemented env var surface, the `read_genesis` compatibility prerequisite, and the observed non-blocking `insufficient total gain` background-sync log nuance
- updated `docs/design/36_dolos_utxorpc_implementation_prd.md` to record the implemented network-derivation behavior, backend asymmetries, current CLI validation differences, and the delegated-lookup watchpoint
- updated `.opencode/workflows/rust.md`, `rust/README.md`, and `rust/crates/konduit-cli/README.md` so future Rust sessions and operators inherit the current workspace/backends/config truth instead of stale future-tense or Blockfrost-only guidance
- added durable findings in `.opencode/plans/dolos-utxorpc/research/task-104.md`
- updated the canonical task plan doc to reflect the approved strict-truth path and current implementation-review state

Files touched:
- `.opencode/plans/dolos-utxorpc/task-plans/task-104.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-104-impl-review.md`
- `.opencode/plans/dolos-utxorpc/research/task-104.md`
- `docs/adrs/06-dolos-utxorpc-adaptor-backend.md`
- `docs/design/33_cardano_connector.md`
- `docs/design/35_adaptor_deployment.md`
- `docs/design/36_dolos_utxorpc_implementation_prd.md`
- `docs/design/37_adaptor_deployment_prd.md`
- `.opencode/workflows/rust.md`
- `rust/README.md`
- `rust/crates/konduit-cli/README.md`

Verification run:
- re-read the edited docs against the current runtime sources in `konduit-server/src/cardano/args.rs`, `konduit-server/src/admin/service.rs`, `konduit-server/src/env.rs`, `konduit-cli/src/connector.rs`, `konduit-cli/src/env/connector.rs`, `konduit-cli/src/env/{admin,adaptor,consumer}.rs`, `konduit-cli/src/cmd.rs`, `konduit-cli/src/cmd/*/show.rs`, and `konduit-cli/src/shared/setup.rs`
- verified the task tracker, canonical plan doc, research note, and implementation review log are synchronized for the current in-review state
- no code changes were needed, so no additional cargo verification was required beyond repository-local consistency checks for this docs-only pass

Deviations from approved plan:
- none

Whether user interaction is now required:
- no

Code Review: Iteration 4
Timestamp: 2026-04-12T17:37:16Z
Outcome: requires_changes

Findings:
- `.opencode/workflows/rust.md:134-138` still says `When the UTxO RPC crate exists:` before `cargo test -p cardano-connector-utxorpc`, but `rust/Cargo.toml:2-20` already includes `crates/cardano-connector-utxorpc` and the same workflow doc now acknowledges that at `.opencode/workflows/rust.md:79-82`. `task-104` explicitly required updating `.opencode/workflows/rust.md` anywhere it still described the crate as future work, so this remains blocking documentation drift inside the approved scope.

Residual watchpoints:
- The current docs correctly keep the remaining Blockfrost non-parity explicit: static protocol-parameter fallback and narrower `utxos_at(payment, None)` behavior.
- The startup/readiness split remains documented truthfully between `konduit-server/src/cardano/args.rs` and `konduit-server/src/admin/service.rs`; future cleanup should not collapse those stages.
- The CLI docs now correctly distinguish parsed `utxorpc` config from live connector requirements; keep that split explicit in future edits.

Decision: requires_changes

Implementation: Iteration 5
Timestamp: 2026-04-12T17:38:12Z
Outcome: Removed the last stale future-tense wording in the Rust workflow guide so it now refers to the existing UTxO RPC crate consistently throughout the file.

Changes made:
- updated `.opencode/workflows/rust.md` to replace `When the UTxO RPC crate exists:` with `For the current UTxO RPC crate:`

Files touched:
- `.opencode/workflows/rust.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-104-impl-review.md`

Verification run:
- re-read the updated workflow section against `rust/Cargo.toml` and the earlier workflow references that already acknowledge `cardano-connector-utxorpc` as a current workspace member

Deviations from approved plan:
- none

Whether user interaction is now required:
- no

Code Review: Iteration 5
Timestamp: 2026-04-12T17:42:47Z
Outcome: approved

Findings:
- Prior sequencing problem noted: the implementation review log already contains historical review/implementation entries out of order, so this recovery entry resumes from the next valid transition after `Implementation: Iteration 5` instead of rewriting history.
- No blocking implementation or documentation issues remain in the current repository state for `task-104`. The last flagged workflow drift is fixed in `.opencode/workflows/rust.md:134-138`, the CLI README now matches the real clap flag placement in `rust/crates/konduit-cli/README.md:97-107` and `rust/crates/konduit-cli/src/cmd.rs:18-25`, and the docs now correctly distinguish parsed UTxO RPC config from live connector requirements across `rust/README.md:24-27`, `docs/design/33_cardano_connector.md:185-194`, `rust/crates/konduit-cli/README.md:54-63`, `rust/crates/konduit-cli/src/env/connector.rs:38-56`, and `rust/crates/konduit-cli/src/connector.rs:35-54`.

Residual watchpoints:
- The direct Blockfrost path still intentionally remains non-parity with UTxO RPC for static protocol-parameter fallback and narrower `utxos_at(payment, None)` behavior; future docs should keep that explicit.
- The server readiness split remains accurate between `rust/crates/konduit-server/src/cardano/args.rs` and `rust/crates/konduit-server/src/admin/service.rs`; future cleanup should not collapse those stages.
- The CLI still separates parsed `utxorpc` config from live connector use: `KONDUIT_NETWORK` is required for parsed config, while `KONDUIT_UTXORPC_URI` is only required when tip/tx flows build a live connector.

Decision: approved

Code Review: Iteration 1
Timestamp: 2026-04-12T17:30:02Z
Outcome: requires_changes

Findings:
- `rust/crates/konduit-cli/README.md:100` documents `konduit admin setup --backend utxorpc --network preview --utxorpc http://127.0.0.1:1337 >> .env.admin`, but the actual clap surface only accepts connector flags on the parent `admin` command, not after the `setup` leaf subcommand. Verified behavior: `cargo run -p konduit-cli -- admin setup --backend ...` fails with `error: unexpected argument '--backend' found`, while `cargo run -p konduit-cli -- admin --backend ... setup` succeeds. This is blocking documentation drift for `task-104` because the README now prescribes a broken bootstrap command for the finalized runtime flow.

Residual watchpoints:
- The current docs correctly keep Blockfrost non-parity explicit: static protocol-parameter fallback and narrower `utxos_at(payment, None)` behavior remain documented and should stay that way in follow-up edits.
- The readiness model is still correctly split between `konduit-server/src/cardano/args.rs` and `konduit-server/src/admin/service.rs`; future doc cleanup should avoid collapsing those checks into a single startup stage.

Decision: requires_changes

Implementation: Iteration 2
Timestamp: 2026-04-12T17:30:18Z
Outcome: Fixed the task-104 README command example so the documented local-dev `setup` flow matches the real clap surface, which requires backend flags on the parent role command before the `setup` subcommand.

Changes made:
- updated `rust/crates/konduit-cli/README.md` so the UTxO RPC and Blockfrost `admin setup` examples use `konduit admin --backend ... setup` instead of the invalid `konduit admin setup --backend ...` ordering
- updated the canonical task plan doc to record that this clap-ordering correction is part of the completed docs pass

Files touched:
- `rust/crates/konduit-cli/README.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-104.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-104-impl-review.md`

Verification run:
- verified the corrected command shape against the clap-driven role command structure in `rust/crates/konduit-cli/src/cmd.rs` and `rust/crates/konduit-cli/src/cmd/admin.rs`
- confirmed the README now matches the parent-command flag placement implied by the current clap surface

Deviations from approved plan:
- none

Whether user interaction is now required:
- no

Code Review: Iteration 2
Timestamp: 2026-04-12T17:32:27Z
Outcome: requires_changes

Findings:
- `rust/crates/konduit-cli/README.md:54-67` overstates the CLI UTxO RPC config requirement by saying `KONDUIT_UTXORPC_URI` is required for the backend in general. Current code only requires `KONDUIT_NETWORK` for parsed CLI config, while `KONDUIT_UTXORPC_URI` is required only when a command constructs a live connector (`rust/crates/konduit-cli/src/env/connector.rs:49-55`, `rust/crates/konduit-cli/src/connector.rs:35-55`, `rust/crates/konduit-cli/src/cmd/admin/show.rs:20-45`, `rust/crates/konduit-cli/src/cmd/adaptor/show.rs:21-57`, `rust/crates/konduit-cli/src/cmd/consumer/show.rs:22-54`). That drift matters because task-104’s approved plan explicitly calls out config-derived `show config` and `show address` as live-backend-independent; the README currently tells operators they need more runtime config than those commands actually require.

Residual watchpoints:
- The docs correctly keep the two accepted non-parity points explicit: direct Blockfrost still uses static protocol-parameter fallback and narrower `utxos_at(payment, None)` behavior.
- The UTxO RPC delegated-lookup watchpoint remains documented accurately as payment-credential paging plus local delegation filtering; keep that explicit in future connector or deployment edits.

Decision: requires_changes

Implementation: Iteration 3
Timestamp: 2026-04-12T17:33:09Z
Outcome: Tightened the CLI README wording so it distinguishes between parsed UTxO RPC config and live UTxO RPC connector use, matching the current command behavior where `show config` and `show address` remain config-derived but live tip and tx flows require the endpoint.

Changes made:
- updated `rust/crates/konduit-cli/README.md` so the backend-config section now says parsed `utxorpc` CLI config requires `KONDUIT_NETWORK`, while live connector use for tip and tx commands additionally requires `KONDUIT_UTXORPC_URI`

Files touched:
- `rust/crates/konduit-cli/README.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-104-impl-review.md`

Verification run:
- re-checked the README wording against `rust/crates/konduit-cli/src/env/connector.rs`, `rust/crates/konduit-cli/src/connector.rs`, and the role `show` command modules that keep `show config` and `show address` config-derived while `show tip` constructs a live connector

Deviations from approved plan:
- none

Whether user interaction is now required:
- no

Code Review: Iteration 3
Timestamp: 2026-04-12T17:35:15Z
Outcome: requires_changes

Findings:
- `docs/design/33_cardano_connector.md:187-190` and `rust/README.md:24-25` still overstate the CLI UTxO RPC requirement by saying the UTxO RPC path/config requires both `KONDUIT_UTXORPC_URI` and `KONDUIT_NETWORK`. That is not true for parsed CLI config: `ConnectorEnv -> Connector` only requires `KONDUIT_NETWORK` for `utxorpc` config construction, while `KONDUIT_UTXORPC_URI` is only enforced later when a live connector is built for tip/tx flows (`rust/crates/konduit-cli/src/env/connector.rs:49-54`, `rust/crates/konduit-cli/src/connector.rs:35-54`, `rust/crates/konduit-cli/src/cmd/admin/show.rs:20-39`, `rust/crates/konduit-cli/src/cmd/adaptor/show.rs:22-50`, `rust/crates/konduit-cli/src/cmd/consumer/show.rs:23-47`). This is blocking because the approved task-104 plan explicitly requires docs to preserve the config-derived `show config` / `show address` split instead of overstating live-backend requirements.

Residual watchpoints:
- The current docs correctly keep Blockfrost non-parity explicit: static protocol-parameter fallback and narrower `utxos_at(payment, None)` behavior remain documented and should stay explicit.
- The readiness model is still correctly split between `konduit-server/src/cardano/args.rs` and `konduit-server/src/admin/service.rs`; future doc edits should not collapse those stages.
- The UTxO RPC delegated-lookup watchpoint remains documented accurately as payment-credential paging plus local delegation filtering.

Decision: requires_changes

Implementation: Iteration 4
Timestamp: 2026-04-12T17:35:38Z
Outcome: Corrected the remaining overstatements in the connector design doc and Rust workspace overview so they distinguish parsed UTxO RPC CLI config from live connector requirements, matching the current command and env-loader behavior.

Changes made:
- updated `docs/design/33_cardano_connector.md` so the current config/runtime notes now state that parsed CLI `utxorpc` config requires `KONDUIT_NETWORK`, while live tip and tx flows additionally require `KONDUIT_UTXORPC_URI`
- updated `rust/README.md` so the backend-status section makes the same parsed-config versus live-connector distinction for the UTxO RPC path

Files touched:
- `docs/design/33_cardano_connector.md`
- `rust/README.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-104-impl-review.md`

Verification run:
- re-checked the corrected wording against `rust/crates/konduit-cli/src/env/connector.rs`, `rust/crates/konduit-cli/src/connector.rs`, and the role `show` command modules that keep parsed config separate from live connector use

Deviations from approved plan:
- none

Whether user interaction is now required:
- no

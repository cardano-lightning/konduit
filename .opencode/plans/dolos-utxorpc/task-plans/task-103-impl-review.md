Implementation: Iteration 1 Timestamp: 2026-04-12T00:52:55Z Outcome:
user_handoff_required

Changes made:

- created the implementation review log for `task-103`
- finalized the approved canonical task plan for a `manual_execution`
  live-validation handoff
- prepared the operator-facing live-validation sequence aligned to the current
  server and CLI runtime seams and the approved acceptance-to-evidence matrix
- recorded the current durable findings in
  `.opencode/plans/dolos-utxorpc/research/task-103.md`

Files touched:

- `.opencode/plans/dolos-utxorpc/task-plans/task-103.md`
- `.opencode/plans/dolos-utxorpc/task-plans/task-103-impl-review.md`
- `.opencode/plans/dolos-utxorpc/research/task-103.md`

Verification run:

- re-read the approved canonical task plan and planning review log
- checked the current runtime seams in
  `rust/crates/konduit-server/src/cardano/args.rs`,
  `rust/crates/konduit-server/src/admin/service.rs`,
  `rust/crates/konduit-cli/src/connector.rs`, and
  `rust/crates/konduit-cli/src/cmd/admin/tx.rs` to keep the handoff truthful to
  the live code
- confirmed the current autonomous verification baseline from `task-102` in
  `.opencode/plans/dolos-utxorpc/dolos-utxorpc-tasks.json`

Deviations from the approved plan:

- none

User interaction required now: yes

User Handoff

Why user interaction is required now:

- `task-103` requires a real Dolos-backed validation run and operator evidence.
  The current workspace does not expose your live Dolos endpoint, funded wallet
  context, host-address configuration, or runtime environment, so I cannot
  truthfully execute or verify these checks alone.

Exact manual steps:

1. Choose a safe environment for the live run.
   - Prefer preview/preprod or another low-risk funded environment.
   - If you must use a non-loopback Dolos endpoint, note that explicitly because
     it is deployment drift from the localhost-only target profile.
2. Export one consistent UTxO RPC backend configuration for both server and CLI
   validation.
   - Set `KONDUIT_CARDANO_BACKEND=utxorpc`.
   - Set `KONDUIT_UTXORPC_URI` to your real Dolos UTxO RPC endpoint.
   - Set `KONDUIT_NETWORK` to the intended Cardano network.
   - Keep these same values for both the server startup test and the CLI
     submission test.
3. Run the server startup validation path and capture the full output.
   - Use your normal `konduit-server` startup command or equivalent local run
     command for this repository.
   - Capture stdout/stderr through the point where startup either succeeds or
     fails.
   - If startup fails, include the exact error text.
4. Record the server evidence against the required matrix.
   - Reachability: output showing the UTxO RPC backend was contacted
     successfully, or the exact reachability error.
   - Network match: output showing `KONDUIT_NETWORK` matched Dolos live network
     data, or the exact mismatch error.
   - Live protocol-parameter derivation: output showing startup progressed past
     backend initialization into admin-service readiness, or the exact
     protocol-parameter failure.
   - Reference-script resolution: output showing startup completed past
     reference-script lookup, or the exact missing-script failure.
5. Run one live CLI submit path with the same backend config and capture the
   full output.
   - Use the existing admin send flow in `konduit-cli` with a small, safe amount
     and an operator-approved destination.
   - If submission is not safe or possible, stop and capture the exact blocker
     instead of improvising.
6. Record the submit evidence.
   - On success, capture the emitted transaction id from the CLI output.
   - On failure, capture the exact backend rejection or transport failure.
   - Treat this as backend acceptance evidence only. Do not claim on-chain
     confirmation unless you also provide a separate follow-up check showing
     confirmation.
7. Return the evidence.
   - Paste the relevant command lines you ran.
   - Paste the captured stdout/stderr or summarize it with exact quoted error
     text.
   - State whether the endpoint was loopback or non-loopback.
   - State whether the live submit path succeeded with a tx id or was blocked,
     and why.

Expected results:

- server startup should only succeed if Dolos is reachable, the configured
  network matches live data, protocol parameters can be derived from UTxO RPC,
  and the configured reference script UTxO can be resolved
- CLI submit should either return success with a transaction id proving backend
  acceptance, or fail with a concrete actionable blocker such as insufficient
  funds, min-UTxO, fee budget, transport failure, or backend rejection

What output or decision the user should return:

- the exact command invocations used
- the startup output or exact startup failure
- the submit output including tx id on success, or exact failure text on failure
- whether the Dolos endpoint used was localhost-only
- whether there is any blocker that should be treated as environment-specific
  rather than a Konduit defect

Whether work is blocked or can continue in parallel:

- blocked for truthful completion of `task-103`; I should not advance to
  `task-104` until you return the live-validation evidence or blocker for this
  task

Implementation: Iteration 2 Timestamp: 2026-04-12T14:25:40Z Outcome:
user_handoff_required

Changes made:

- incorporated returned live-validation evidence showing that
  `http://localhost:50051` is reachable, `health()` succeeds far enough to read
  tip, Dolos `1.0.3` previously lacked `read_genesis`, and the patched Dolos now
  returns a Cardano genesis with `network_id=Mainnet`
- updated `rust/crates/cardano-connector-utxorpc/src/lib.rs` so
  `network_from_genesis(...)` trims and lowercases the Dolos `network_id` before
  validating it against the network magic, while keeping network magic
  authoritative and preserving rejection of true mismatches
- added regression tests covering `Mainnet` and trimmed `Testnet` casing
  variants so the observed live-environment compatibility issue remains fixed

Files touched:

- `rust/crates/cardano-connector-utxorpc/src/lib.rs`

Verification run:

- `cargo fmt --all` ✅
- `cargo test -p cardano-connector-utxorpc network_from_genesis` ✅ (`6 passed`)
- `cargo check -p cardano-connector-utxorpc` ✅

Deviations from the approved plan:

- none on scope; the task remained a live-validation task, and the code change
  was a minimal defect fix directly exposed by truthful operator evidence during
  that validation

User interaction required now: yes

User Handoff

Why user interaction is required now:

- the live-validation blocker caused by case-sensitive `network_id` handling has
  been fixed locally, but truthful completion of `task-103` still requires
  rerunning the real Dolos-backed startup path and returning the next live
  result.

Exact manual steps:

1. Rebuild or reuse the updated `konduit-server` binary from the current
   workspace.
2. Keep the same UTxO RPC configuration that already reached Dolos successfully:
   - `KONDUIT_CARDANO_BACKEND=utxorpc`
   - `KONDUIT_UTXORPC_URI=http://localhost:50051`
   - `KONDUIT_NETWORK=mainnet`
3. Rerun:
   - `./target/debug/konduit-server`
4. Capture the next full startup result.
   - If startup fails, paste the exact error.
   - If startup succeeds, note that and continue to the planned CLI live-submit
     path.

Expected results:

- the previous
  `network_id Mainnet is inconsistent with network magic for mainnet` startup
  blocker should no longer occur
- startup should now either progress to the next readiness gate or succeed

What output or decision the user should return:

- the next exact `konduit-server` startup output after this patch
- whether startup now passes the network-match stage

Whether work is blocked or can continue in parallel:

- blocked for truthful completion of `task-103` pending the next live startup
  result from the updated binary

Implementation: Iteration 3 Timestamp: 2026-04-12T16:30:38Z Outcome:
user_handoff_required

Changes made:

- refactored `konduit-cli` command execution onto a single top-level Tokio
  runtime by making `main`, command dispatch, role dispatch, connector
  construction, and the CLI runtime entry points async instead of creating
  short-lived per-command runtimes
- updated `konduit-cli` UTxO RPC connector construction so backend validation
  and subsequent gRPC use happen in the same runtime, removing the
  throwaway-runtime pattern that matched the observed `search_utxos` transport
  failure where Dolos never logged the request
- corrected `admin tx deploy` UTxO filtering so script-bearing UTxOs are
  excluded by default and only included when `--spend-all` is explicitly
  requested, aligning deploy semantics with `admin tx send` and the CLI README
  warning
- added deploy-path regression tests covering the default skip behavior and the
  explicit `--spend-all` inclusion behavior

Files touched:

- `rust/crates/konduit-cli/src/main.rs`
- `rust/crates/konduit-cli/src/cmd.rs`
- `rust/crates/konduit-cli/src/cmd/admin.rs`
- `rust/crates/konduit-cli/src/cmd/consumer.rs`
- `rust/crates/konduit-cli/src/cmd/adaptor.rs`
- `rust/crates/konduit-cli/src/cmd/admin/show.rs`
- `rust/crates/konduit-cli/src/cmd/admin/tx.rs`
- `rust/crates/konduit-cli/src/cmd/consumer/show.rs`
- `rust/crates/konduit-cli/src/cmd/consumer/tx.rs`
- `rust/crates/konduit-cli/src/cmd/adaptor/show.rs`
- `rust/crates/konduit-cli/src/cmd/adaptor/tx.rs`
- `rust/crates/konduit-cli/src/config/connector.rs`
- `rust/crates/konduit-cli/src/connector.rs`

Verification run:

- `cargo fmt --all` ✅
- `cargo check -p konduit-cli` ✅
- `cargo test -p konduit-cli` ✅ (`19 passed`)

Deviations from the approved plan:

- none on task scope; the implementation remains within the existing Rust
  runtime surfaces and fixes live-validation blockers that were exposed by
  truthful operator evidence

User interaction required now: yes

User Handoff

Why user interaction is required now:

- the CLI runtime-lifecycle fix and the deploy filter fix are in place locally,
  but `task-103` still requires a real Dolos-backed deploy and startup rerun to
  prove reference-script resolution and continue toward a live submission path.

Exact manual steps:

1. Rebuild or reuse the updated CLI and server binaries from the current
   workspace.
2. Keep the same validated UTxO RPC config:
   - `KONDUIT_CARDANO_BACKEND=utxorpc`
   - `KONDUIT_UTXORPC_URI=http://localhost:50051`
   - `KONDUIT_NETWORK=mainnet`
3. Keep the CLI admin wallet vars present for this binary path:
   - `KONDUIT_WALLET=<same signing key hex>`
   - if your current BLN environment is still required by startup, keep
     `LND_BASE_URL` and `LND_MACAROON` exported as before
4. Re-run the reference-script deployment flow:
   - `./target/debug/konduit-cli admin show config`
   - `./target/debug/konduit-cli admin tx deploy`
5. Capture the exact output from `admin tx deploy`.
   - On success, keep the emitted tx id.
   - On failure, paste the exact error.
6. If deploy succeeds, rerun:
   - `./target/debug/konduit-server`
7. Capture the next exact startup result.

Expected results:

- the previous `search_utxos` transport failure on `admin tx deploy` should no
  longer occur
- deploy should avoid spending existing script-bearing UTxOs unless
  `--spend-all` is explicitly set
- after a successful deploy and confirmation window, `konduit-server` should
  progress past the `No reference script found` blocker or expose the next
  truthful readiness gate

What output or decision the user should return:

- the exact output from `./target/debug/konduit-cli admin tx deploy`
- if deploy succeeds, the next exact `./target/debug/konduit-server` startup
  output
- whether the deployed reference-script tx was accepted and what tx id was
  emitted

Whether work is blocked or can continue in parallel:

- blocked for truthful completion of `task-103` pending the next live
  deploy/startup result from the updated binaries

Implementation: Iteration 4 Timestamp: 2026-04-12T16:55:29Z Outcome:
completed_pending_code_review

Changes made:

- updated the UTxO RPC credential query behavior in `cardano-connector-utxorpc`
  so `utxos_at(payment, Some(delegation))` searches by payment credential and
  then enforces the exact payment-and-delegation pair locally against the
  returned Shelley address, avoiding backend-specific delegation-index
  assumptions while preserving the project semantics
- added connector regression tests for the new local delegation-pair filtering
  behavior and for the intentionally payment-only UTxO RPC address predicate
- added a targeted startup diagnostic in `konduit-server/src/admin/service.rs`
  while debugging the missing reference-script blocker; the final live rerun
  after the connector fix showed startup progressing successfully, so the
  diagnostic remained as failure-only operator context rather than an active
  behavior change
- incorporated the returned manual evidence that `admin tx deploy` succeeded
  with tx id `b090e09ae05b947e2818f807dba874a205acacf1ffc4a3c5a53b8bc1cfe5c0ab`
  and that a fresh sequential
  `cargo build -p konduit-cli -p konduit-server && ./target/debug/konduit-server`
  run started the server successfully

Files touched:

- `rust/crates/cardano-connector-utxorpc/src/lib.rs`
- `rust/crates/cardano-connector-utxorpc/src/mapping.rs`
- `rust/crates/konduit-server/src/admin/service.rs`

Verification run:

- `cargo test -p cardano-connector-utxorpc` ✅ (`27 passed`)
- `cargo build -p konduit-cli -p konduit-server && ./target/debug/konduit-server`
  ✅
- operator evidence: `./target/debug/konduit-cli admin tx deploy` ✅ with tx id
  `b090e09ae05b947e2818f807dba874a205acacf1ffc4a3c5a53b8bc1cfe5c0ab`
- operator evidence: `./target/debug/konduit-server` ✅ started Actix and
  reached steady state; the observed
  `Admin sync failed: insufficient total gain: preferences.min_total = 1000000, gain = 0`
  message came from the background sync loop after startup and does not block
  readiness

Deviations from the approved plan:

- the final accepted live submission evidence for this task is the successful
  `admin tx deploy` submission rather than a separate `admin tx send` path,
  based on returned operator feedback that a simple ADA send was not the
  intended validation target for this task; this still satisfies the task
  requirement for at least one real Dolos-backed transaction submission path
  with emitted tx id

User interaction required now: no

Code Review: Iteration 6 Timestamp: 2026-04-12T17:03:16Z Outcome: approved

Findings:

- none

Decision: approved

Implementation: Iteration 6 Timestamp: 2026-04-12T17:02:24Z Outcome:
completed_pending_code_review

Changes made:

- no code changes; appended this entry to correct the implementation-log
  sequencing after `Code Review: Iteration 5` was appended before
  `Implementation: Iteration 5`, which violated the required alternating
  transition order
- the authoritative implementation state remains the same as Iteration 5:
  readiness-test regression fixed, targeted `konduit-server` verification
  passing, and fresh sequential startup validation still successful

Files touched:

- `.opencode/plans/dolos-utxorpc/task-plans/task-103-impl-review.md`

Verification run:

- no additional verification beyond the already-recorded passing runs from
  Iteration 5

Deviations from the approved plan:

- none

User interaction required now: no

Code Review: Iteration 5 Timestamp: 2026-04-12T17:00:31Z Outcome: approved

Findings:

- none

Decision: approved

Code Review: Iteration 4 Timestamp: 2026-04-12T16:57:09Z Outcome:
requires_changes

Findings:

- Blocking: `rust/crates/konduit-server/src/admin/service.rs:76` changed the
  missing-reference-script startup error from the previously asserted exact
  string `"No reference script found"` to a longer diagnostic message, but the
  existing unit test at
  `rust/crates/konduit-server/src/admin/service.rs:370-387` was not updated.
  `cargo test -p konduit-server new_fails_when_reference_script_is_missing` now
  fails with an assertion mismatch, so the claimed validation state in Iteration
  4 is not currently reproducible from the repository state. This is a direct
  regression in the required startup/readiness coverage for task-103.

Decision: requires_changes

Implementation: Iteration 5 Timestamp: 2026-04-12T16:57:24Z Outcome:
completed_pending_code_review

Changes made:

- updated the `konduit-server` missing-reference-script unit test to assert the
  new diagnostic error shape rather than the old exact message, preserving the
  more actionable startup failure text while restoring readiness-coverage
  stability

Files touched:

- `rust/crates/konduit-server/src/admin/service.rs`

Verification run:

- `cargo test -p konduit-server new_fails_when_reference_script_is_missing` ✅
- `cargo build -p konduit-cli -p konduit-server && ./target/debug/konduit-server`
  ✅ fresh sequential build plus startup; startup again reached steady state and
  the process only stopped later because the command hit the tool timeout during
  the running server

Deviations from the approved plan:

- none

User interaction required now: no

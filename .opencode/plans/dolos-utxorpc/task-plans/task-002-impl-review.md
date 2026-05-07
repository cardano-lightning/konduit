Implementation: Iteration 1 Timestamp: 2026-04-11T20:20:00Z Outcome: completed

Changes made:

- replaced the `task-001` placeholder in `cardano-connector-utxorpc` with a real
  `UtxoRpc` connector that builds query, submit, and sync clients from an
  explicit endpoint config
- added `config.rs` to carry explicit UTxO RPC endpoint and configured Cardano
  network
- added `mapping.rs` to map UTxO RPC `TxoRef` and `TxOutput` values into
  `cardano-sdk` `Input` and `Output`, using native CBOR bytes when available and
  a parsed fallback for datum, asset, and reference-script fields
- added `params.rs` to derive live protocol parameters from `read_params` and
  `read_era_summary`, including fees, execution prices, reference-script cost
  base fee, Plutus V3 cost model, `start_time`, and `first_shelley_slot`
- implemented paginated `utxos_at` lookup using a remote address-pattern
  predicate plus local payment-credential filtering for
  `utxos_at(payment, None)` semantics
- implemented `health()` through `read_tip()` and `submit()` through
  `submit_tx()` with endpoint-specific error context

Files touched:

- `rust/crates/cardano-connector-utxorpc/Cargo.toml`
- `rust/crates/cardano-connector-utxorpc/src/lib.rs`
- `rust/crates/cardano-connector-utxorpc/src/config.rs`
- `rust/crates/cardano-connector-utxorpc/src/mapping.rs`
- `rust/crates/cardano-connector-utxorpc/src/params.rs`
- `rust/Cargo.lock`

Verification run:

- `cargo check -p cardano-connector-utxorpc` ✅
- `cargo test -p cardano-connector-utxorpc` ✅
- `cargo doc -p cardano-connector-utxorpc --no-deps` ✅
- `cargo fmt --all -- --check` ✅ after formatting
- `cargo clippy -p cardano-connector-utxorpc --all-targets -- -D warnings` ✅

Deviations from the approved plan:

- protocol-parameter derivation uses live `read_params` plus `read_era_summary`;
  `start_time` is reconstructed from the Shelley era boundary and the
  ledger-fixed 20-second Byron slot length because the reviewed `utxorpc` client
  helper exposes raw genesis bytes but not a parsed genesis config helper on
  this path
- output mapping prefers native CBOR decoding when present, but falls back to
  parsed protobuf fields when native bytes are absent so the connector remains
  truthful against the current SDK surface

User interaction required now: no

Implementation: Iteration 2 Timestamp: 2026-04-11T20:28:00Z Outcome: completed

Changes made:

- fixed parsed fallback `BigUInt` conversion so values larger than `u64` are
  rejected instead of silently truncating
- strengthened `health()` failure context to include the configured endpoint on
  read-tip errors
- added focused unit tests for overflow handling, payment-only predicate
  semantics, `matches_payment` delegation-insensitive behavior, and
  protocol-parameter helper error handling

Files touched:

- `rust/crates/cardano-connector-utxorpc/src/lib.rs`
- `rust/crates/cardano-connector-utxorpc/src/mapping.rs`
- `rust/crates/cardano-connector-utxorpc/src/params.rs`

Verification run:

- `cargo fmt --all -- --check` ✅
- `cargo check -p cardano-connector-utxorpc` ✅
- `cargo test -p cardano-connector-utxorpc` ✅ (`5 passed`)
- `cargo clippy -p cardano-connector-utxorpc --all-targets -- -D warnings` ✅
- `cargo doc -p cardano-connector-utxorpc --no-deps` ✅

Deviations from the approved plan:

- none beyond the already documented implementation details from iteration 1;
  this iteration only closed code-review findings and added focused tests ahead
  of the broader phase-2 test task

User interaction required now: no

Code Review: Iteration 1 Timestamp: 2026-04-11T22:50:51Z Outcome:
requires_changes

Findings:

- `rust/crates/cardano-connector-utxorpc/src/mapping.rs:172-176` silently
  truncates parsed `cardano::big_int::BigInt::BigUInt` values to `u64` by
  left-folding arbitrary-length byte slices without a width check. On the parsed
  fallback path this can corrupt lovelace or native-asset quantities instead of
  failing truthfully when Dolos returns an out-of-range value, which breaks the
  approved mapping-correctness requirement.
- `cargo test -p cardano-connector-utxorpc` currently proves none of the new
  connector-core behavior because the crate has zero tests (`running 0 tests`).
  For `task-002`, that leaves the highest-risk behavior introduced here
  unverified: `utxos_at(payment, None)` semantics, live protocol-parameter
  derivation, parsed-output fallback mapping, and submit error translation.
  `task-100` is the larger test workstream, but this implementation iteration
  still needs at least minimal focused coverage for the new core before
  downstream runtime wiring builds on it.
- `rust/crates/cardano-connector-utxorpc/src/lib.rs:53-58` returns `health()`
  failures with only `failed to read Dolos tip`, while the implementation log
  claims endpoint-specific error context for the health path too. The success
  string is useful, but the failure diagnostics are currently weaker than the
  approved plan's operator-debuggability goal.

Verification re-run:

- `cargo check -p cardano-connector-utxorpc` ✅
- `cargo test -p cardano-connector-utxorpc` ✅ (`running 0 tests`)
- `cargo clippy -p cardano-connector-utxorpc --all-targets -- -D warnings` ✅
- `cargo doc -p cardano-connector-utxorpc --no-deps` ✅

Decision: requires_changes

Code Review: Iteration 2 Timestamp: 2026-04-11T22:53:52Z Outcome: approved

Findings:

- The prior parsed-fallback correctness bug is fixed.
  `rust/crates/cardano-connector-utxorpc/src/mapping.rs:180-188` now rejects
  `BigUInt` byte sequences larger than `u64`, and the new
  `big_uint_overflow_is_rejected` unit test covers that regression directly.
- The prior health-diagnostics gap is fixed.
  `rust/crates/cardano-connector-utxorpc/src/lib.rs:53-57` now includes the
  configured endpoint in the `read_tip()` failure context, which matches the
  approved operator-debuggability goal for this task's connector-core scope.
- The prior zero-test gap is fixed for this iteration.
  `cargo test -p cardano-connector-utxorpc` now runs 5 focused tests covering
  overflow rejection, payment-only predicate shape, delegation-insensitive
  payment matching, and protocol-parameter helper error paths. Broader connector
  behavior coverage is still rightly deferred to `task-100`, but this iteration
  now has the minimal core coverage that the previous review required.
- I did not find new scope drift into CLI/server backend selection,
  startup/readiness enforcement, or Blockfrost-path changes. The work remains
  contained to `cardano-connector-utxorpc`, preserving the approved crate
  boundary for `task-002`.

Verification re-run:

- `cargo fmt --all -- --check` ✅
- `cargo check -p cardano-connector-utxorpc` ✅
- `cargo test -p cardano-connector-utxorpc` ✅ (`5 passed`)
- `cargo clippy -p cardano-connector-utxorpc --all-targets -- -D warnings` ✅
- `cargo doc -p cardano-connector-utxorpc --no-deps` ✅

Decision: approved

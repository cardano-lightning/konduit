Planner: Iteration 1 Timestamp: 2026-04-11T23:43:35Z Outcome: Created the
initial planning draft for `task-100` with autonomous interaction mode, scope
limited to connector-crate-local test coverage in `cardano-connector-utxorpc`, a
minimal-refactor approach to improve testability only where needed, and
verification centered on targeted crate checks and tests rather than live Dolos
validation.

Plan summary for critique: Expand `cardano-connector-utxorpc` unit coverage for
mapping success and rejection behavior, protocol-parameter derivation and
failure paths, explicit `utxos_at(payment, None)` semantics, and submit-path
error translation, while keeping any code changes minimal, local to the
connector crate, and free of broad new abstractions or runtime-surface drift.

Critiquer: Iteration 1 Timestamp: 2026-04-11T23:45:30Z Outcome: requires_changes

Findings:

- The plan still leaves a false-positive path for `utxos_at(payment, None)`
  acceptance. In the current crate, the real semantics are implemented by
  `UtxoRpc::load_utxos()` paging `search_utxos()` to exhaustion, mapping each
  page, then locally re-filtering outputs by payment credential when delegation
  is absent (`rust/crates/cardano-connector-utxorpc/src/lib.rs:60-98`). The plan
  says helper-level coverage may be enough if current tests cover predicate
  shape and post-map matcher separately, but that would not catch a regression
  where pagination stops early, the local filter is skipped on later pages, or
  mapping failures inside the page loop drift. The plan needs a concrete test
  seam for the whole page-accumulation and filter path, even if that is a small
  extracted pure helper rather than a broad fake-client abstraction.
- The mapping section does not explicitly protect the highest-fidelity source
  rule already established by `task-002` research. `map_output_data()` prefers
  native transaction-output bytes over parsed protobuf output whenever native
  bytes are present
  (`rust/crates/cardano-connector-utxorpc/src/mapping.rs:31-44`), but the plan
  only says to add native-bytes and parsed-output mapping success cases. That
  leaves a real stale-assumption gap: a test suite could pass while never
  asserting that native bytes win over parsed fallback, or that parsed fallback
  is only used when native bytes are absent. The plan should require an explicit
  precedence test plus a fallback test so a future refactor cannot silently
  reintroduce lower-fidelity parsed mapping when native bytes exist.
- The protocol-parameter section is still too generic around the current
  Dolos-specific era-summary assumption. The live implementation reconstructs
  `start_time` from the Shelley era boundary and the fixed 20-second Byron slot
  length, and it fails if that computed chain start underflows
  (`rust/crates/cardano-connector-utxorpc/src/params.rs:45-55`). The plan
  mentions missing Shelley era data and missing execution prices or cost model,
  but it does not explicitly require success assertions for `start_time` and
  `first_shelley_slot` derivation or the underflow failure path. Given the ADR
  and PRD forbid silently falling back to static presets, these reconstructed
  timing semantics are exactly the kind of stale UTxO RPC assumption that
  `task-100` should lock down.

Required changes:

- Strengthen the `utxos_at` implementation approach so `task-100` requires a
  truthful unit-test seam for the full paginated accumulation path in
  `load_utxos()`, not only separate predicate and matcher helper tests. The plan
  should state how multi-page results, post-page local payment filtering, and
  mapping errors will be exercised without introducing a large new abstraction.
- Amend the mapping-test section to require explicit coverage that native output
  bytes take precedence over parsed output when both are present, and that
  parsed fallback remains the path only when native bytes are absent.
- Amend the protocol-parameter section to require explicit success and failure
  tests for the current Shelley-boundary-based `start_time` derivation,
  including a success assertion for derived `start_time` and
  `first_shelley_slot` plus a failure assertion for computed negative chain
  start.
- Add a small `lib.rs` network-handling note so `task-100` closes the remaining
  connector-core stale-assumption cases already in scope: unsupported
  `network_magic`, empty `network_id`, and inconsistent `network_id` versus
  magic should remain covered as connector-core behavior rather than being left
  only to later runtime tests.

Decision: requires_changes

Planner: Iteration 2 Timestamp: 2026-04-11T23:46:59Z Outcome: Revised the
`task-100` plan to require full paginated `utxos_at(payment, None)` coverage
through a narrow crate-local seam, explicit native-bytes-over-parsed mapping
precedence tests, explicit Shelley-boundary timing assertions for
protocol-parameter derivation, and connector-core network edge-case coverage in
`lib.rs`.

Plan summary for critique: Keep `task-100` limited to
`cardano-connector-utxorpc`, but require unit coverage for the full paginated
page-accumulation and post-filter path behind `utxos_at(payment, None)`,
explicit native-output precedence over parsed fallback, explicit `start_time`
and `first_shelley_slot` derivation plus underflow failure coverage, and the
remaining connector-core network mapping edge cases, while still keeping any
testability refactors narrow and local to the crate.

Critiquer: Iteration 2 Timestamp: 2026-04-11T23:48:03Z Outcome: approved

Findings:

- The revised plan addresses the earlier semantic coverage gaps in a way that
  matches the current crate behavior and the task boundary. It now requires a
  truthful unit-test seam for the full paginated `load_utxos()` accumulation and
  post-page payment-only filtering path in
  `rust/crates/cardano-connector-utxorpc/src/lib.rs:60-98`, which is the right
  place to guard the documented `utxos_at(payment, None)` contract from
  regressions.
- The mapping section now explicitly locks down the highest-fidelity-source rule
  in `rust/crates/cardano-connector-utxorpc/src/mapping.rs:31-44` by requiring
  both native-bytes precedence coverage and parsed-fallback-only coverage. That
  closes the stale-assumption risk called out by `task-002` research and stays
  aligned with the ADR and connector mapping guidance.
- The protocol-parameter coverage now explicitly targets the live
  Shelley-boundary timing reconstruction in
  `rust/crates/cardano-connector-utxorpc/src/params.rs:45-55`, including both
  derived `start_time` and `first_shelley_slot` success assertions and the
  negative-chain-start failure path. That is the right safeguard against
  drifting back toward static preset assumptions, which the design docs
  explicitly forbid for the UTxO RPC backend.
- The added network-edge-case note for `network_from_genesis()` keeps the
  remaining connector-core assumptions under test in
  `rust/crates/cardano-connector-utxorpc/src/lib.rs:140-161` without expanding
  scope into later runtime-surface tasks. That is a minimal and appropriate
  addition for this plan.

Decision: approved

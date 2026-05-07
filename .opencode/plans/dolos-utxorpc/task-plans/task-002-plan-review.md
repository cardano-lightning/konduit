Planner: Iteration 1 Timestamp: 2026-04-11T19:57:26Z Outcome: Created the
initial planning draft for `task-002` with autonomous interaction mode, scope
limited to `cardano-connector-utxorpc`, explicit non-goals excluding CLI/server
wiring and fake live Dolos claims, a module-oriented implementation approach for
config, mapping, and live protocol-parameter derivation, and concrete critique
targets around SDK transport assumptions, payment-only UTxO matching semantics,
parameter completeness, mapping fidelity, and submit truthfulness.

Plan summary for critique: Replace the placeholder crate with a real Dolos UTxO
RPC `CardanoConnector` implementation in `cardano-connector-utxorpc`, derive
protocol parameters from UTxO RPC query data instead of static network presets,
map UTxOs into existing `cardano-sdk` types with truthful datum and script
handling, preserve `utxos_at(payment, None)` semantics, and keep CLI/server
readiness work deferred to later tasks.

Critiquer: Iteration 1 Timestamp: 2026-04-11T20:01:49Z Outcome: requires_changes

Findings:

- The transport contingency in the plan is stale. The current `utxorpc` Rust SDK
  examples already use `ClientBuilder::new().uri("http://localhost:50051")`, so
  task-002 should plan to use the SDK directly for the documented localhost
  Dolos path instead of carrying a speculative fallback to a custom plaintext
  `tonic` endpoint path.
- The `utxos_at(payment, None)` section is still underspecified on completeness
  and performance. `search_utxos` is paginated and returns a `next_token`, so
  the plan needs to say the connector will page until exhaustion or an exact
  stopping condition before applying any local payment-credential filter.
  Without that, the implementation can silently truncate results or accidentally
  turn a broad local filter into an unbounded scan.
- Verification is slightly light for a connector crate that will expose the
  first real public API in `cardano-connector-utxorpc`.
  `.opencode/workflows/rust.md` says public crate API changes should build docs
  when interfaces change materially. The plan should either add
  `cargo doc -p cardano-connector-utxorpc --no-deps` to task-002 verification or
  explicitly defer that check with a truthful note.
- The research-update language should be tightened from optional to expected for
  this task. Task-002 is where upstream UTxO RPC and Dolos behavior first
  becomes concrete; if mapping, era-summary/genesis derivation, or submit-stage
  behavior differs from plan assumptions, recording that in
  `.opencode/plans/dolos-utxorpc/research/task-002.md` should be part of the
  task output, not just a maybe.

Required changes:

- Replace the custom plaintext transport contingency with a plan to use the
  current `utxorpc` SDK client builder for `http://` localhost Dolos endpoints
  unless implementation proves otherwise.
- Amend the `utxos_at` implementation approach and risks to require explicit
  pagination handling and bounded local filtering so payment-only semantics are
  complete and truthful.
- Add `cargo doc -p cardano-connector-utxorpc --no-deps` to the verification
  plan, or explicitly justify why that public-API doc build is deferred.
- Change the task-002 research update note so implementation is expected to
  write `.opencode/plans/dolos-utxorpc/research/task-002.md` with the concrete
  upstream API findings that shaped the final connector behavior.

Decision: requires_changes

Planner: Iteration 2 Timestamp: 2026-04-11T20:02:33Z Outcome: Revised the
`task-002` plan to address all critique requirements. The plan now commits to
using the reviewed `utxorpc` SDK client builder directly for localhost `http://`
Dolos endpoints unless implementation proves a real SDK gap, makes pagination to
exhaustion explicit for `utxos_at(payment, None)` before any local
payment-credential filtering, adds
`cargo doc -p cardano-connector-utxorpc --no-deps` to the verification plan
because this task introduces the first real public API in the crate, and
upgrades `.opencode/plans/dolos-utxorpc/research/task-002.md` from an optional
note to an expected durable task output.

Plan summary for critique: Keep `task-002` scoped to
`cardano-connector-utxorpc`, implement the connector with the current UTxO RPC
SDK surface, derive live protocol parameters from UTxO RPC data, exhaust
paginated UTxO search results before applying any payment-only local filter,
verify the new public crate API with targeted docs build alongside
fmt/check/test/clippy, and require a durable research note documenting the
upstream API findings that shaped the final implementation.

Critiquer: Iteration 2 Timestamp: 2026-04-11T20:03:23Z Outcome: approved

Findings:

- The prior transport blocker is fixed. The revised plan now aligns with the
  currently reviewed `utxorpc` SDK surface and uses the documented localhost
  `http://` client-builder path instead of carrying a speculative custom
  transport fallback.
- The prior `utxos_at(payment, None)` completeness blocker is fixed. The plan
  now requires explicit pagination handling before any local payment-credential
  filtering, which addresses both silent truncation risk and accidental
  open-ended scan drift.
- The prior verification blocker is fixed. The plan now includes
  `cargo doc -p cardano-connector-utxorpc --no-deps` alongside
  fmt/check/test/clippy for this new public connector crate surface.
- The prior research-output blocker is fixed. The revised plan now expects a
  durable `.opencode/plans/dolos-utxorpc/research/task-002.md` capturing the
  upstream API findings that shape the final connector behavior.
- Remaining uncertainties around payment-only predicate expressiveness,
  protocol-parameter derivation sources, mapping source of truth, and whether
  `submit_tx` alone is sufficiently truthful are already captured as
  implementation-time risks and do not require another planner iteration.

Decision: approved

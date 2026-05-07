# task-103 research

- live-validation seam finding: the truthful server evidence seam remains the
  existing startup path split between `konduit-server/src/cardano/args.rs` for
  UTxO RPC reachability and live-network match and
  `konduit-server/src/admin/service.rs` for protocol-parameter derivation plus
  reference-script resolution
- live-submit seam finding: the preferred minimal live submission seam remains
  the CLI admin `send` path in `konduit-cli/src/cmd/admin/tx.rs` because it
  exercises network-derived address selection, UTxO lookup, protocol-parameter
  retrieval, transaction building, signing, and backend submission in one
  current runtime flow
- evidence-contract finding: task completion for live submission must be phrased
  as backend submit acceptance plus transaction id or backend rejection details;
  the current `CardanoConnector::submit()` contract in both Blockfrost and UTxO
  RPC paths does not by itself prove chain confirmation
- operator-drift finding: if the returned Dolos endpoint is not localhost-only,
  that should be recorded as deployment-environment drift against the current
  target profile rather than silently normalized in later docs
- live-compatibility finding: Dolos `1.0.3` in the observed environment accepted
  gRPC tip reads but originally panicked on `read_genesis`; after an
  operator-side Dolos patch implemented `read_genesis`, the returned Cardano
  genesis used `network_id=Mainnet`, which exposed a Konduit-side case-sensitive
  validation bug even though the returned `network_magic` already identified
  mainnet correctly
- accepted fix finding: `cardano-connector-utxorpc::network_from_genesis(...)`
  now trims and lowercases `network_id` before comparing it with the network
  derived from `network_magic`, preserving `network_magic` as the authoritative
  signal while still rejecting true network-id or magic mismatches
- cli-runtime finding: the observed `admin tx deploy` failure
  `Service was not ready: transport error` was consistent with `konduit-cli`
  constructing and validating a `UtxoRpc` connector inside a temporary Tokio
  runtime in `src/connector.rs`, then reusing that connector in a different
  runtime for later gRPC calls; fresh Dolos logs showed `read_genesis` requests
  but no `search_utxos` handler entry, which pointed to client-side transport
  failure before the server handler body ran
- accepted cli fix finding: `konduit-cli` now runs on a single top-level Tokio
  runtime and creates or validates its connectors within that same runtime,
  removing the per-command `Runtime::new()?.block_on(...)` pattern across admin,
  adaptor, and consumer command paths
- deploy-safety finding: `admin tx deploy` previously inverted its `spend_all`
  filter and could include script-bearing UTxOs by default; the filter now
  matches `admin tx send` semantics so script UTxOs are skipped unless
  `--spend-all` is explicitly requested
- utxos-at semantics finding: live reference-script lookup exposed that backend
  delegation-index assumptions were too strong for the UTxO RPC path;
  `cardano-connector-utxorpc` now queries by payment credential and enforces the
  exact payment-and-delegation pair locally for
  `utxos_at(payment, Some(delegation))`, preserving the fixed task semantics
  without relying on backend-specific `delegation_part` matching behavior
- final live-evidence finding: operator validation on localhost Dolos succeeded
  with `./target/debug/konduit-cli admin tx deploy` returning tx id
  `b090e09ae05b947e2818f807dba874a205acacf1ffc4a3c5a53b8bc1cfe5c0ab`, followed
  by fresh sequential
  `cargo build -p konduit-cli -p konduit-server && ./target/debug/konduit-server`
  startup reaching steady state; the later `insufficient total gain` log was a
  non-blocking background admin-sync condition rather than a readiness failure

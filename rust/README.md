# Konduit Rust Workspace

> Rust crates for Konduit runtime, connector, transaction, and client surfaces

## Workspace

- `cardano-connector` - shared Cardano connector trait boundary
- `cardano-connector-direct` - direct Blockfrost-backed connector implementation
- `cardano-connector-utxorpc` - direct Dolos UTxO RPC connector implementation
- `cardano-sdk` - Cardano primitives and transaction-building support
- `konduit-tx` - Konduit transaction builders
- `konduit-data` - shared Konduit protocol data and encoding
- `konduit-cli` - CLI runtime for admin, adaptor, and consumer flows
- `konduit-server` - adaptor-facing HTTP server runtime
- `konduit-client` - reusable client logic over Konduit surfaces
- `konduit-wasm` - WASM-facing API surface

## Cardano Backend Status

- `konduit-server` and `konduit-cli` both support explicit backend selection via
  `blockfrost` and `utxorpc`.
- the target production-style backend for this effort is `utxorpc` against a
  localhost Dolos instance.
- the UTxO RPC path uses explicit `KONDUIT_CARDANO_BACKEND=utxorpc`
  configuration. `KONDUIT_NETWORK` is required for parsed CLI config and server
  startup, while `KONDUIT_UTXORPC_URI` is additionally required when a live UTxO
  RPC connector is constructed.
- the direct Blockfrost path remains available in parallel via
  `KONDUIT_CARDANO_BACKEND=blockfrost` and `KONDUIT_BLOCKFROST_PROJECT_ID`.

## Current Runtime Truth

- `konduit-server` performs backend construction in
  `konduit-server/src/cardano/args.rs` and then blocks startup in
  `konduit-server/src/admin/service.rs` until live protocol parameters and the
  configured reference script UTxO are available.
- `konduit-cli` performs eager live reachability and network validation for the
  UTxO RPC backend when a command constructs a live connector for tip or tx
  flows.
- `show config` and `show address` commands remain config-derived and do not
  require a live backend.
- the current Blockfrost path remains weaker than UTxO RPC in two known ways:
  it still uses static per-network protocol-parameter presets and its
  `utxos_at(payment, None)` path still queries one constructed address rather
  than the broader payment-credential-wide behavior the UTxO RPC backend now
  implements.

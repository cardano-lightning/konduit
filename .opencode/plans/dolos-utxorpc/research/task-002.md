# task-002 research

- sdk transport finding: the reviewed `utxorpc` Rust SDK client builder is
  sufficient for localhost `http://` Dolos endpoints; no custom plaintext
  transport path was needed for the connector core
- utxo paging finding: `search_utxos` is token-paginated via `next_token`, so
  truthful `utxos_at(payment, None)` handling requires paging to exhaustion
  before applying the final local payment-credential filter
- predicate finding: UTxO RPC `cardano::AddressPattern` supports separate
  `payment_part` and `delegation_part`, which lets the connector bound remote
  search by payment credential alone when delegation is absent and by the exact
  pair when delegation is present
- mapping finding: `ChainUtxo` exposes both parsed protobuf outputs and
  `native_bytes`; native bytes are the highest-fidelity source and decode
  cleanly into `pallas_primitives::conway::TransactionOutput`, while parsed
  fallback is still needed when native bytes are absent
- mapping limitation: parsed fallback can map lovelace, multi-assets, datum
  bytes or datum hash, and Plutus reference scripts, but it intentionally
  rejects native reference scripts because `cardano-sdk::Output` only supports
  Plutus reference scripts today
- parameter finding: the current `utxorpc` query surface provides `read_params`
  and `read_era_summary`; those are enough to derive live fee coefficients,
  execution prices, reference-script base fee, Plutus V3 cost model, Shelley
  boundary time, and first Shelley slot without falling back to
  `ProtocolParameters::{mainnet,preprod,preview}` presets
- parameter limitation: the reviewed client helper does not expose parsed
  genesis config directly from `read_genesis`, so `start_time` is currently
  reconstructed from the Shelley era boundary and the ledger-fixed 20-second
  Byron slot length rather than from parsed genesis data
- submit finding: `submit_tx` returns the transaction reference directly; the
  current `CardanoConnector::submit() -> Result<()>` contract can truthfully
  treat successful acceptance by Dolos as success while surfacing
  endpoint-specific context on transport or backend failures
- correctness finding: parsed fallback `BigUInt` values must reject byte lengths
  larger than 8 so connector-core mapping does not silently truncate oversized
  quantities when native bytes are unavailable
- verification finding: task-002 now carries 5 focused unit tests covering
  overflow rejection, payment-only predicate shape, payment-match semantics
  independent of delegation, and protocol-parameter helper error paths; broader
  connector coverage still belongs to `task-100`

---
title: "UTxO RPC Connector Server PRD"
authors:
  - "AndrewWestberg"
created-at: 2026-09-02
status: draft
---

# Objective

Implement a separate, Mainnet-only Cardano connector server for Ferret, backed
by Dolos over UTxO RPC and compatible with the existing Cloudflare connector
API.

Dolos must remain authoritative for live Cardano state and transaction
submission. Koios may be used only to implement the existing connector's
transaction lookup and address-history responses, which UTxO RPC does not
provide directly in the complete required form. The service must not perform
automatic provider failover.

# Background

Ferret is a Kotlin Multiplatform wallet client. It does not connect directly to
Dolos, Blockfrost, Koios, or a Cardano node. Its `ConnectorClient` consumes a
strict HTTPS/JSON contract for health, network identity, protocol parameters,
address UTxOs, transaction history, and durable L1 operation submission and
lookup.

Konduit currently contains two different connector surfaces:

- `cardano-connector-utxorpc` implements the internal Rust `CardanoConnector`
  abstraction through Dolos.
- `cardano-connector-server` is a separate Cloudflare Worker backed by
  Blockfrost for reads and Koios for submission.

The existing UTxO RPC implementation supports network validation, health,
protocol parameters, current UTxO lookup, and transaction submission, but it is
not an HTTP server and does not implement Ferret's complete wire contract.

The new service must be independent of the existing Konduit adaptor runtime.

# Problem Statement

There is no server implementation in the repository that simultaneously:

- serves Ferret's strict connector JSON contract
- validates and serves Mainnet state from Dolos
- returns Bloxbean-compatible live protocol parameters
- represents all wallet UTxOs without datum or reference-script ambiguity
- provides address transaction history
- submits signed transactions without Blockfrost or Koios
- persists operation identity before submission
- reconciles indeterminate submissions across disconnects and process restarts

UTxO RPC also has no address-to-transaction-history method. A bounded,
explicitly isolated history source is required unless Konduit builds and
operates its own rollback-aware history projection.

# Goals

- Add a dedicated Rust `cardano-connector-server-utxorpc` binary crate.
- Make the new service Mainnet-only and fail closed on network mismatch.
- Use Dolos for network identity, tip, protocol parameters, UTxOs, transaction
  lookup, submission, and operation confirmation.
- Use Koios only for bounded transaction lookup and address history discovery
  and hydration.
- Preserve the complete public API exposed by the existing Cloudflare
  connector server while adding Ferret's durable L1 operation endpoints.
- Preserve Ferret's current strict JSON and finality semantics.
- Persist one operation ID to one transaction ID before mutation.
- Recover safely from request loss, process death, duplicate submission, and
  rollback.

# Non-goals

- Removing the existing Cloudflare connector server.
- Removing or changing the response semantics of existing connector-server
  compatibility endpoints.
- Removing dormant Blockfrost support from other Rust runtime surfaces.
- Adding automatic fallback between Dolos, Koios, or any other provider.
- Using Koios for UTxOs, protocol parameters, network identity, operation
  confirmation, or transaction submission.
- Implementing lease-gated channel `/submit` or `/session/claim` endpoints.
- Provisioning or deploying Dolos or the connector server.
- Configuring DNS, TLS termination, process supervision, or infrastructure.
- Modifying Ferret code, configuration, release state, or feature reachability.
- Building a local Mainnet address-history index in this iteration.
- Generalizing the service to Preprod or Preview.

# Users

Primary users:

- Ferret Mainnet wallet clients
- operators running or diagnosing the connector and Dolos
- developers maintaining Cardano provider and wire-format mappings

Secondary users:

- operators diagnosing transaction submission and reconciliation
- future implementations replacing the Koios history slice with a local Dolos
  projection

# Assumptions

- A private Mainnet Dolos endpoint is available during integration verification.
- Dolos has persistent storage and retention sufficient to reconcile an
  operation through 2160 blocks plus rollback margin.
- Dolos and the Rust UTxO RPC client use compatible protocol versions.
- Public connector reads are anonymous.
- L1 submission is authorized by the signed Cardano transaction, but still
  requires abuse controls to protect disk and upstream capacity.
- Koios queries disclose payment addresses and transaction hashes to Koios.

# Constraints

- The service is Mainnet-only and must fail startup on any other live network.
- Dolos must not be publicly exposed.
- Ferret rejects unknown JSON fields and enforces strict value bounds.
- Ferret retries GET requests once only on timeout and never retries POST
  mutations.
- Ferret accepts connector responses up to 1 MiB.
- One operation ID must resolve to one expected transaction ID permanently.
- The same transaction ID must not be accepted under several operation IDs.
- An indeterminate submission must not be classified as rejected without
  canonical chain and validity evidence.
- Existing `konduit-server` and Cloudflare connector behavior must remain
  unchanged.

# Provider Boundaries

## Dolos UTxO RPC

Dolos is authoritative for:

- live Mainnet identity
- ledger tip and block height
- current protocol parameters and era summaries
- current UTxOs
- datum and reference-script data available from the ledger
- transaction lookup by hash
- signed transaction submission
- submission acknowledgement and confirmation observation
- operation depth and rollback reconciliation

The implementation must exercise the exact required RPCs during startup or
integration verification:

- `ReadGenesis`
- `ReadParams`
- `ReadEraSummary`
- `SearchUtxos`
- `ReadTx`
- `SubmitTx`
- `WaitForTx`, where available and reliable for the pinned Dolos version

Endpoint reachability alone is not readiness.

## Koios

Koios is assigned one bounded compatibility capability:

- discover recent transaction hashes for a payment address through
  `POST /address_txs`
- hydrate those hashes and direct `/transaction/{id}` lookups through
  `POST /tx_info`

Koios must not be consulted when a Dolos request fails. A Koios failure makes
`/transaction/{id}` and `/transactions/{address}` unavailable and must not alter
the provider used by another endpoint.

Transaction depth must be computed against the Dolos tip. Koios must not submit
transactions or determine operation state.

## Future Koios Removal

Removing Koios requires a local projection that:

- consumes Dolos `FollowTip` apply, undo, and reset events
- persists address involvement, transaction content, and canonical block points
- applies rollback updates transactionally
- backfills through `DumpHistory` or an equivalent archive source
- proves response equivalence against the current Ferret history contract

That projection is deferred until eliminating Koios becomes an explicit
requirement.

# HTTP API

The first release must preserve the existing Cloudflare connector API and add
the durable operation endpoints required by Ferret:

| Method | Path                         | Source                                        |
| ------ | ---------------------------- | --------------------------------------------- |
| `GET`  | `/`                          | generated API documentation                   |
| `GET`  | `/openapi.yaml`              | checked-in OpenAPI document                   |
| `GET`  | `/health`                    | Dolos, Koios, and local database readiness    |
| `GET`  | `/network`                   | fixed Mainnet identity verified against Dolos |
| `GET`  | `/protocol-parameters`       | Dolos                                         |
| `GET`  | `/balance/{address}`         | Dolos                                         |
| `GET`  | `/utxos_at/{address}`        | Dolos                                         |
| `GET`  | `/transaction/{id}`          | Koios candidate cross-checked through Dolos   |
| `GET`  | `/transactions/{address}`    | Koios candidates cross-checked through Dolos  |
| `POST` | `/submit`                    | legacy compatibility submission through Dolos |
| `POST` | `/operations`                | redb and Dolos submission                     |
| `GET`  | `/operations/{operation_id}` | redb and Dolos reconciliation                 |

The OpenAPI document must use `additionalProperties: false` and match Ferret's
snake_case field names, lowercase-hex rules, decimal-string quantities, bounds,
and optionality.

# Functional Requirements

## API Documentation

`GET /` must render API documentation from the checked-in OpenAPI contract.
`GET /openapi.yaml` must return that contract. Both routes must remain available
without provider credentials and must describe the Mainnet-only service.

## Health

`GET /health` must return exactly:

```json
{ "status": "ok" }
```

with HTTP 200 only when:

- the operation database opened successfully
- Dolos is reachable
- Dolos reports Mainnet
- Koios reports Mainnet matching Dolos
- a fresh ledger point is available
- all required protocol parameters are available and representable
- the Koios transaction lookup and history dependency is reachable

Otherwise it must return a bounded non-2xx response. Public errors must not
include internal URLs, provider bodies, credentials, or signed transaction
data.

## Network

`GET /network` must return exactly:

```json
{ "network": "mainnet" }
```

The process must fail before binding its public socket if Dolos genesis does not
identify Mainnet.

## Protocol Parameters

`GET /protocol-parameters` must return:

- current era
- current epoch
- current slot
- a Bloxbean-compatible JSON `payload`

For the initial plain L1 transfer path, the payload must define exact types and
live values for at least:

- `min_fee_a`: JSON number
- `min_fee_b`: JSON number
- `max_tx_size`: JSON number
- `key_deposit`: decimal string
- `pool_deposit`: decimal string
- `min_pool_cost`: decimal string
- `protocol_major_ver`: JSON number
- `protocol_minor_ver`: JSON number
- `coins_per_utxo_size`: decimal string
- `collateral_percent`: JSON number
- `max_collateral_inputs`: JSON number

Every value must come from Dolos. The endpoint must fail if a required value is
missing, out of range, or cannot be represented exactly. Static Mainnet
parameter defaults are prohibited.

Script execution prices and cost models are deferred unless they are made
mandatory and verified before channel transaction construction is enabled.

## Balance

`GET /balance/{address}` must preserve the existing connector-server contract:

- accept only a valid Mainnet Shelley address
- query the exact address through Dolos
- sum the lovelace quantity across all returned UTxOs with checked arithmetic
- return `{"lovelace":"0"}` when no UTxOs exist
- return the quantity as a decimal string

## UTxOs

`GET /utxos_at/{address}` must:

- accept only a valid Mainnet Shelley address
- query the exact decoded address bytes through Dolos; credential-pair lookup is
  insufficient for enterprise and pointer addresses
- return transaction ID, output index, full address, and all asset values
- preserve datum hash and inline datum as separate fields
- return a reference-script hash for native and Plutus reference scripts
- reject duplicate assets, invalid quantities, malformed hashes, and oversized
  fields
- exhaust Dolos pagination and return the complete UTxO set, including addresses
  with more than 100 UTxOs
- return a bounded non-2xx response only when the serialized response would
  exceed Ferret's 1 MiB response limit

The server must map directly from the UTxO RPC provider structures needed by the
wire contract. It must not force the response through `cardano_sdk::Output`,
which cannot currently represent every native reference script.

An unsupported output must fail explicitly rather than disappear silently.

## Transaction History

`GET /transactions/{address}` must:

- request the newest 100 transaction hashes from Koios `/address_txs`
- hydrate hashes in bounded batches through `/tx_info`
- cross-check every Koios row against Dolos `ReadTx` and canonical chain data;
  never return a Koios-only inclusion
- deduplicate transaction hashes
- map block transaction index, timestamp, validity interval, inputs, outputs,
  assets, datum metadata, and reference-script hashes
- filter reference inputs and collateral according to successful or failed
  phase-2 semantics
- discard a transaction if the queried address no longer appears in its
  effective inputs or outputs after filtering
- compute depth from the current Dolos height and the transaction block height
- clamp inconsistent ahead-of-tip rows to pending and log a sanitized provider
  inconsistency
- return no more than 1000 records or 1 MiB

Koios timeout, throttling, schema drift, or unavailability must return 503. The
service must not switch any other endpoint to Koios.

The complete `/transactions/{address}` route deadline must be less than
Ferret's 20-second request timeout.

Dolos retention must cover every transaction Koios can return through this API,
including direct transaction-ID lookup and the newest 100 transactions of an
inactive address. Deployments must use archival retention for that horizon. A
deployment with less retention must reject startup rather than silently omit
Koios rows.

## Transaction by ID

`GET /transaction/{id}` must preserve the existing connector-server contract:

- accept only a 64-character lowercase transaction hash
- return one transaction using the same mapping and effective input/output
  semantics as `/transactions/{address}`
- compute depth against the current Dolos tip
- return JSON `null` when the transaction does not exist
- enforce the same field and response-size bounds as address history

Koios `/tx_info` is part of the same explicitly bounded history capability as
`/transactions/{address}`. It must not become a general fallback for Dolos.

## Legacy Submission

`POST /submit` must preserve the existing Cloudflare connector contract for
clients that send `{"transaction":"<lowercase-cbor-hex>"}` and expect
`{"transaction_id":"<lowercase-transaction-id>"}`.

The handler must validate and hash the signed CBOR, submit through Dolos, and
reuse the operation store under an internal transaction-derived key so repeated
submission of identical bytes remains idempotent. This compatibility route is
not the future lease-gated Ferret channel submission contract.

Legacy transactions without a validity upper bound remain eligible for
reconciliation for at most one hour from first persistence. If no canonical
inclusion is found by then, the server marks the internal record rejected,
deletes its CBOR, and releases pending capacity.

# Durable L1 Operations

## Public Contract

`POST /operations` accepts:

- `operation_id`: UUID
- `expected_transaction_id`: 64-character lowercase hex
- `transaction`: signed transaction CBOR as lowercase hex

`GET /operations/{operation_id}` returns the stored public operation state.

The client must persist the originating connector URL with each operation and
send all retries and status reads to that same connector. A reverse proxy must
provide sticky routing to the single process that owns the redb operation
database; multi-process deployments require shared operation storage.

Public states are:

- `pending`
- `accepted`
- `confirmed`
- `settled`
- `rejected`

Depth must be non-negative. A returned `transaction_id`, when present, must equal
`expected_transaction_id`.

## Validation

Before persistence or submission, the server must:

- enforce an HTTP body limit before JSON parsing or hex decoding
- decode the signed CBOR
- compute the Cardano transaction body hash
- require the computed hash to equal `expected_transaction_id`
- reject CBOR larger than the live Cardano `max_tx_size`
- require a bounded transaction validity upper bound (TTL), and reject when the
  current tip slot is greater than or equal to it

The operation record also stores a monotonic revision and a submission-lease
timestamp. Updates are compare-and-set against the observed revision, and an
abandoned submission may be retried only after its lease expires.

## Persistent Schema

Use redb with at least two tables:

```text
operations: operation_id -> operation record
transaction_ids: expected_transaction_id -> operation_id
```

An operation record must retain:

- operation ID
- expected transaction ID
- signed-CBOR digest
- signed CBOR until the operation is settled or rejected
- validity upper bound
- internal state
- attempt timestamps and bounded retry metadata
- inclusion block point when known

Compact terminal operation mappings must remain available for offline Ferret
clients. Capacity must be managed through admission limits and volume planning,
not by silently deleting records that clients may still reconcile.

## Idempotency

- The first accepted UUID and transaction pair must be persisted before
  submission.
- Repeating the same UUID and identical transaction must return the existing
  operation.
- Reusing the UUID with different bytes or transaction ID must return 409.
- Reusing the same transaction ID under another UUID must return 409.
- The transition from internal `prepared` to `submitting` must be claimed
  atomically.
- Only one submission attempt may be in flight for an operation.

## Reconciliation

The server must query `ReadTx(expected_transaction_id)` before retrying an
indeterminate submission.

Duplicate, already-known, or inputs-spent responses after an indeterminate first
submission are not proof of rejection. The server must keep the operation
pending until chain evidence or the transaction validity bound resolves it.

The server may resubmit only the identical persisted CBOR. It must retain that
CBOR until the operation is settled or rejected.

State mapping:

- internal `prepared` or `submitting` maps to public `pending`
- successful Dolos submission returning the expected hash maps to `accepted`
- a definitive Dolos `SubmitTx` rejection maps to `rejected`
- canonical inclusion with depth below 5 maps to `accepted`
- depth at least 5 maps to `confirmed`
- depth at least 2160 maps to `settled`
- rollback before settlement returns the operation to `pending`
- canonical absence through the transaction validity bound maps to `rejected`
- `settled` is terminal and never returns to `prepared` or `pending`

The app maps both `accepted` and `confirmed` to its `Confirming` state and
continues reconciliation. Only `settled` maps to the app's terminal `Confirmed`
state; `rejected` maps to `Failed`.

`GET /operations/{id}` may refresh and persist status from Dolos. It must never
create or submit an operation.

# Non-functional Requirements

- Predictable startup and fail-closed Mainnet validation.
- Backend timeouts shorter than Ferret's 20-second request timeout.
- At most 1 MiB per response.
- Bounded Koios batch size and concurrency.
- One active redb writer instance.
- Persistent operation storage across process and machine restart.
- Structured, sanitized dependency and operation-state logs.
- No address, signed CBOR, operation body, credential, or upstream body logging.
- Read endpoints must remain available under submission overload.
- Existing `konduit-server` runtime behavior must be unaffected.

# Security Requirements

- The public API must not expose or proxy the private Dolos gRPC service.
- Public submission requires per-source rate limiting, a global in-flight limit,
  a maximum pending-operation count, a hard database capacity threshold, and
  transaction-ID deduplication.
- Capacity exhaustion must reject new operations without degrading reads.
- Provider and database errors must not expose internal topology.

# Workstreams

## Workstream 1: Contract and provider fixtures

Tasks:

- encode the connector contract in OpenAPI
- capture valid and invalid strict JSON fixtures
- verify Koios effective transaction mapping
- verify Dolos UTxO completeness above 100 outputs

Definition of done:

- server responses validate against the OpenAPI contract
- malformed, extra, or incorrectly encoded fields fail
- failed-script transaction semantics match the current contract
- provider limitations are explicit before server implementation

## Workstream 2: UTxO RPC extensions

Tasks:

- expose live tip, raw parameters, era summary, transaction lookup, and raw-CBOR
  submission as inherent UTxO RPC methods
- preserve the existing `CardanoConnector` trait
- implement provider-specific UTxO wire mapping
- implement exact Bloxbean protocol-parameter mapping

Definition of done:

- existing UTxO RPC connector consumers remain unchanged
- all connector UTxO and parameter fixtures are derived from Dolos data

## Workstream 3: Public HTTP service

Tasks:

- create the Rust binary crate and Actix routes
- implement configuration and startup readiness
- implement health, network, protocol parameter, balance, UTxO, single
  transaction, address history, legacy submit, and operation endpoints
- add bounds, timeouts, error translation, and sanitized logs

Definition of done:

- the service satisfies the complete OpenAPI contract against Mainnet Dolos and
  the bounded Koios transaction lookup and history capability

## Workstream 4: Durable operation service

Tasks:

- add redb operation and transaction-ID tables
- validate and hash signed CBOR
- implement atomic idempotency and submission claims
- implement startup and periodic reconciliation
- implement rollback, expiry, and finality transitions
- add public-write admission controls

Definition of done:

- one operation and transaction pair remains stable across duplicates,
  disconnects, crashes, and rollback

# Task Breakdown

Suggested execution order:

1. freeze OpenAPI and connector contract fixtures
2. prove Koios transaction lookup/history semantics and Dolos UTxO completeness
3. expose the narrow UTxO RPC methods
4. implement provider-specific UTxO and protocol mappings
5. implement the complete public API Actix routes
6. implement redb operation persistence and idempotency
7. implement reconciliation and finality state transitions
8. add submission admission and capacity controls
9. run repository and provider integration verification

# Risks

- Koios transaction lookup and history create address privacy and availability
  dependencies.
- Koios schema drift may break Ferret's strict transaction DTO.
- Current Dolos `SearchUtxos` pagination behavior may truncate large address
  result sets unless explicitly verified or corrected.
- UTxO RPC alpha/beta protocol compatibility may change; exact versions must be
  pinned and exercised.
- Native reference scripts do not fit the current `cardano_sdk::Output` model.
- Datum-hash-only outputs are unsafe with Ferret's current DTO.
- A lost submit response can be mistaken for rejection unless chain lookup
  precedes retry.
- An unauthenticated Mainnet submission route can be abused as a relay or for
  disk exhaustion.
- redb requires a single active writer and durable volume ownership.
- Provider integration verification uses Mainnet data and must not submit a
  transaction unless the operator explicitly supplies a controlled test case.

# Acceptance Criteria

- A separate `cardano-connector-server-utxorpc` binary builds and runs.
- Startup rejects non-Mainnet Dolos data.
- Dolos is the source for all live Cardano state and submission.
- Koios is called only by the transaction lookup and address-history
  implementations.
- The existing Cloudflare `/balance`, `/transaction/{id}`, and `/submit`
  contracts remain available through the new service.
- Every public response matches the existing connector-server and durable
  operation wire contracts.
- Protocol parameters match the documented Bloxbean JSON fixture.
- UTxO responses preserve datum hash, inline datum, assets, and native/Plutus
  reference-script hashes.
- An address with more than 100 UTxOs returns a complete result.
- Transaction history preserves effective success/failure semantics and finality
  depth.
- Duplicate and concurrent operation requests submit no distinct transaction.
- Indeterminate operations remain recoverable through crash and rollback.
- Public operation abuse cannot exhaust storage or starve read endpoints within
  configured limits.

# Verification Plan

Repository verification:

```console
cargo test -p cardano-connector-utxorpc
cargo test -p cardano-connector-server-utxorpc
cargo check -p cardano-connector-server-utxorpc
```

Behavioral verification must include:

- strict OpenAPI/JSON fixture compatibility
- Dolos Mainnet mismatch startup failure
- Koios timeout, throttle, malformed body, and schema-drift failures
- native reference script and datum-hash-only UTxOs
- more than 100 UTxOs at one address
- zero and non-zero `/balance/{address}` responses
- found and missing `/transaction/{id}` responses
- repeated identical legacy `/submit` requests
- concurrent duplicate submissions
- conflicting UUID and transaction-ID reuse
- crash before submission
- crash after upstream acceptance
- duplicate/already-known upstream responses
- validity expiry without inclusion
- rollback before settlement
- exact depth 4, 5, 2159, and 2160 boundaries
- redb reopen and capacity limits
- overload isolation between write and read routes

# Open Questions

- Whether Dolos `SearchUtxos` returns complete results above its requested page
  size in the pinned release.
- Which exact Dolos and UTxO RPC protocol versions will be pinned for production.
- What fixed operation-store capacity and pending-operation admission limits fit
  the production volume.
- What retention beyond compact terminal mappings is required by support and
  audit policy.
- Whether a later project should replace Koios history with a local Dolos Sync
  projection.

# References

- `docs/design/33_cardano_connector.md`
- `docs/design/36_dolos_utxorpc_implementation_prd.md`
- UTxO RPC specification: <https://github.com/utxorpc/spec>
- Dolos documentation: <https://docs.txpipe.io/dolos/what>
- Koios OpenAPI: <https://api.koios.rest/koiosapi.yaml>

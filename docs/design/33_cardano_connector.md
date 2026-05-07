---
title: "Cardano Connector"
---

Cardano Connector (CaCo) is the boundary between Cardano L1 data and the rest of
Konduit.

The key design point is that Konduit business logic should depend on a compact,
well-defined connector contract rather than on a specific Cardano provider such
as Blockfrost, Ogmios, or Dolos.

# Purpose

The connector is responsible for giving Konduit the Cardano information it needs
to:

- inspect relevant UTxOs
- build transactions with correct protocol parameters
- submit transactions
- detect health and network mismatches early

For the adaptor deployment currently targeted by this work, the selected backend
is a local `dolos` instance accessed through `UTxO RPC`.

# Current state

The repository currently contains a direct Blockfrost integration path. That is
a legitimate backend option and provided an initial implementation of the
connector boundary.

The broader project direction is not to replace one universal Cardano provider
with another, but to support multiple Cardano connectivity options behind a
stable connector abstraction.

For the implementation effort currently targeted by the companion ADR and PRDs,
backend parity is required only across the Rust runtime surfaces that currently
instantiate or configure the direct Blockfrost connector:

- `konduit-server`
- `konduit-cli`
- the shared connector implementation layer

This document does not imply repo-wide backend parity for unrelated repository
subprojects.

# Target state

The target state for the next specific implementation effort is:

- `konduit-server` uses a Cardano connector selected by configuration.
- `konduit-cli` uses the same backend selection model for the runtime flows it
  already supports.
- the selected production backend is `dolos` over `UTxO RPC`.
- `dolos` runs locally and syncs from either the operator's same-host
  `cardano-node` or an external relay, with same-host upstream preferred for the
  primary deployment profile.
- backend traffic remains localhost-only.

# Connector contract

The adaptor currently needs a connector that can supply at least:

- `network()`
- `health()`
- `protocol_parameters()`
- `utxos_at(payment, delegation)`
- `submit(tx)`

These functions are sufficient for the current adaptor runtime and provide a
clear seam for multiple backend implementations.

For this implementation effort, backend selection is required where Rust runtime
code currently instantiates or configures a concrete provider. Generic crates
that only depend on `CardanoConnector` are not themselves parity targets unless
they currently construct the direct Blockfrost connector.

Connector semantics for this effort:

- `utxos_at(payment, Some(delegation))` means UTxOs at addresses matching that
  specific payment and delegation pair.
- for the UTxO RPC backend, `utxos_at(payment, None)` means any UTxO whose
  address shares the given payment credential, regardless of delegation.

Current implementation note:

- `cardano-connector-utxorpc` satisfies the intended semantics by paging UTxOs
  by payment credential and applying delegation filtering locally when
  `delegation` is present.
- the still-supported direct Blockfrost path does not currently have the same
  `utxos_at(payment, None)` behavior because it queries one constructed address.
  Treat that as a known limitation of the current Blockfrost connector, not as
  documented parity.

# Backend options

## Blockfrost

Pros:

- simple initial implementation
- easy to use for development

Cons:

- external hosted dependency
- credential management burden
- weaker fit for a self-hosted adaptor deployment
- current direct implementation still uses static per-network protocol parameter
  presets and narrower `utxos_at(payment, None)` semantics than the UTxO RPC
  connector contract intends

## Dolos MiniBF

Pros:

- smaller migration from Blockfrost-shaped code
- can still run locally

Cons:

- keeps the integration conceptually tied to Blockfrost semantics
- does not exercise the UTxO RPC path chosen for this implementation effort

## Dolos UTxO RPC

Pros:

- typed machine-to-machine interface
- good long-term separation of concerns
- local and self-hosted production fit

Cons:

- larger implementation effort up front
- requires explicit mapping into Konduit's `cardano-sdk` types

# Why UTxO RPC for this effort

`UTxO RPC` is the selected Cardano boundary for this implementation effort
because it lets Konduit integrate with a local Cardano service using a typed
interface that is not tied to one vendor's HTTP surface.

This should not be read as excluding other connector implementations. The
project-level direction remains support for multiple Cardano backends selected
by configuration and deployment needs.

This fits the adaptor service model well:

- Konduit remains the public application service.
- Dolos becomes a private Cardano infrastructure service.
- backend boundaries are explicit and localhost-only.

# Data requirements

The connector must provide enough information to map into existing `cardano-sdk`
types used by Konduit. In practice this means careful handling of:

- addresses and credentials
- lovelace and multi-asset values
- datum and datum hash information
- script or reference-script related fields, where relevant
- protocol parameters used by transaction builders

This mapping work is the main technical risk in the UTxO RPC integration.

For the UTxO RPC backend, the authoritative sources of these facts must be UTxO
RPC modules only. The backend should derive what it needs from UTxO RPC queries,
including live chain parameters and related Cardano facts, rather than falling
back to local genesis files, `cardano-node` artifacts, or static per-network
presets inside Konduit.

# Configuration

The server and CLI support selecting a Cardano backend explicitly. The current
runtime shape is:

- backend kind, e.g. `blockfrost` or `utxorpc`
- backend endpoint, e.g. Dolos gRPC address for `utxorpc`
- explicit network selection for `utxorpc`, cross-checked against live provider
  data
- Blockfrost project id for `blockfrost`

Current config/runtime notes:

- `konduit-server` uses `KONDUIT_CARDANO_BACKEND`,
  `KONDUIT_BLOCKFROST_PROJECT_ID`, `KONDUIT_UTXORPC_URI`, and `KONDUIT_NETWORK`.
- `konduit-cli` uses the same backend env vars, with dotenv precedence of CLI
  args, exported env vars, `.env.<role>`, then `.env`.
- parsed CLI `utxorpc` config requires explicit `KONDUIT_NETWORK`, while live
  UTxO RPC connector use for tip and tx flows also requires
  `KONDUIT_UTXORPC_URI`.
- the direct Blockfrost path still allows network inference or defaulting from
  the project id, so its config behavior is intentionally not identical to the
  UTxO RPC path.

For this effort, that explicit backend selection is required in the Rust runtime
surfaces that currently expose direct Blockfrost configuration:

- `konduit-server`
- `konduit-cli`

Unrelated repository subprojects, such as `cardano-connector-server`, are out of
scope for this backend parity requirement.

# Trust model

The adaptor operator trusts their local Cardano infrastructure enough to use it
for query and submission. This is a different trust model from mobile consumer
clients choosing their own Cardano connector.

For the adaptor runtime here:

- Dolos is trusted as the Cardano service boundary
- Dolos is not exposed publicly
- network and health mismatches should fail early at startup where possible
- the configured network should be explicit in Konduit config and cross-checked
  against live data from Dolos
- Konduit startup with the UTxO RPC backend depends on Dolos successfully
  serving `read_genesis` so the live network can be derived before startup
  continues

# Failure handling

The connector should fail clearly in the following cases:

- Dolos is unreachable
- Dolos is on the wrong network
- protocol parameters are unavailable or incomplete
- the configured reference script UTxO cannot be resolved at startup
- UTxO query responses cannot be mapped into Konduit types
- transaction submission fails or is rejected

For the UTxO RPC backend, these are startup blockers for Rust runtime surfaces
that need the backend to be ready before serving traffic or executing runtime
flows.

Current runtime split:

- `konduit-server/src/cardano/args.rs` checks backend construction,
  reachability, and UTxO RPC live-network match.
- `konduit-server/src/admin/service.rs` remains the startup blocker for live
  protocol-parameter derivation and reference-script resolution.
- `konduit-cli` performs the same live reachability and network validation only
  for the UTxO RPC backend when a command constructs a live connector. Commands
  such as `show config` and `show address` remain config-derived and can run
  without a live backend.
- the current direct Blockfrost CLI path validates project-id presence and
  network-prefix consistency, but does not perform the same eager live
  validation pass before connector use.

# Testing

At minimum, backend work should include:

- unit tests for data mapping
- connector-level tests for health, params, UTxO queries, and submit
- manually driven integration checks against a real local Dolos instance

# Related documents

- [Architecture](./20_architecture.md)
- [Adaptor Deployment](./35_adaptor_deployment.md)
- [Dolos UTxO RPC Implementation PRD](./36_dolos_utxorpc_implementation_prd.md)
- [Adaptor Deployment PRD](./37_adaptor_deployment_prd.md)
- [ADR: Dolos UTxO RPC Adaptor Backend](../adrs/06-dolos-utxorpc-adaptor-backend.md)

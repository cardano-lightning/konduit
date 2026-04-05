---
title: "Cardano Connector"
---

Cardano Connector (CaCo) is the boundary between Cardano L1 data and the rest
of Konduit.

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

# Target state

The target state for the next specific implementation effort is:

- `konduit-server` uses a Cardano connector selected by configuration.
- the selected production backend is `dolos` over `UTxO RPC`.
- `dolos` runs locally and connects to the operator's local `cardano-node`.
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

# Backend options

## Blockfrost

Pros:

- simple initial implementation
- easy to use for development

Cons:

- external hosted dependency
- credential management burden
- weaker fit for a self-hosted adaptor deployment

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
project-level direction remains support for multiple Cardano backends selected by
configuration and deployment needs.

This fits the adaptor service model well:

- Konduit remains the public application service.
- Dolos becomes a private Cardano infrastructure service.
- backend boundaries are explicit and localhost-only.

# Data requirements

The connector must provide enough information to map into existing
`cardano-sdk` types used by Konduit. In practice this means careful handling of:

- addresses and credentials
- lovelace and multi-asset values
- datum and datum hash information
- script or reference-script related fields, where relevant
- protocol parameters used by transaction builders

This mapping work is the main technical risk in the UTxO RPC integration.

# Configuration

The server should support selecting a Cardano backend explicitly. A likely shape
is:

- backend kind, e.g. `blockfrost` or `utxorpc`
- backend endpoint, e.g. Dolos gRPC address
- network selection where automatic detection is insufficient

# Trust model

The adaptor operator trusts their local Cardano infrastructure enough to use it
for query and submission. This is a different trust model from mobile consumer
clients choosing their own Cardano connector.

For the adaptor runtime here:

- Dolos is trusted as the Cardano service boundary
- Dolos is not exposed publicly
- network and health mismatches should fail early at startup where possible

# Failure handling

The connector should fail clearly in the following cases:

- Dolos is unreachable
- Dolos is on the wrong network
- protocol parameters are unavailable or incomplete
- UTxO query responses cannot be mapped into Konduit types
- transaction submission fails or is rejected

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

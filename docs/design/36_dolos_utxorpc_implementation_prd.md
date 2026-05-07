---
title: "Dolos UTxO RPC Implementation PRD"
authors:
  - "AndrewWestberg"
created-at: 2026-04-05
status: draft
---

# Objective

Implement a production-grade Cardano backend for the Rust runtime surfaces that
currently use direct Blockfrost connectivity, using a local `dolos` instance
over `UTxO RPC`.

# Background

Konduit already has a Cardano connector abstraction, but the adaptor runtime is
currently centered on a direct Blockfrost-backed implementation. The target
operator deployment is a single Ubuntu server running local Bitcoin, Lightning,
and Cardano infrastructure under `systemd`, with anonymous public consumer API
traffic coming mostly from mobile wallet applications.

The project direction is to support multiple Cardano connectivity options behind
a stable connector boundary. This PRD covers one concrete implementation of that
direction: integrating Konduit with local Cardano infrastructure through
`dolos` and `UTxO RPC`.

# Problem Statement

The Rust runtime surfaces currently using direct Blockfrost do not yet have a
UTxO RPC backend path that can:

- use a local `dolos` service over gRPC
- query protocol parameters and UTxOs through UTxO RPC
- submit Cardano transactions through UTxO RPC
- be configured as a first-class backend beside the existing Blockfrost path in
  the Rust runtime surfaces in scope

# Goals

- Add a new UTxO RPC Cardano connector implementation.
- Allow the Rust runtime surfaces that currently use direct Blockfrost to select
  that backend through configuration.
- Keep the implementation aligned with the existing connector abstraction.
- Support single-host production deployment with localhost-only Cardano access.
- Provide sufficient tests and documentation for future maintenance.

# Non-goals

- Replacing `cardano-node`
- Replacing `Ogmios` or `Kupo`
- Generalizing Konduit to every possible Cardano provider in this phase
- Repo-wide backend parity for unrelated repository subprojects
- Changing the public consumer API surface unless implementation needs force it
- Building automated deployment orchestration beyond documented manual rollout

# Assumptions

- `dolos` runs on the same host as Konduit.
- `dolos` may sync from the same-host `cardano-node` or an external relay,
  depending on operator choice.
- `lnd` REST is already enabled on localhost.
- public clients are mostly mobile wallet apps.
- public access to the consumer API is anonymous.
- admin endpoints remain non-public.

# Constraints

- Minimize disruption to existing Konduit architecture.
- Preserve the Cardano connector abstraction.
- Respect the repo's role-oriented and connector-oriented direction.
- Avoid introducing production dependence on Blockfrost.

# Scope

In scope for backend parity in this phase:

- `konduit-server`
- `konduit-cli`
- the shared connector implementation layer used by those Rust runtime surfaces

Out of scope for backend parity in this phase:

- `cardano-connector-server`
- unrelated repository subprojects
- generic crates that already depend only on `CardanoConnector` and do not
  instantiate or configure Blockfrost directly

# Users

Primary users:

- adaptor operators deploying Konduit on real infrastructure
- future developers extending or debugging Cardano backends

Secondary users:

- future agent sessions performing implementation increments

# Functional Requirements

## Backend Selection

The Rust runtime surfaces that currently instantiate or configure the direct
Blockfrost connector must support selecting a Cardano backend that targets UTxO
RPC.

For this phase, that means at minimum:

- `konduit-server`
- `konduit-cli`

## Cardano Network

The backend must determine or be configured with the intended Cardano network in
a way that maps correctly to Konduit's `cardano_sdk::Network` types.

For this phase, the intended network is explicit Konduit configuration and must
be cross-checked against live data available through UTxO RPC.

Current implementation note:

- live network derivation depends on Dolos successfully serving `read_genesis`.
- `network_magic` is the authoritative discriminator, while Dolos `network_id`
  casing is tolerated.

## Health

The backend must expose a meaningful health signal based on reachability and
basic correctness of the local Dolos service.

The backend must fail startup unless:

- Dolos is reachable
- the configured network matches live data from Dolos
- live protocol parameters required for transaction building can be derived from
  UTxO RPC
- the configured reference script UTxO can be resolved

## Protocol Parameters

The backend must obtain protocol parameters sufficient for Konduit transaction
building.

For the UTxO RPC backend, these parameters must be derived from UTxO RPC modules
only. Konduit should not fall back to local genesis files, `cardano-node`
artifacts, or static per-network presets for this backend.

## UTxO Lookup

The backend must retrieve UTxOs by address or credential patterns in a form that
can be mapped into the types used by existing Konduit logic.

Semantics:

- `utxos_at(payment, Some(delegation))` matches that specific payment and
  delegation pair.
- for the UTxO RPC backend, `utxos_at(payment, None)` matches any UTxO whose
  address shares the given payment credential, regardless of delegation.

Current implementation note:

- `cardano-connector-utxorpc` achieves the exact pair semantics by paging on the
  payment credential and applying delegation filtering locally.
- the direct Blockfrost implementation still does not satisfy the broader
  payment-credential-wide `utxos_at(payment, None)` behavior; that remains a
  known non-parity point outside this task's code-cleanup scope.

## Transaction Submission

The backend must submit Cardano transactions through Dolos and return success or
failure with actionable error information.

# Non-functional Requirements

- Localhost-only backend traffic
- Predictable startup and failure behavior
- Good logging at the backend boundary
- Clear configuration errors when Dolos is unreachable or misconfigured
- Sufficient tests for type conversion and connector behavior
- Clear startup failure reasons for network mismatch, missing live protocol
  parameters, or missing reference script UTxO

# Security Requirements

- Do not expose Dolos publicly for this integration.
- Do not require Blockfrost credentials.
- Do not broaden LND privileges as part of Cardano backend work.
- Avoid accidental leakage of internal topology details through public errors.

# Workstreams

## Workstream 1: Connector contract review

Purpose:

- confirm the current `CardanoConnector` contract is sufficient
- identify any gaps before implementation begins

Tasks:

- inspect the existing trait and all current call sites
- confirm data required by `konduit-server` handlers
- document any trait changes if unavoidable

Definition of done:

- the team can state whether the existing trait is sufficient as-is

## Workstream 2: New UTxO RPC connector crate

Purpose:

- introduce a dedicated crate, likely `cardano-connector-utxorpc`

This crate is the shared implementation layer for the Rust runtime surfaces in
scope. It is not a requirement of this phase to retrofit unrelated repository
subprojects onto that backend.

Tasks:

- add client dependency on UTxO RPC Rust tooling
- implement connector construction and config parsing
- implement `health`, `network`, `protocol_parameters`, `utxos_at`, and
  `submit`

Definition of done:

- the crate compiles and satisfies the connector contract

## Workstream 3: Data mapping

Purpose:

- map UTxO RPC Cardano structures into Konduit's `cardano-sdk` structures

Tasks:

- map outputs, values, multi-assets, datums, and script-relevant fields
- map protocol parameters required by Konduit tx builders
- explicitly document unsupported or deferred fields if any
- confirm that the data required for live protocol parameters can be derived from
  UTxO RPC modules only

Definition of done:

- connector responses are consumable by current Konduit flows

## Workstream 4: `konduit-server` integration

Purpose:

- make the new backend selectable and usable in the adaptor runtime

Tasks:

- add CLI/env/config for UTxO RPC endpoint and explicit network selection
- wire backend selection in server bootstrap
- add startup validation and useful logs
- fail startup on reachability, network, live-parameter, or reference-script
  validation failures

Definition of done:

- `konduit-server` can boot against Dolos and pass basic health checks

## Workstream 4b: `konduit-cli` integration

Purpose:

- make the new backend selectable anywhere the CLI currently configures or
  instantiates the direct Blockfrost connector

Tasks:

- add CLI/env/config for backend kind, UTxO RPC URI, and explicit network
- wire runtime enum selection in CLI connector construction
- ensure current admin, adaptor, and consumer flows can use either backend
- ensure runtime flows fail clearly when startup-equivalent backend validation
  cannot be satisfied

Current implementation note:

- UTxO RPC commands perform eager connector reachability and live-network
  validation during connector construction for live tip and tx flows.
- the current Blockfrost path validates project-id presence and network-prefix
  consistency, but otherwise fails lazily on later API use rather than through
  the same eager live-validation path.

Definition of done:

- `konduit-cli` can run its current Blockfrost-backed runtime flows with either
  backend selection

## Workstream 5: Tests

Purpose:

- reduce integration risk for the new backend

Tasks:

- add unit tests for type conversion
- add connector tests for query/submit behavior where practical
- add at least one manually driven integration path documented for operators

Definition of done:

- failures in mapping or backend configuration are caught early

## Workstream 6: Documentation and rollout readiness

Purpose:

- make future implementation and operations coherent

Tasks:

- update design docs and deployment docs
- ensure config surface is documented
- record known limits and follow-up work

Current known limits to keep explicit:

- UTxO RPC delegated lookup currently depends on payment-credential paging plus
  local delegation filtering rather than a guaranteed Dolos-side delegation
  index.
- the direct Blockfrost path remains available in parallel, but still differs in
  protocol-parameter sourcing and `utxos_at(payment, None)` behavior.

Definition of done:

- future agent sessions can continue the work without rediscovering context

# Task Breakdown

Suggested execution order:

1. inspect trait and call sites
2. create connector crate skeleton
3. wire network and health behavior
4. wire protocol parameter retrieval
5. implement UTxO search and mapping
6. implement transaction submission
7. integrate backend selection into `konduit-server`
8. integrate backend selection into `konduit-cli`
9. add tests and docs

# Risks

- UTxO RPC response shapes may not line up perfectly with Konduit's current
  assumptions.
- Protocol parameter mapping may need more fields than initially expected.
- UTxO RPC may not expose enough information to fully satisfy Konduit's current
  transaction-building needs without changing the connector shape or adjacent
  code.
- Transaction submission errors may require translation to fit current server
  behavior.
- The existing connector trait may expose Blockfrost-era assumptions.

# Acceptance Criteria

- `konduit-server` can start with a UTxO RPC backend configuration.
- `konduit-cli` can execute its current direct-Blockfrost-backed runtime flows
  with a UTxO RPC backend configuration.
- The server can query Dolos for health and Cardano state over localhost.
- The server can obtain UTxOs needed for its Cardano flows.
- The server can submit Cardano transactions through Dolos.
- The new backend is documented for the Rust runtime surfaces in scope.

# Verification Plan

Minimum verification:

- build the relevant Rust workspace targets
- run unit tests for the new connector and mapping code
- manually validate startup against a local Dolos instance
- manually validate at least one end-to-end Cardano submission path

# Rollout Plan

- land connector implementation behind explicit config selection
- validate on the target production-style host
- switch the adaptor deployment to the UTxO RPC backend only after health and
  submission checks succeed

# Rollback Plan

- preserve the prior Konduit binary and config
- if the new backend fails, restore the previous binary and backend config
- do not remove Dolos merely because Konduit rollback is required

# Open Questions

- whether the current connector trait needs extension for richer health data
- whether a local test harness for Dolos should be introduced later

---
title: Dolos UTxO RPC Adaptor Backend
status: accepted
authors:
  - "@OpenCode"
date: 2026-04-05
tags:
  - adaptor
  - cardano
  - connector
  - dolos
  - deployment
  - lnd
  - utxorpc
---

## Context

Konduit currently has a Cardano integration path centered on Blockfrost. The
broader direction of the project is to support multiple Cardano connectivity
options behind a stable connector boundary, including options such as
Blockfrost, Ogmios and Kupo, Dolos, and others.

This ADR does not change that broader direction. Instead, it records the backend
choice for one specific implementation and deployment effort.

For the target deployment discussed here, the operator already runs:

- `bitcoind` on the same Ubuntu 24.04 host.
- `lnd` on the same host, managed by `systemd`.
- `cardano-node` on the same host, managed independently.
- `Ogmios` and `Kupo` for other workloads, but they are not part of this
  project scope.

The desired runtime shape is:

- public consumer API for mobile wallet clients.
- localhost-only access to backend infrastructure.
- no dependency on hosted third-party Cardano infrastructure for this production
  deployment.
- a typed localhost Cardano integration path for this deployment.
- a least-privilege integration with `lnd`.

Dolos provides a local Cardano service layer and exposes UTxO RPC over gRPC,
which is a typed machine-oriented interface. Konduit already has a connector
abstraction for Cardano, so introducing a new backend is architecturally
consistent with the codebase direction of supporting multiple backends.

## Decision

We will implement a new Cardano backend for the adaptor based on `Dolos` over
`UTxO RPC`.

More precisely:

1. `konduit-server` will gain a Cardano backend selection that can target a new
   UTxO RPC connector implementation.
2. The new connector will communicate with a local `dolos` instance over
   localhost gRPC.
3. `dolos` will be deployed beside the existing `cardano-node`, using the local
   node as its Cardano source.
4. `konduit-server` will continue to communicate with the local `lnd` over
   localhost, using a dedicated least-privilege macaroon.
5. Only `konduit-server` will be exposed publicly, via `nginx`; `dolos`,
   `lnd`, and admin interfaces remain localhost-only.

## Decent, counter, and comments

Alternatives considered:

- Keep using Blockfrost directly.
- Use Dolos MiniBF with a smaller patch.
- Implement an Ogmios and Kupo specific backend.

Direct Blockfrost was not selected for this implementation effort because the
target deployment is explicitly a same-host setup centered on local Cardano
infrastructure.

Dolos MiniBF was not selected for this effort because the chosen scope is to add
a UTxO RPC backend rather than a Blockfrost-compatible backend, even if the
MiniBF path would likely require less immediate implementation work.

An Ogmios and Kupo specific backend was not selected for this project because
the deployment scope chosen here is centered on Dolos, and because Ogmios and
Kupo are intentionally out of scope for this particular integration effort.

## Status

Accepted.

## Consequences

Positive:

- Konduit gets an additional Cardano backend aligned with the project's
  multi-backend direction.
- Production deployments can rely on local Cardano infrastructure.
- The adaptor architecture becomes cleaner: public API in one service, Cardano
  state in another, with localhost-only boundaries between them.
- Future backends can follow the same connector contract.

Negative:

- This is a larger implementation than a MiniBF compatibility patch.
- The UTxO RPC integration requires careful mapping into `cardano-sdk` data
  types.
- Deployment becomes a multi-service stack that includes Dolos.

Neutral / follow-up:

- The exact connector crate shape, test strategy, and deployment procedures are
  specified in companion design and PRD documents.
- `Ogmios` and `Kupo` remain available for other operator workloads but are not
  considered part of the Konduit adaptor runtime for this effort.

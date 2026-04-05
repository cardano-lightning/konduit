---
title: "Adaptor Deployment PRD"
authors:
  - "@OpenCode"
created-at: 2026-04-05
status: draft
---

# Objective

Define the production deployment requirements for operating a Konduit adaptor on
a single Ubuntu 24.04 host with local `bitcoind`, `lnd`, `cardano-node`, and a
new local `dolos` instance.

# Background

The target operator already manages Bitcoin and Lightning infrastructure through
`systemd`, wants to deploy directly to production, expects anonymous public API
traffic from mobile wallet applications, and wants a manual, controlled upgrade
workflow.

# Problem Statement

Konduit needs a documented deployment shape that is:

- compatible with local `lnd` and `cardano-node`
- safe for anonymous public exposure of consumer API endpoints
- operationally manageable under `systemd`
- explicit about secrets, localhost boundaries, and rollback

# Goals

- Define the runtime topology and service boundaries.
- Define exposure policy and reverse proxy requirements.
- Define secret handling and least-privilege access.
- Define operator workflows for deployment, verification, and rollback.

# Non-goals

- Providing fully automated orchestration
- Replacing the operator's existing node management
- Defining generic cloud-native deployment for all environments

# Users

Primary user:

- adaptor operator running the stack on a single server

Secondary users:

- future developers and agents implementing deployment artifacts

# Assumptions

- Ubuntu 24.04
- `systemd`
- local `lnd` REST enabled on localhost
- local `cardano-node`
- anonymous public traffic
- nginx is acceptable and familiar
- `Ogmios` and `Kupo` are ignored for this project

# Functional Requirements

## Service Topology

The stack must include:

- `konduit-server`
- `dolos`
- `nginx`

beside the already existing:

- `bitcoind`
- `lnd`
- `cardano-node`

## Exposure Policy

Only the consumer-facing Konduit API should be publicly exposed.

## Backend Locality

Konduit, Dolos, and LND backend traffic must remain on localhost.

## Access Control

Konduit must use a dedicated least-privilege LND macaroon.

## Upgrade Model

Deployments must support manual, SHA-pinned upgrades and quick rollback.

# Non-functional Requirements

- restart behavior suitable for `systemd`
- basic host hardening through unit settings and file permissions
- request shaping suitable for anonymous public API traffic
- clear operational visibility through logs and health checks

# Security Requirements

- no public exposure of Dolos
- no public exposure of Konduit admin endpoints
- no use of `admin.macaroon`
- secrets stored outside the repository checkout
- service users constrained to the minimum files they need

# Workstreams

## Workstream 1: Runtime layout

Tasks:

- define ports and bind addresses
- define service dependencies
- define per-service users and writable paths

Definition of done:

- an operator can explain the full runtime topology from docs alone

## Workstream 2: Secrets and credentials

Tasks:

- define Konduit env/config locations
- define dedicated LND macaroon handling
- define certificate access requirements

Definition of done:

- secrets are not co-mingled with source and are least-privilege by default

## Workstream 3: systemd artifacts

Tasks:

- define unit responsibilities and ordering
- define hardening options
- define restart and dependency behavior

Definition of done:

- future work can produce unit files directly from the documented requirements

## Workstream 4: Reverse proxy and public API controls

Tasks:

- define nginx placement
- define TLS termination expectations
- define path exposure, rate limiting, and timeout requirements

Definition of done:

- public API exposure is intentionally narrow and rate limited

## Workstream 5: Operator workflow

Tasks:

- define upgrade procedure
- define validation checks
- define rollback steps

Definition of done:

- operators have a clear change-management path for production updates

# Acceptance Criteria

- deployment docs identify every service in the runtime path
- public and localhost-only boundaries are explicit
- least-privilege LND credential guidance is explicit
- reverse proxy controls are specified for anonymous public traffic
- manual deployment and rollback steps are documented

# Verification Plan

- confirm all required services can be started in dependency order
- confirm Dolos and LND are only reachable on localhost
- confirm Konduit is only publicly reachable through nginx
- confirm rate limiting and basic request guards are enabled

# Rollout Plan

1. deploy Dolos
2. verify Dolos locally
3. deploy Konduit with UTxO RPC backend
4. verify Konduit locally
5. place nginx in front of public paths
6. verify public API behavior and rate limits

# Rollback Plan

- restore prior Konduit binary and config
- disable or revert nginx route changes if needed
- keep Dolos running unless it is the direct cause of failure

# Open Questions

- exact rate-limit values
- exact health endpoints and monitoring hooks
- whether future releases should standardize install paths further

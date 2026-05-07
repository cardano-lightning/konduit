---
title: Assumptions and dependencies
---

Konduit relies on BLN. It needs to ascertained precisely what interface is
available for handling routing and payment channels.

The mobile App is running on a "normal" mobile. The Operator is deploying the
Konduit on a "standard" server.

## Adaptor deployment assumptions for self-hosting

For the current adaptor deployment work we assume:

- The operator runs Ubuntu 24.04.
- Core services are managed by `systemd`.
- `bitcoind`, `lnd`, and `cardano-node` already run on the same host.
- `lnd` exposes REST on localhost.
- `dolos` is added on the same host and communicates with the local
  `cardano-node`.
- The public consumer API is called primarily by mobile wallet applications.
- Public client access is anonymous.
- Admin and backend infrastructure remain localhost-only.
- Deployments are manual and deliberate, but each deployed release should still
  be associated with a pinned Git SHA.

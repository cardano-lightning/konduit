---
title: Architecture
---

We adopt [c4 model](https://c4model.com/abstractions) conventions.

```
Software System >> Containers >> Compontents >> Code
```

Where `>>` can be read as "is made of multiple".

# Containers

Internal:

- App
- Node
- Cardano Connector

External:

- Cardano Node
- BLN Node
- Price feed source
- BLN Invoicer
- Mobile OS API

## Container App

Won't do. See wireframe

## Container Node

- API:
  - Open Info
- API with auth:
  - Channel status
  - Pay
  - Snapshot
  - Resolve
- Queue of pending HTLCs
- DB of channels, cheques, and snapshots.
- Connectors:
  - Cardano
  - BLN
  - Price feeds
- Cron:
  - If exposure > threshold, then withdraw
  - If HTLC not being resolved, then withdraw

## Container Cardano Connector

- API:
  - Add key
  - Drop key
  - Get state
  - Submit tx
- Interface with kupmios or equivalent

TODO

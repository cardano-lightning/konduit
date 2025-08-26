---
title: "Partitioned App"
---

Partitioned App, partitions App in to two parts:

- P1 : Intended for desktop browsers (ie non-mobile). Manages L1 part eg
  creates, and manages channels.
- P2 : Intended as a mobile. Manages L2 eg pays invoices.

The partition is such that P1 has all tx building. We can assume that User has a
web wallet, accessible via CIP30. P2 retains the PPP, scans and pays BLN
invoices, but no embedded wallet is required.

Most of the functionality, minus the embedded wallet is then the same. The L1
Connector is not critical from a safety perspective.

There is an additional need to sync the two parts, most crucially the two parts
must both hold the same key. P1 is responsible for the channel setup, and thus
will generate the key. P2 imports this on its setup.

For now we defer entirely to the App requirements.

# PAp1

- Launch Page
- Setup Page
- Home Page.
- Channels Page.
  - Activity contains only L1 activity.
  - New buttons "Sync" ("Sync without secrets" may come later). Creates QR code
    scannable by PAp2.

# PAp2

- Home page:
  - New "Sync" Button scans a QR code created by PAp1. If channel exists, update
    any properties. If channel does not exists, then create channel.
- Channels page

TODO : Finish this

---
title: "Cardano Connector"
---

Cardano Connector (CaCo) is a service to provide a layer between the Cardano L1
and other parts of Konduit.

API:

- Track : VKey to track. This includes any
- Drop : VKey to untrack
- Utxos :
- Tx Submit
- Health

# Track

Track VKey. This will index UTXOs with payment credentials matching the `vkey`.

EP: `/track`

Query params:

- `vkey`. Required
- `from`. Optional. Default to "now"

Response:

- 202 - OK, regardless if it is already tracked.
- 400 - Input not parse-able
- 401 - Request not authorized
- 404 - VKey not tracked

# Drop

EP: `/drop`

Query params:

- `vkey`. Required

Response:

- 202 - OK
- 400 - Input not parse-able or VKey not tracked
- 401 - Request not authorized
- 404 - VKey not tracked

# Utxos

`/utxos`

# Tx

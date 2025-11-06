# Adaptor server

> Does all the layer 2 things.

API

```
- [x] /info/ :: information that allows a consumer to join.
- [ ] /ch - Header field required `konduit: <keytag-base16>`.
    - [x] /squash :: Also to init a channel.
    - [x] /quote
    - [ ] /pay
- /opt :: Optional endpoints that may or may not require auth
    - /cardano :: cardano connect
    - [x] /fx :: "Foreign exchange" : the relative prices of bitcoin and ada
- /admin ::This is exposed only to trusted entities.
    - [x] /tip :: Sync current tip. Tip must include only and all valid channels at tip.
    Channels in the local DB not visible are deemed Ended.
    The response is the results in all relevant keytags
    - [x] /show :: Show the state of the DB, with latest evidence (mixed receipts)
    - [ ] /edit :: Will update current state of channel with body
    - [ ] /prune :: Will drop ended
```

## TODOs

Loads.

Incorp admin into crons transaction.

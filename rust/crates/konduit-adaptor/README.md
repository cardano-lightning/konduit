<<<<<<< HEAD
# Cardano connect

> No frills interface to Cardano

Just enough to get konduit running
=======
# Adaptor server

> Does all the layer 2 things.

API

```
- /info/ :: information that allows a consumer to join.
- /ch - Header field required `konduit: <keytag-base16>`.
    - /squash :: Also to init a channel.
    - /quote
    - /pay
- /opt :: Optional endpoints that may or may not require auth
    - /cardano :: cardano connect
    - /fx :: "Foreign exchange" : the relative prices of bitcoin and ada
- /admin ::This is exposed only to trusted entities.
    - /tip :: Sync current tip. Tip must include only and all valid channels at tip.
    Channels in the local DB not visible are deemed Ended.
    The response is the results in all relevant keytags
    - /show :: Show the state of the DB, with latest evidence (mixed receipts)
    - /edit :: Will update current state of channel with body
    - /prune :: Will drop ended
```
>>>>>>> e3cb13e (Updates to konduit data.)

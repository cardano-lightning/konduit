# Konduit Server.

The server uses the Cardano Lightning Network (CLN) for routed payments. It
creates recipient paymes at `POST /ch/x/cln/payme`; consumers send the returned
payme object to `/ch/quote` and `/ch/pay`. It also serves the CLN peer protocol
on `/ch/x/cln/{quote,commit,sync}`. The optional `KONDUIT_CLN_CONFIG_PATH`
points to a Konduit CLN TOML config containing `[inbound]` peers and the
`[outbound]` core-node config (`params`, `signer`, `channels`, and `paymes`).
Without a config file, the server runs as a local terminal peer with no
inbound or outbound routes.

CLN's own peer context (`standalone::Ctx`) has no way to check that an
inbound keytag is actually backed on-chain — it only knows whether it
recognizes the keytag at all. So before any `/ch/x/cln/{quote,commit,sync}`
request reaches it, `server::data::Data::assert_cln_peer_backed` looks the
keytag up as a Konduit channel and requires it to have a funded on-chain
retainer, the same way `/ch/pay` requires one before accepting a cheque.
A keytag with no backed Konduit channel is rejected before it ever reaches
CLN's routing.

## TODOs

### Info

Previously there had been an accidental equivalence of config options and info.
As info has got more complex, the two have diverged, but we've not yet fixed
this.

#### Newer design

Channel params:

- adaptor key - Verification key
- (min) close period - Int (Duration in millis)
- (max) tag length - Int
- delegation - Bool - True if channel delegation supported, currently always
  false.

Versioning info:

- Script hash - this is hard coded through our dependency on konduit-tx

Tx building assistance:

- Script Host - Address - Address to query to find reference script

Fee:

- Any - Fee is purely infomational and it is not safety critical. We know that
  the current mechanism is insufficient, and we leave out options open.

### Admin

#### CLN Sync

For cheques that do not immediately resolve, we need to handle either:

1. A later resolution
2. An expiration

At the moment we do neither.

#### L1 Sync

Sync with l1: what's an upstream responsibility and whats a downstream
responsibility?

The problem: channel retainers must be confirmed utxos, while transactions are
built over current UTXOs (which may or may not be considered confirmed).

For the purposes of getting shit done, we will assume that all Utxos fed in via
admin are confirmed. This does make the system more sensitive add spamming, and
for the same reason more transactions will fail, but this is measured risk, and
we will fix this problem later.

Error handling. We have "global" and "local" error potential. Global, being
"cannot reach db". Local being "something went wrong with a specific channel".

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

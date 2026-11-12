# Konduit Server.

The server uses the Cardano Lightning Network (CLN) for routed payments. It
creates recipient invoices at `POST /ch/x/cln/invoice`; consumers send the returned
invoice object to `/ch/quote` and `/ch/pay`. It also serves the CLN peer protocol
on `/ch/x/cln/{quote,commit,sync}`. The optional `KONDUIT_CLN_CONFIG_PATH`
points to a Konduit CLN TOML config containing `[inbound]` peers and the
`[outbound]` core-node config (`params`, `signer`, `channels`, and `invoices`).
Without a config file, the server runs as a local terminal peer with no
inbound or outbound routes.

The BLN rail is advertised in `/info` only when `KONDUIT_BLN_BACKEND` selects
`lnd` or `cln` and the selected backend's credentials are configured. Both
backends use `KONDUIT_BLN_URL`; LND uses `KONDUIT_BLN_MACAROON` and optionally
`KONDUIT_BLN_TLS` (a PEM certificate path), while CLN uses
`KONDUIT_BLN_RUNE`. `KONDUIT_BLN_BLOCK_TIME` optionally overrides the default
`10m` block-time estimate. An unset backend omits BLN from the registry and
`/info`; selecting a backend with missing or invalid configuration fails
startup.

BLN interprets the quote amount as inbound lovelace. Its FX conversion sets a
millisatoshi ceiling, and the invoice principal must be less than or equal to
that ceiling; any surplus is available for the Lightning fee. Amount
conversions use integer division. The rail quotes fees in lovelace, re-quotes
at commit, and caps the Lightning client's fee at the amount authorized by the
inbound cheque. Payment timeouts are bounded by both the node route estimate
and the BOLT11 expiry using the system clock and configured block-time
estimate. The initial BLN consumer surface is the Rust CLI (`--features cli,bln`);
WASM BLN invoice/payment support is deferred.

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

# Rail

There is one consumer, `konduit-client`, and it is rail-agnostic. A rail is
what carries a payment onward; originating one is not a rail's job, so
`cln-rail` should hold no client at all.

Done: the rail is instructed with its own amount (one `Outbound` for both
quote and commit), its config separates known keys from the channels held,
and its commitments survive a restart.

## 1. `konduit-client` lib stops knowing about rails

`l2::{quote,pay}` take `rail: &str` and `instruction: Vec<u8>`; drop
`assert_terminal` and the `konduit_rail::RESERVED_NAME` import. The terminal
case is not special — it is `rail = "konduit"` with an empty instruction,
which the caller chooses like any other.

The lib already has everything else: `Adaptor` is the whole HTTP surface,
`l2` signs against the `quote.index` the adaptor returns, `l1` does the
on-chain side, and the CLI is load-bearing already (`submit-null-squash.sh`).

## 2. `konduit-client` bin learns the rails

`--rail cln`, plus the instruction's own fields, encoded in the request's
codec. Same shape as `konduit-server`: the lib talks to an opaque boundary,
the bin assembles the concrete rails. Only the bin ever names one.

## 3. Delete `cln-rail`'s `client` feature

`src/client.rs` and `src/bin/client.rs` go, and with them the
`reqwest`/`clap`/`toml`/`tracing-subscriber`/`anyhow` deps and the default
feature. Nothing is lost: that client starts from `SquashBody::zero()` and
recovers the receipt before it can act, so its local mirror buys a one-shot
CLI nothing that fetch-and-sign does not already give.

Fallout to carry: `examples/consumer_gateway_merchant` writes a
`client::Config` that no longer exists — the consumer's config becomes
konduit-client's own env shape; the dev-env's consumer step becomes a
`konduit-client` invocation; README and RESULTS commands follow.

## 4. Where `Outbound` lives

One struct, wanted by the rail and by konduit-client's bin. Keep it in
`cln-rail` and let the bin depend on `cln-rail`, which is thin by then. A
`cln-wire` crate is only worth it if a consumer must not compile the rail's
state machine — a compile-time nicety, not a boundary. Don't split until
something forces it.

## What stays

`cln-rail`'s `Channel`/`Receipt`: the rail's downstream leg is long-lived and
proposes squashes between payments, so local state earns its keep there. Only
the consumer half goes.

The dependency runs rail -> client, because a rail is a konduit client pointed
downstream. Client and rail as siblings over a shared type bag is the shape to
avoid.

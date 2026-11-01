# Plan: the outbound-rail spec, as crates

Target: [konduit-as-an-outbound-routing-platform.md](./konduit-as-an-outbound-routing-platform.md).

No legacy. The `/ch/*` surface, the `Option<Outbound>` commit shape, `wire::sync`, the two placeholder
constants and the two `todo!()` handlers all go; nothing is kept as an alias. Blast radius is under
"HTTP surface" — it reaches `konduit/client` and everything riding on it.

## Crate layout

| crate            | path                      | holds                                                                                                        |
| ---------------- | ------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `konduit-rail`   | `packages/konduit/rail`   | `Constraints`, `Quoted`, `Outcome`, `Resolution`, the problem enums, `Rail`, the erasing layer, the registry |
| `cln-rail`       | `packages/cln/rail`       | `impl Rail for Cln` — outbound over a Cardano Lightning channel — plus the `cln-client` binary               |
| `konduit-server` | `packages/konduit/server` | the platform: inbound verification, fee and time margin, `/pay/{rail}/…`, `/pay/konduit/…`                   |

`packages/cln/server` → `packages/cln/rail`, crate `cln-server` → `cln-rail`, lib `cln_server` → `cln_rail`.
Touches `Cargo.toml:36`, `Cargo.toml:136`, `packages/konduit/server/Cargo.toml`, the seven
`konduit/server` modules that name `cln_rail`, `flake/process-compose/dev-env.nix:135` and `RESULTS.md:74`.

The standalone server goes: `src/bin/server.rs` and `standalone/{ctx,inbound,inbounds,config}.rs`, with the
`server` and `standalone` features and the `actix-web` dependency. `konduit-server` is the platform and
already does this work against a real DB — `server/data.rs`'s `cln_quote`/`cln_commit` do the inbound
verification (`assert_cln_peer_backed`, `can_commit`) and the fee arithmetic
(`FEE_PLACEHOLDER`, `QUOTE_PAY_TIME_MARGIN_MS`) that `standalone::Inbounds` only stands in for. Nothing in
the nix dev-env runs `cln-rail` as a server; it runs `konduit-server` for both the gateway and the
merchant role (`dev-env.nix:140-178`).

What stays is the client: `standalone/client.rs` → `src/client.rs` behind the existing `client` feature,
and `src/bin/client.rs` (the `cln-client` binary) unchanged in shape. It depends only on `Channel`,
`Receipt` and `Signer`, not on `standalone::Ctx`, so the drop is a module move and a `use` fix. Its stdout
JSON / stderr tracing split stays as is.

`examples/consumer_gateway_merchant.rs` is load-bearing — it is the nix demo's config generator
(`dev-env.nix:114-137`), writing each role's rail TOML plus `.env.<role>` for `konduit-server` to read via
`KONDUIT_CLN_CONFIG_PATH`. It keeps emitting the rail config and the consumer config, and loses its
`ServerConfig`/`inbound`/`inbounds` blocks, which nothing consumes.

### Modules that are konduit's, not the rail's

In `cln/server` today; they move out, because the platform owns them and `bln` needs them too:

- `wire/auth.rs` — `Keytag` and the header. `konduit_tmp::Keytag` already exists and `data.rs` converts
  between the two with `keytag.as_bytes().to_vec().try_into().unwrap()` three times
  (`data.rs:240,265,291`). Keep one type, in `konduit-tmp`; the unwraps go away.
- `wire/payme.rs` — likewise duplicates `konduit_tmp::{Payme, PaymeRequest}`.
- `paymes.rs` — the terminal payee's secret store, served from `/pay/konduit/pay`. Already driven entirely
  from `konduit-server` (`data.payme`, `data.cln_commit`'s `paymes_reveal`). Move it to `konduit-server`.
- `ctx::Params` — deleted outright, not moved. Both its fields are fee and margin, and neither is the
  rail's. See below.
- `wire/{quote,commit,sync}.rs` — deleted, not moved: the old node-to-node protocol, which the consumer
  API now subsumes.

`receipt.rs` stays in `cln-rail` for its outbound channel state. Note there are two receipt
implementations, this one and `konduit_tmp::receipt`, which `konduit/server/src/channel.rs` uses for
inbound. Converging them is worth doing and is not in this plan.

## Phase A — `konduit-rail` _(done)_

`packages/konduit/rail` exists, compiles, and has four tests. It holds `Constraints`, `Quoted`, `Outcome`,
`Resolution`, the `Rail` trait, the erasing `DynRail` over `RawTarget` + boxed futures, and a `Registry`
that rejects `konduit` and duplicate names at construction. Dependencies: `konduit-data`, `minicbor`,
`serde`, `serde_json`, `thiserror`. No actix, no transport.

Three decisions worth recording:

- **No `problem-details` yet.** `QuoteProblem`, `PreProblem`, `PostProblem` are plain `thiserror` enums,
  each variant carrying a `String` of runtime context that is what the user sees. Each keeps a
  `slug() -> &'static str` with the spec's stable slugs, so the HTTP status mapping is a one-line match in
  `konduit-server` and the later `problem-details` migration is mechanical. No HTTP in this crate.
- **`Relative(Duration)`** is the type of `Constraints.timeout` and `Quoted.timeout`, a deliberate
  deviation from the spec's code block. `konduit_data::Duration` is used both absolutely and relatively
  across these crates and the boundary is exactly where they get mixed: `data.cln_commit:267` subtracts the
  margin from an absolute timeout and keeps it absolute, while `ctx::commit` subtracts it and then compares
  to `now()`. `Relative::{absolute, until}` are the only two conversions.
- **Targets carry both codecs.** `Target: Deserialize + for<'b> minicbor::Decode<'b, ()>` and
  `RawTarget::{Json, Cbor}`, because the boundary is content-negotiated and
  `for<'de> Deserialize<'de>` alone covers only half of it. `RawTarget::decode` raises `bad-target`, so no
  rail hand rolls it, and `DynRail::commit` reports it as `FailedPre` — nothing was sent.
- **`fx-unavailable` is a placeholder.** The variant exists and nothing produces it; no rail fetches a
  rate, and `fx-client` stays unwired until `bln` needs it.

## Fees and margins

Konduit charges; a rail does not. Today this is spread over four places that disagree:
`konduit/server`'s `FEE_PLACEHOLDER` and `QUOTE_PAY_TIME_MARGIN_MS` (`data.rs:18-19`,
"TODO :: MOVE TO CONFIG"), `cln_rail::ctx::Params::{fee, hop_timeout}`, `cln.rs::build(args, fee)`
passing konduit's fee _into_ those params, and the example setting them from `KONDUIT_FEE`.

After: one owner, one struct, `konduit_server::Margins { fee: u64, time: Relative }`. `ctx::Params` is
deleted, `cln.rs::build` loses its `fee` argument, `KONDUIT_FEE` stays the platform's and never reaches a
rail, and the example stops plumbing it there. A rail is not configured with a fee or a margin because it
has neither: `Quoted` reports what its _network_ costs, which the rail learns from its backend and passes
through unchanged.

**Quote, outward.** `rail.quote(amount, target) -> Quoted { fee_net, timeout_net }`, then konduit answers
`fee = fee_net + margins.fee` and `relative_timeout = timeout_net + margins.time`.

**Commit, inward.** From a cheque of amount `A` and absolute timeout `T`:

```
amount_net  = A - margins.fee
timeout_net = Relative::until(T, now) - margins.time
```

**The invariant**: strip is exactly the inverse of add, so a cheque that satisfies a quote satisfies the
rail. A consumer quoted `(a, target) -> (fee, timeout)` sends `A ≥ a + fee` and `T - now ≥ timeout`; then
`amount_net ≥ a + fee_net` and `timeout_net ≥ timeout_net(quoted)` — precisely what the rail quoted. One
`add_margins`/`strip_margins` pair in one module, with a round-trip test, is what keeps that true; two
inlined additions are what let it drift, and a quote promising less margin than commit demands is an
`insufficient-margin` the consumer could not have anticipated.

Consequences worth being explicit about:

- **`Constraints.amount` is a ceiling, not a payment.** The rail spends at most that and keeps nothing;
  `FailedPre(insufficient-margin)` is what it says when the ceiling cannot cover what its backend quotes.
- **A volatility buffer is not a fee and not a margin.** A rail whose network prices in another asset, or
  reads another clock, may hold back a buffer against divergence between quote and commit. That is the
  rail's own and belongs to such a rail's config. CLN is ADA on Cardano's Posix clock, both sides, so there
  is nothing to diverge and no buffer: the concept waits for `bln`.
- **Quotes stay stateless.** `channel.rs`'s quote cache is commented out (`channel.rs:112,339-349`); leave
  it that way. Konduit's cover check is only what it owns — both subtractions positive — and the real cover
  check is the rail's, since only the rail knows its network's cost at commit time.
- **`index` is the platform's.** It leaves `Quoted` (`wire::quote::Response.index`, already FIXME'd) and
  stays in the consumer-facing `konduit_tmp::Quote`, filled from `channel.can_commit` as `data.cln_quote`
  does now. `can_commit` takes the _total_ cheque amount, so it is called with `a + fee`.
- Konduit's fee stays flat in v0, as both current implementations already assume. A ppm component is a
  later change to one struct and one function.

## Phase B — `cln-rail`

CLN is the Cardano Lightning Network, so this rail's hop is a konduit channel and its backend is a konduit
node: the rail is a konduit client pointed downstream, concretely and not behind a generic. `channel.rs`
keeps the channel state (receipt, index proposal, secret cache) and drops its private `http-client` stack
for `konduit-client`, which is already a client of this API.

### Target and the shapes it replaces

- `QuoteTarget = CommitTarget = Payee(VerifyingKey)`, replacing `wire::quote::Request.payee` and
  `wire::commit::Outbound`. Always present — no `Option` anywhere in a target. Terminal payment is
  `/pay/konduit/pay`, which calls no rail at all.
- `Outbound.amount` goes: the amount is `Constraints.amount`, already net of konduit's fee. Two sources of
  truth for one number is what `data.rs:276`'s `inbound.amount().checked_sub(FEE_PLACEHOLDER)` and
  `ctx.rs`'s `inbound.amount().checked_sub(amount)` each separately work around.
- `Option<Outbound>` goes. Terminal-or-onward was an `Option` in the body; it is now which path the rail
  `POST`s to downstream, which is the spec's structural selection: the peer is the payee →
  `/pay/konduit/pay` on it; otherwise `/pay/cln/pay` with the payee as target, for the peer to route on.
- `wire::sync` goes. Resolution downstream is `GET /pay/konduit/receipt` + `POST /pay/konduit/squash`, the
  same two calls a consumer makes — the spec's "an inbound routing peer uses the same shape". The
  bidirectional one-round-trip receipt exchange has no counterpart in the new API.

### Outbound configuration: a runtime lookup

`channels::Config { channels: Vec<channel::Config> }` is a startup-only list, so a new outbound peer means
a restart. Replace it with a lookup the rail can be told about while running:

- `Lookup`, a `BTreeMap<VerifyingKey, Outbound>` behind the same `Mutex`/`RwLock` discipline as `Channels`,
  where `Outbound` is the peer's `url` plus the `tag` of the channel to it. Keyed by key, as asked:
  `{ key: url }` is the config surface, with the tag alongside it because the channel needs one.
- Resolving a `Payee` target goes through `Lookup`; a miss is `FailedPre(unroutable)`.
- Seeded from the TOML as today, and settable at runtime: `Lookup::upsert`/`remove`, exposed on a
  `konduit-server` admin route (the `/admin` scope already exists, trusted-sources-only per its comment).
  Persisting the lookup can wait; re-reading config at startup is the fallback.

### Seeing its own channels

The rail can only route where it holds a funded outbound channel, and it has no way to check that today —
the inbound mirror of this is `assert_cln_peer_backed`/`assert_backed`, which exists. Add a chain view:
one read-only call that lists the konduit channels where this node is the payer, over the Cardano
connector/indexer `konduit-server` already holds (`cardano.rs`, `index.rs`, `connector-client`). Use it to
(a) validate a `Lookup` entry when it is set, and (b) answer an admin query of "what can I pay over". Not a
generic port — the one concrete client, injected at construction.

### `commit`

`ctx::commit2` is already the rail entry point — `(payee, amount, timeout, lock)` is `Constraints` plus
target, and `data.cln_commit` already calls it that way. So `ctx::commit` and `ctx::commit2` collapse into
`Rail::commit(&self, inbound: Constraints, Payee) -> Outcome`, and the inbound-specific parts of
`ctx::commit` (the `timeout < now()` check, the fee check, the `outbound: None` paymes branch) do not come
with it.

`Outcome` has no `Err` arm, so every current error path needs a decision:

| today                                                  | becomes                          |
| ------------------------------------------------------ | -------------------------------- |
| `Payee` not in `Lookup`, or `channels::Error::NoRoute` | `FailedPre(unroutable)`          |
| amount beyond channel capacity                         | `FailedPre(amount-out-of-range)` |
| `Constraints` cannot cover what the backend quotes     | `FailedPre(insufficient-margin)` |
| transport error _before_ the locked cheque is sent     | `FailedPre(rail-unavailable)`    |
| transport error or no answer _after_ it is sent        | `Pending` — never an error       |
| downstream declines explicitly, post-send              | `FailedPost(outbound-rejected)`  |
| `res.secret = Some`                                    | `Success(secret)`                |
| `res.secret = None`                                    | `Pending`                        |

The "after it is sent" row is the whole point of the no-`Err` rule and the one place the current code is
actively wrong: `channel.commit(&req).await?` propagates a dropped response as `Err`, and both callers read
that as a non-payment.

### Idempotency: rewrite `commits.rs`

`Commits::insert` fails with `AlreadyCommitted`, which `ctx` maps to `Error::NoRetries`. A consumer
retrying under one lock therefore gets an error indistinguishable from "not paid" while the payment is in
flight. The lock is the spec's idempotency key, so:

```rust
struct Commit { index: u64, amount: u64, timeout: Duration, state: State }
enum State { InFlight, Resolved(Secret), Failed(PostProblem) }
```

- `get_or_claim(lock) -> Claim::{Fresh, Existing(Outcome)}` under one lock acquisition; `Existing` replays
  the recorded outcome (`InFlight` → `Pending`) instead of erroring.
- Retain each entry at least to its commitment timeout, then prune (mirror `Paymes::prune`) — `reveal` has
  to answer for exactly that window.

### `reveal`

New; nothing in the package does it today. `Resolution` from, in order: `Commits` state → the outbound
channel's secret cache → `Unknown` with no record → `Failed(outbound-timeout)` if in flight past the
commitment timeout, `Pending` if before.

**Cache the secret before squashing.** `Receipt::apply_squash` drops every squashed cheque
(`receipt.rs:239`) and `ctx::spawn_dispatch_squash` proposes a squash immediately on `apply_secret`, so the
unlocked cheque holding the secret is evicted from the outbound receipt as soon as that squash lands and
`Channel::reveal` (`channel.rs:144`) goes blind — inside the very window the spec says the rail must cache
it. Write it into `Commits` first.

Optional push: `spawn_dispatch_squash` is already a background task, so give it a hook that notifies the
platform when a `Pending` resolves. The platform's pull loop then stays a fallback rather than the only path.

## Phase C — the platform, in `konduit-server`

### HTTP surface

Everything under `/pay` is keytag-authenticated, so `auth::no_auth` and its FIXME go and `cln_auth`'s
middleware — on the one `Keytag` type, now in `konduit-tmp` — is the only one left.

| route                      | handler                                                         |
| -------------------------- | --------------------------------------------------------------- |
| `GET /info`                | `info`, extended with asset class, rail names, fee, time margin |
| `POST /pay/konduit/payme`  | `payme`                                                         |
| `POST /pay/konduit/pay`    | terminal pay, on the `paymes` table moved in from `cln/server`  |
| `GET /pay/konduit/receipt` | `receipt`                                                       |
| `POST /pay/konduit/squash` | `squash` → `SquashStatus`                                       |
| `POST /pay/{rail}/quote`   | registry → `rail.quote`                                         |
| `POST /pay/{rail}/pay`     | registry → `rail.commit`                                        |

Deleted outright: the `/ch/x/cln` scope (`quote`, `commit`, `sync`), the `/ch` scope
(`receipt`, `squash`, `quote`, `pay`), `handlers::{cln_quote, cln_commit, cln_sync, quote, pay}`,
`data::{quote, pay}` (both `todo!("Superceded")`), `data::{PayBody, PayResponse}` and
`konduit_tmp::PayBody`'s unmediation glue (`handlers.rs:166-176`, "FIXME :: Remove the glue required here
for historical reasons"), and `data::align_commitments`, whose lock, fee and time checks fold into the
ordered commit function below. `handlers::squash` currently discards its own result and re-reads status
(`handlers.rs:143-152`, "FIXME :: REMOVE THIS TEMPORARY PATCH") — straighten it as the route moves.

`konduit/client/src/adaptor.rs` hard-codes `/ch/receipt`, `/ch/quote`, `/ch/pay`, `/ch/squash`
(`adaptor.rs:69,76,84,107`) and is what `l2.rs`, `konduit-cli`, `konduit-wasm` and the integration tests
ride on. No-legacy means it moves to the table above in the same change; `cln-rail`'s downstream calls then
come from that same client.

### Registry

Hold a `konduit_rail::Registry` in `server::Data` beside `db`/`fx`, built from config at startup; the cln
rail registers under `"cln"`, and `konduit` is already rejected there rather than at request time.
`cln.rs::build` becomes rail construction plus registration. `Data`'s `cln: Arc<cln_rail::Ctx>` field and
its `cln()` accessor go — the platform holds rails, not one named rail.

### `POST /pay/{rail}/pay`

One ordered function, replacing `data.cln_commit` and the `todo!()` `data.pay`, failing before any rail call:

1. Verify inbound: keytag → channel, `assert_backed`, `can_commit`, `apply_locked`. Konduit's alone.
2. `amount_net = A - margins.fee`, `checked_sub` → konduit's own `insufficient-fee` problem.
3. `timeout_net = Relative::until(T, now) - margins.time`.
4. `registry.get(rail)?.commit(Constraints { .. }, raw_target)`.
5. Map `Outcome`: `Success` → `apply_secrets` then return the secret; `Pending` → record and answer pending;
   `FailedPre`/`FailedPost` → the problem's slug, its status, and its string, with the inbound cheque left
   live to elapse — say so in the body, since the consumer carries that liquidity cost.

Steps 2 and 3 are `strip_margins`, nowhere else. The lock comes from the cheque and konduit never parses
the target; that holds today and only needs to keep holding once the target is opaque bytes.

### `POST /pay/{rail}/quote`

`rail.quote(amount, target)` then `add_margins`, answering `konduit_tmp::Quote { index, amount, fee,
relative_timeout }`. Replaces `data.cln_quote`'s inline `response.fee + FEE_PLACEHOLDER` and
`saturating_add(QUOTE_PAY_TIME_MARGIN_MS)`.

### Pending resolver

Nothing resolves a `Pending` today. Add a background task on the existing `cron.rs` pattern: walk pending
commits, `rail.reveal(lock)`, and on `Success` apply the secret to the inbound channel so it appears as an
unlocked cheque in the next squash proposal — which is what the spec's resolution clause means by "the
secret arrives as an unlocked cheque in the proposal". Read `Unknown` past the commitment timeout as
failed; before it, as a rail fault to log, not a failed payment.

### Problems at the boundary

`handlers::Error` maps everything that isn't a mediation failure to `400` with the `Display` string
(`handlers.rs:22-40`). Replace with one match from slug to status — `bad-target` 400, `unroutable`,
`amount-out-of-range`, `insufficient-margin` 422, `rail-unavailable`, `fx-unavailable` 503,
`outbound-rejected` 502, `outbound-timeout` 504 — carrying the problem's string as the body, content
negotiated like `Mediate` already is. One match, one place, swappable for `problem-details` later.

## Phase D — demo and tests

- `cln-client` keeps paying: `payme` from the merchant, `/pay/cln/pay` through the gateway, squash. Both
  ends are `konduit-server`, so its changes are the new paths, the `Payee` target, and `konduit-client`
  underneath.
- Update `dev-env.nix:135` (`-p cln-rail --example … --features "server client"` → `-p cln-rail … client`)
  and `RESULTS.md:74`.
- `standalone::ctx`'s `local_payme_commits_through_the_standalone_client_path` dies with standalone. Replace
  with: `Outcome` mapping unit tests, one per row of the Phase B table; an idempotency test (one lock
  committed twice → one outbound payment, same secret); an `add_margins`/`strip_margins` round-trip test
  asserting the cover invariant; and a platform test in `konduit-server` driving a stub `Rail` through
  commit-pending-reveal — the `konduit-rail` tests already do this much against a stub, so the harness
  exists.

## Open questions and risks

1. **Persistence.** `Commits` is in-memory (`ctx.rs:78` TODO), and so is the rail's outbound receipt.
   `Pending` only means something if it survives a restart: today a restart between commit and reveal loses
   the secret and the consumer's cheque elapses. `recover_receipt` recovers the outbound receipt from the
   partner; the secret has no such source. This is the one item that is load-bearing for the spec rather
   than tidying — `konduit-server` has a DB, the rail does not. The runtime `Lookup` has the same question
   and a cheaper answer: config on restart is fine.
2. **Flag day.** No-legacy makes `konduit-server`, `konduit-client`, `cln-rail`, the cli, the wasm bindings
   and the nix demo one atomic change. Worth doing in that order on one branch, with the demo as the gate.
3. **Two receipt implementations** (`cln_rail::receipt`, `konduit_tmp::receipt`). Out of scope, worth a
   follow-up.
4. **`problem-details`** is skipped for now by choice; the `slug()` methods are the seam for picking it up.
5. **Fx** — placeholder only: the `fx-unavailable` variant exists, unproduced, and `fx-client` stays
   unwired until a rail denominates in something other than the instance asset.
6. **Terminality** — the spec's own open issue. The plan takes the stated resolution (`/pay/konduit/pay`,
   fees still konduit's) and does nothing cleverer.

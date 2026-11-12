# BLN Client

> Clients for konduit server needs

## Implementations

- [x] LND
- [x] Core Lightning health, invoice generation, quote, commit, and reveal

## Health checks

The `bln-cli health` command performs an authenticated, read-only node health
check and prints JSON. LND uses `LND_BASE_URL` and `LND_MACAROON`; Core Lightning
REST uses `CLN_BASE_URL` and `CLN_RUNE`.

```sh
LND_BASE_URL=https://lnd.example.org LND_MACAROON=<hex-macaroon> \
  cargo run -p bln-client --features cli -- health

CLN_BASE_URL=https://cln.example.org CLN_RUNE=<rune> \
  cargo run -p bln-client --features cli -- health
```

Health fields not supplied by a backend are returned as `null`. For CLN,
`synced_to_chain` and `synced_to_graph` are derived from the absence of the
corresponding `getinfo` warnings.

## Invoice generation

Create a fixed-amount BOLT11 invoice on either supported backend. The command
prints JSON containing the `bolt11` payment request. Invoice creation changes
node state and the invoice remains payable until it expires or is canceled.

```sh
LND_BASE_URL=https://lnd.example.org LND_MACAROON=<hex-macaroon> \
  cargo run -p bln-client --features cli -- invoice \
    --amount-msat 1000000 --description "test invoice"

CLN_BASE_URL=https://cln.example.org CLN_RUNE=<rune> \
  cargo run -p bln-client --features cli -- invoice \
    --amount-msat 1000000 --description "test invoice"
```

## Quote and commit

The payment flow is two-stage. `quote` reports the invoice principal, quoted
fee cap, and maximum total debit. `commit` requotes immediately before payment,
aborts if the fee changed, and requires the operator to type `commit`. Both
backends use the quoted fee as an absolute cap, so payment cannot exceed the
displayed maximum total.

```sh
# Stage 1: obtain and inspect the quote
cargo run -p bln-client --features cli -- quote --invoice "$BOLT11"

# Stage 2: pass the exact quoted fee and confirm the displayed maximum
cargo run -p bln-client --features cli -- commit \
  --invoice "$BOLT11" --quoted-fee-msat 22 \
  --quoted-timeout-seconds 29400
```

CLN quoting accounts for invoice route hints and evaluates routes using
`getroute`; CLN commits pass the quote as `maxfee` to `pay`. CLN reveal queries
`listsendpays` by payment hash. CLN quote timeouts include a 135-block margin for
its randomized shadow routes.

## Justfile

There is a complementary justfile. This is a quick way to crosscheck information
coming from the server.

## Notes

The LND backend uses the REST API. Reveal looks up payment preimages from the
paginated payments endpoint and caches results locally.

The key property we want is: robustness. If (when) the thing falls over its very
simple to stand back up again.

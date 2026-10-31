# Consumer-to-merchant L2 transfer and wind-down rerun

Run date: 2026-10-30 (local process-compose Cardano testnet, network magic 42)

## Rerun status

The rerun could not be started in this checkout. `nix develop .#extras --impure`
fails during flake evaluation because the installed Nix (2.35.2) does not expose
the `fetchClosure` built-in used by the pinned nixpkgs input. The original
results below are retained as the last successful run; this invocation did not
replace or validate them.

## Checks

- `cargo test --workspace` — passed (ignored tests remained ignored).
- `cargo clippy --workspace --all-targets -- -D warnings` — passed.

## L2 payments and claims

The full process-compose environment ran with a 30-second channel close period
and 2-second server/admin sync interval. Ten 100,000-lovelace payments were
initiated by the consumer to merchant-issued paymes and routed through the
gateway. The gateway then settled 1,000,000 lovelace to the merchant over the
gateway/merchant channel.

| Tag                | Payer    | Payee    | L1 open transaction                                                |
| ------------------ | -------- | -------- | ------------------------------------------------------------------ |
| `c001000000000001` | Consumer | Gateway  | `1f605a8d905b95344e50859d0d17866bea2d26d175fa5ca3eef8a8470f286cef` |
| `c002000000000002` | Gateway  | Merchant | `6f4b1378b2fbb90ef2ca2fcc604165e44c8d54454b260ad40f2ee8d930b74537` |

The gateway claimed the consumer-side channel: `consumer show tip` reported
`subbed = 1,024,150` lovelace (ten quotes of 102,415 lovelace). The merchant
claimed the gateway-side channel: its `subbed` amount was 1,001,001 lovelace.
The gateway-to-merchant batch settlement was a separate L2 payment after the
ten consumer-originated payments.

### Gateway fees

The gateway's CLN routing fee is a flat 1,414 lovelace per routed payment
(`cln_server::ctx::Params::default`). Across ten payments, that is 14,140
lovelace of configured routing fees. The recorded consumer quotes were 102,415
lovelace for each 100,000-lovelace merchant payment, i.e. 2,415 lovelace above
the requested amount per quote (24,150 total quote overhead). This quote
overhead is the observed consumer charge; it is distinct from the configured
1,414-lovelace CLN hop fee. The recorded settlement and claim figures do not
provide an independent fee ledger to reconcile the difference.

## Reproduction

Run from the repository root with Nix flakes enabled and a working toolchain
compatible with the pinned nixpkgs:

1. Start the local testnet and services in one shell:

   ```sh
   nix --extra-experimental-features 'nix-command flakes' develop .#extras --impure \
     -c bash -lc 'source .env.process_compose && process-compose-dev-env'
   ```

   This creates a fresh local testnet, publishes the reference script, starts
   the merchant and gateway, and runs one consumer payment. Keep this process
   running through the payment and channel-verification steps. Startup
   automatically makes one 1000-unit consumer payment.

2. In a second shell, enter the same development environment and source
   `.env.process_compose`. The generated CLN configs are under
   `$RUN_DIR/cln-demo-configs`. Issue nine more 1000-unit payments to make ten
   total (the demo converts each to a 100,000-lovelace payment):

   ```sh
   nix --extra-experimental-features 'nix-command flakes' develop .#extras --impure
   source .env.process_compose
   for i in $(seq 1 9); do
     cargo run -p cln-server --bin cln-client -- \
       --config "$RUN_DIR/cln-demo-configs/consumer.toml" pay \
       --merchant http://127.0.0.1:5664 --amount 1000
   done
   ```

3. Verify the consumer-side and merchant-side channel tips with their
   respective `show tip` queries. Create a merchant payme for 1,000,000
   lovelace and pay it from the gateway client to the merchant as a separate L2
   payment; this is the batch settlement, not one of the ten consumer payments.
4. Close both channels, wait at least the configured 30-second close period,
   then submit both wind-down transactions. Verify both channel lists are empty
   before stopping process-compose.

The rerun command used in this environment was the startup command above. It
stopped before process-compose could run because of the Nix `fetchClosure`
evaluation error noted under Rerun status.

## Close and wind down

| Tag                | Close transaction                                                  | Wind-down transaction                                              |
| ------------------ | ------------------------------------------------------------------ | ------------------------------------------------------------------ |
| `c001000000000001` | `f7fa42ae4e6e013e458f4118ae55fd7f7d66da012d3b485aaf71f1fa6567901a` | `44f656696bc618c21448625b7a6cdb961f4602d15738853833230a3edc86ab76` |
| `c002000000000002` | `824726721cbc67514375baeb1fd677c27e17a8c660f03bda6c137b55f919bdec` | `0ae33dfdfa10c882e2a2694ca7cb84113bb6a6e2cfb11a5b121591462569ac05` |

After both close periods elapsed, wind-down transactions were submitted. Final
`consumer show tip` queries for both payer keys returned empty channel lists.
Process-compose was stopped after verification.

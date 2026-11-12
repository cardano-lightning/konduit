# Heavy Cardano development flake

This flake contains the Cardano node/CLI, Kupo, Cardonnay, and process-compose
environment, as well as the NixOS preprod VM configuration. Its dependencies
are kept separate from the default repository development shell.

Run the following commands from the repository root. The first invocation may
ask you to trust the IOG binary cache configured by this flake.

## Start the local devnet

```sh
nix --extra-experimental-features 'nix-command flakes fetch-closure' --accept-flake-config develop path:./flake/heavy --command bash -c 'PC_DISABLE_TUI=true process-compose -D -f "$PROCESS_COMPOSE_YAML" -L "$RUN_DIR/process-compose-dev-env"'
```

Allow about 90 seconds for Cardonnay to start the local cluster and submit its
configuration transaction. Check readiness from another repository-root shell:

```sh
nix --extra-experimental-features 'nix-command flakes fetch-closure' --accept-flake-config develop path:./flake/heavy --command bash -c 'for p in initialize-testnet kupo-utxo kupo-indexer konduit-indexer publish-reference-script konduit-server-merchant konduit-server-gateway; do process-compose process get "$p"; done'
```

Continue when `initialize-testnet`, both Kupo processes, and both servers report
`Ready`. `publish-reference-script` should report `Completed`; the indexer is a
scheduled process. Give the indexer one polling cycle after opening channels:
the schedule interval is 10 seconds, so the commands below wait 15 seconds.

## Optional external BLN node

BLN is disabled unless `$KONDUIT_BLN_ENV_FILE` exists and selects a backend.
By default this is the repository-root `.env.bln`; keep that file user-owned
and out of version control. It uses shell `KEY=value` assignments. For LND:

```sh
KONDUIT_BLN_BACKEND=lnd
KONDUIT_BLN_URL=https://your-lnd-rest-host:8080
KONDUIT_BLN_MACAROON=<hex macaroon>
# Optional: KONDUIT_BLN_TLS=/path/to/tls.cert
```

For Core Lightning REST, use `KONDUIT_BLN_BACKEND=cln`, `KONDUIT_BLN_URL`, and
`KONDUIT_BLN_RUNE`. Optional settings are `KONDUIT_BLN_BLOCK_TIME` and the
smoke-helper amounts `KONDUIT_BLN_INVOICE_AMOUNT_MSAT` (default 1000000) and
`KONDUIT_BLN_INBOUND_AMOUNT_LOVELACE` (default 10000000). Override the file
location by exporting `KONDUIT_BLN_ENV_FILE` before entering the dev shell.
The `validate-bln-config` process checks shape before gateway startup; it does
not probe node reachability. In a dev shell, verify credentials/reachability
with `cargo run -p bln-client --features cli -- health`, mapping the selected
server values to `LND_BASE_URL`/`LND_MACAROON` or `CLN_BASE_URL`/`CLN_RUNE`.
The gateway `/info` reporting `bln` means the rail is registered, not that the
external node is currently healthy. The merchant remains CLN-only.

After funding/opening the consumer-to-gateway channel and submitting initial
squashes as described below, run the separate consented helper. It creates a
fresh node invoice and invokes `l2 bln` without `--yes`, showing the quoted fee,
maximum debit, and timeout for approval:

```sh
nix --extra-experimental-features 'nix-command flakes fetch-closure' --accept-flake-config develop path:./flake/heavy --command scripts/submit-bln-payment.sh
```

The existing CLN demo command remains unchanged:

```sh
nix --extra-experimental-features 'nix-command flakes fetch-closure' --accept-flake-config develop path:./flake/heavy --command bash -c 'set -a; source "$CLN_DEMO_CONFIG_DIR/consumer.env"; set +a; cargo run -q -p konduit-client -- -y l2 cln --payee "http://127.0.0.1:$KONDUIT_SERVER_MERCHANT_PORT" --amount 1000'
```

## Reproduce the CLN-backed channel lifecycle

Fund the three demo wallets and generate the channel parameters:

```sh
nix --extra-experimental-features 'nix-command flakes fetch-closure' --accept-flake-config develop path:./flake/heavy --command bash -c 'eval "$(flake/heavy/scripts/fund-cln-wallets.sh)"'
```

Open the consumer-to-gateway and gateway-to-merchant channels with
`konduit-client`:

```sh
nix --extra-experimental-features 'nix-command flakes fetch-closure' --accept-flake-config develop path:./flake/heavy --command bash -c 'source "$RUN_DIR/cln-channels.env"; KONDUIT_SIGNING_KEY="$KONDUIT_CONSUMER_WALLET" KONDUIT_TAG="$KONDUIT_CONSUMER_GATEWAY_TAG" KONDUIT_SERVER_URL="$KONDUIT_GATEWAY_URL" cargo run -q -p konduit-client -- -y l1 open 10000000'
```

```sh
nix --extra-experimental-features 'nix-command flakes fetch-closure' --accept-flake-config develop path:./flake/heavy --command bash -c 'source "$RUN_DIR/cln-channels.env"; KONDUIT_SIGNING_KEY="$KONDUIT_GATEWAY_WALLET" KONDUIT_TAG="$KONDUIT_GATEWAY_MERCHANT_TAG" KONDUIT_SERVER_URL="$KONDUIT_MERCHANT_URL" cargo run -q -p konduit-client -- -y l1 open 10000000'
```

Wait 15 seconds for the indexer to catch up, then submit the initial null
squashes required by the servers and make one CLN-rail payment from consumer to
merchant through the gateway:

```sh
sleep 15
nix --extra-experimental-features 'nix-command flakes fetch-closure' --accept-flake-config develop path:./flake/heavy --command scripts/submit-null-squash.sh
```

```sh
nix --extra-experimental-features 'nix-command flakes fetch-closure' --accept-flake-config develop path:./flake/heavy --command bash -c 'set -a; source "$CLN_DEMO_CONFIG_DIR/consumer.env"; set +a; cargo run -q -p konduit-client -- -y l2 cln --payee "http://127.0.0.1:$KONDUIT_SERVER_MERCHANT_PORT" --amount 1000'
```

Inspect the channels and balances:

```sh
nix --extra-experimental-features 'nix-command flakes fetch-closure' --accept-flake-config develop path:./flake/heavy --command scripts/show-balances.sh
```

Close and end both channels. Run the teardown helper first. In this run, it
closed gateway-to-merchant, but timed out waiting for `closed`: the channel
had moved directly to `responded` with no pending amount. It exits at that
point, so continue with `l1 step` for that channel, then close and step
consumer-to-gateway:

```sh
nix --extra-experimental-features 'nix-command flakes fetch-closure' --accept-flake-config develop path:./flake/heavy --command scripts/teardown-cln-channels.sh
```

```sh
nix --extra-experimental-features 'nix-command flakes fetch-closure' --accept-flake-config develop path:./flake/heavy --command bash -c 'source "$RUN_DIR/cln-channels.env"; KONDUIT_SIGNING_KEY="$KONDUIT_GATEWAY_WALLET" KONDUIT_TAG="$KONDUIT_GATEWAY_MERCHANT_TAG" KONDUIT_SERVER_URL="$KONDUIT_MERCHANT_URL" cargo run -q -p konduit-client -- -y l1 step'
```

```sh
nix --extra-experimental-features 'nix-command flakes fetch-closure' --accept-flake-config develop path:./flake/heavy --command bash -c 'source "$RUN_DIR/cln-channels.env"; KONDUIT_SIGNING_KEY="$KONDUIT_CONSUMER_WALLET" KONDUIT_TAG="$KONDUIT_CONSUMER_GATEWAY_TAG" KONDUIT_SERVER_URL="$KONDUIT_GATEWAY_URL" cargo run -q -p konduit-client -- -y l1 close'
```

```sh
nix --extra-experimental-features 'nix-command flakes fetch-closure' --accept-flake-config develop path:./flake/heavy --command bash -c 'source "$RUN_DIR/cln-channels.env"; KONDUIT_SIGNING_KEY="$KONDUIT_CONSUMER_WALLET" KONDUIT_TAG="$KONDUIT_CONSUMER_GATEWAY_TAG" KONDUIT_SERVER_URL="$KONDUIT_GATEWAY_URL" cargo run -q -p konduit-client -- -y l1 step'
```

Confirm both channel lists are empty:

```sh
nix --extra-experimental-features 'nix-command flakes fetch-closure' --accept-flake-config develop path:./flake/heavy --command scripts/show-balances.sh
```

The demo environment remains running until stopped with:

```sh
nix --extra-experimental-features 'nix-command flakes fetch-closure' --accept-flake-config develop path:./flake/heavy --command process-compose down
```

The CLN demo helpers live in [`scripts/`](scripts/):
[`fund-cln-wallets.sh`](scripts/fund-cln-wallets.sh),
[`submit-null-squash.sh`](scripts/submit-null-squash.sh),
[`show-balances.sh`](scripts/show-balances.sh), and
[`teardown-cln-channels.sh`](scripts/teardown-cln-channels.sh).

For details on the preprod guest, see
[`nixos/README.md`](nixos/README.md).

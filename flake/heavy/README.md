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

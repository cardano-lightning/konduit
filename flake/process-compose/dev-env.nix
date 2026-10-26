{
  cardano-cli,
  cardano-node,
  cardonnay,
  coreutils,
  formats,
  jq,
  kupo,
  lib,
  postgresql,
  sqitchPg,
  writeShellApplication,
  writeText,
}: let
  testnet-processes = import ./testnet-processes.nix {
    inherit lib coreutils writeShellApplication writeText formats cardonnay cardano-node cardano-cli;
  };

  validate-dev-env = writeShellApplication {
    name = "validate-testnet-env";
    runtimeInputs = [coreutils];
    # Check if all variables are defined and if not stop the process.
    text = ''
      set -euo pipefail
      set -x
      : "''${TESTNET_DIR:?}"
      : "''${CARDONNAY_TESTNET_ID:?}"
      : "''${CARDANO_NODE_NETWORK_ID:?}"
      : "''${CARDANO_NODE_SOCKET_PATH:?}"
      : "''${CARDANO_NODE_CONFIG_PATH:?}"
      : "''${KONDUIT_VALIDATOR_HASH:?}"
      : "''${KUPO_INDEXER_DIR:?}"
      : "''${KUPO_INDEXER_PORT:?}"
      : "''${KONDUIT_INDEXER_DB_PATH:?}"
      : "''${KUPO_UTXO_DIR:?}"
      : "''${KUPO_UTXO_PORT:?}"
      : "''${KONDUIT_SERVER_DB_PATH:?}"
      : "''${KONDUIT_SERVER_HOST:?}"
      : "''${KONDUIT_SERVER_PORT:?}"
      : "''${KONDUIT_FX_EVERY:?}"

      # Check if .env.adaptor exists and is readable
      if [ ! -r .env.adaptor ]; then
        echo ".env.adaptor file is missing or not readable. Please create it with the required variables."
        exit 1
      fi
    '';
  };

  # curl -H 'Accept:application/json' 'http://127.0.0.1:1442/health'

  kupo-readiness-probe = writeShellApplication {
    name = "kupo-readiness-probe";
    runtimeInputs = [coreutils];
    text = ''
      set -x
      : "''${KUPO_PORT:?}"
      STATUS="$(curl -H 'Accept:application/json' "http://127.0.0.1:$KUPO_PORT/health"  | jq -r '.connection_status')"
      if [ "$STATUS" != "connected" ]; then
        echo "Kupo is not ready. Status: $STATUS"
        exit 1
      fi
    '';
  };

  kupo-indexer = writeShellApplication {
    name = "kupo-indexer";
    runtimeInputs = [kupo];
    text = ''
      set -x
      kupo \
        --node-socket "$CARDANO_NODE_SOCKET_PATH" \
        --node-config "$CARDANO_NODE_CONFIG_PATH" \
        --workdir "$KUPO_INDEXER_DIR" \
        --since origin \
        --match "$KONDUIT_VALIDATOR_HASH/*" \
        --log-level Debug
    '';
  };

  kupo-utxo = writeShellApplication {
    name = "kupo-utxo";
    runtimeInputs = [kupo];
    text = ''
      set -x
      kupo \
        --node-socket "$CARDANO_NODE_SOCKET_PATH" \
        --node-config "$CARDANO_NODE_CONFIG_PATH" \
        --prune-utxo \
        --workdir "$KUPO_UTXO_DIR" \
        --port "$KUPO_UTXO_PORT" \
        --match "*/*" \
        --since origin \
        --log-level Error
    '';
  };

  konduit-indexer = writeShellApplication {
    name = "konduit-indexer";
    runtimeInputs = [cardonnay];
    text = ''
      set -x
      export CARGO_TARGET_DIR="''${CARGO_TARGET_DIR:-./.run/cargo-target-indexer}"
      mkdir -p "$CARGO_TARGET_DIR"
      mkdir -p "$(dirname "$KONDUIT_INDEXER_DB_PATH")"
      cargo run -p konduit-indexer --features=cli -- \
        index \
        --kupo-port "$KUPO_INDEXER_PORT" \
        --db-path "$KONDUIT_INDEXER_DB_PATH"
    '';
  };

  # Publishes the konduit reference script to the local testnet.
  #
  # The server refuses to start without it: the admin service looks the script
  # up at KONDUIT_HOST_ADDRESS and errors out when it is not there. The testnet
  # is recreated on every `process-compose up` (clear-testnet-state wipes
  # $TESTNET_DIR), so the script has to be published again every time.
  publish-reference-script = writeShellApplication {
    name = "publish-reference-script";
    runtimeInputs = [cardano-cli jq];
    text = ''
      set -euo pipefail
      set -x

      : "''${CARDANO_NODE_SOCKET_PATH:?}"
      : "''${CARDANO_NODE_NETWORK_ID:?}"
      : "''${FAUCET_ADDR_FILE:?}"
      : "''${FAUCET_SKEY_FILE:?}"

      export CARGO_TARGET_DIR="''${CARGO_TARGET_DIR:-./.run/cargo-target-cli}"
      mkdir -p "$CARGO_TARGET_DIR"

      # The konduit cli reads the .env files of the repo itself, but the cardano
      # connection has to point at the testnet of this run, not at whatever the
      # .env files happen to carry.
      export KONDUIT_CARDANO_BACKEND="node"
      export KONDUIT_NODE_SOCKET="$CARDANO_NODE_SOCKET_PATH"
      export KONDUIT_NETWORK="$CARDANO_NODE_NETWORK_ID"

      # 200 ada for the admin wallet. The deployment pays the fee and the
      # reference script output has to carry its min ada, so a few tens of
      # ada are needed at least.
      ADMIN_FUNDS="200000000"
      # Below this the admin wallet cannot pay for the deployment.
      ADMIN_MIN_LOVELACE="50000000"

      konduit_cli() {
        cargo run -q -p konduit-cli -- "$@"
      }

      # The addresses, as the cli resolves them. The cli reads the .env files
      # of the repo itself, so these are the addresses the server uses as
      # well: the admin wallet pays for the deployment, the host address is
      # where the reference script ends up.
      admin_config="$(konduit_cli admin show config)"
      admin_address="$(sed -n 's/^address = //p' <<< "$admin_config")"
      host_address="$(sed -n 's/^host_address = //p' <<< "$admin_config")"
      : "''${admin_address:?publish-reference-script needs the admin address}"
      : "''${host_address:?publish-reference-script needs the host address}"

      # The reference script is published at the host address. The server
      # looks for it there, so its presence is what this process is about.
      script_published() {
        cardano-cli query utxo --address "$host_address" \
          --testnet-magic "$CARDANO_NODE_NETWORK_ID" \
          --socket-path "$CARDANO_NODE_SOCKET_PATH" \
          --output-json \
          | jq -e 'to_entries | map(select(.value.referenceScript != null)) | length > 0' > /dev/null
      }

      if script_published; then
        echo "the reference script is already published at $host_address"
        exit 0
      fi

      # The lovelace the admin wallet holds.
      admin_balance() {
        cardano-cli query utxo --address "$admin_address" \
          --testnet-magic "$CARDANO_NODE_NETWORK_ID" \
          --socket-path "$CARDANO_NODE_SOCKET_PATH" \
          --output-json \
          | jq '[.[] | .value.lovelace] | add // 0'
      }

      if [ "$(admin_balance)" -lt "$ADMIN_MIN_LOVELACE" ]; then
        faucet_address="$(cat "$FAUCET_ADDR_FILE")"
        faucet_utxo="$(cardano-cli query utxo --address "$faucet_address" \
          --testnet-magic "$CARDANO_NODE_NETWORK_ID" \
          --socket-path "$CARDANO_NODE_SOCKET_PATH" \
          --output-json | jq -r 'keys[0] // empty')"
        if [ -z "$faucet_utxo" ]; then
          echo "the faucet $faucet_address has no utxo to fund $admin_address with" >&2
          exit 1
        fi

        tx_body="$(mktemp)"
        tx_signed="$(mktemp)"
        cardano-cli conway transaction build \
          --testnet-magic "$CARDANO_NODE_NETWORK_ID" \
          --socket-path "$CARDANO_NODE_SOCKET_PATH" \
          --tx-in "$faucet_utxo" \
          --tx-out "$admin_address+$ADMIN_FUNDS" \
          --change-address "$faucet_address" \
          --out-file "$tx_body"
        cardano-cli conway transaction sign \
          --tx-body-file "$tx_body" \
          --signing-key-file "$FAUCET_SKEY_FILE" \
          --testnet-magic "$CARDANO_NODE_NETWORK_ID" \
          --out-file "$tx_signed"
        cardano-cli conway transaction submit \
          --testnet-magic "$CARDANO_NODE_NETWORK_ID" \
          --socket-path "$CARDANO_NODE_SOCKET_PATH" \
          --tx-file "$tx_signed"

        # The deployment spends the funding, so wait for it to confirm.
        for _ in $(seq 60); do
          if [ "$(admin_balance)" -ge "$ADMIN_MIN_LOVELACE" ]; then
            break
          fi
          sleep 2
        done
        if [ "$(admin_balance)" -lt "$ADMIN_MIN_LOVELACE" ]; then
          echo "the funding of $admin_address did not confirm" >&2
          exit 1
        fi
      fi

      # Publish the reference script at the host address.
      konduit_cli admin tx deploy

      # Wait for the script to show up, the server looks for it there.
      for _ in $(seq 60); do
        if script_published; then
          konduit_cli admin show tip
          exit 0
        fi
        sleep 2
      done

      echo "the reference script did not show up at $host_address" >&2
      exit 1
    '';
  };

  konduit-server = writeShellApplication {
    name = "konduit-server";
    runtimeInputs = [];
    text = ''
      set -x
      export CARGO_TARGET_DIR="''${CARGO_TARGET_DIR:-./.run/cargo-target-server}"
      mkdir -p "$CARGO_TARGET_DIR"
      mkdir -p "$(dirname "$KONDUIT_SERVER_DB_PATH")"

      # Cardano: the local testnet. Variables exported here win over the .env
      # and .env.adaptor files the server loads itself.
      export KONDUIT_CARDANO_BACKEND="node"
      export KONDUIT_NODE_SOCKET="$CARDANO_NODE_SOCKET_PATH"
      export KONDUIT_NODE_NETWORK_MAGIC="$CARDANO_NODE_NETWORK_ID"
      # A local testnet is not a public network, but the value is parsed even
      # though the node backend never looks at it.
      export KONDUIT_NETWORK="''${KONDUIT_NETWORK_NAME:-preview}"

      # BLN: mandatory. Falls back to the mock backend unless the dev env
      # points at an LND.
      if [ -n "''${LND_BASE_URL:-}" ] && [ -n "''${LND_MACAROON:-}" ]; then
        unset BLN_MOCK
      else
        export BLN_MOCK="true"
      fi

      # FX: fixed rates unless the dev env brings a live source.
      export FX_EVERY="''${KONDUIT_FX_EVERY:-10m}"
      export FX_ADA="''${FX_ADA:-''${KONDUIT_FX_ADA:-0.6}}"
      export FX_BITCOIN="''${FX_BITCOIN:-''${KONDUIT_FX_BITCOIN:-60000}}"

      export KONDUIT_DB_PATH="$KONDUIT_SERVER_DB_PATH"

      # Started from the repo root, so the server picks up .env and .env.adaptor.
      cargo run -p konduit-server
    '';
  };
in
  (formats.yaml {}).generate "process-compose.yaml" {
    version = "0.5";
    log_location = ".pc.log";
    processes =
      testnet-processes
      // {
        validate-dev-env = {
          command = "${validate-dev-env}/bin/validate-testnet-env";
          log_location = "./.run/validate-dev-env.log";
          namespace = "indexers";
        };

        kupo-indexer = {
          depends_on = {
            "validate-dev-env".condition = "process_completed_successfully";
            "initialize-testnet".condition = "process_healthy";
          };
          command = "${kupo-indexer}/bin/kupo-indexer";
          log_location = "./.run/kupo-indexer.log";
          namespace = "indexers";
          readiness_probe = {
            exec = {
              command = "KUPO_PORT=$KUPO_INDEXER_PORT ${kupo-readiness-probe}/bin/kupo-readiness-probe";
            };
            initial_delay_seconds = 10; # after we reduced the internal sleep
            period_seconds = 2;
            timeout_seconds = 5;
            success_threshold = 1;
            failure_threshold = 300;
          };
        };

        kupo-utxo = {
          command = "${kupo-utxo}/bin/kupo-utxo";
          depends_on = {
            "validate-dev-env".condition = "process_completed_successfully";
            "initialize-testnet".condition = "process_healthy";
          };
          log_location = "./.run/kupo-utxo.log";
          namespace = "indexers";
          readiness_probe = {
            exec = {
              command = "KUPO_PORT=$KUPO_UTXO_PORT ${kupo-readiness-probe}/bin/kupo-readiness-probe";
            };
            initial_delay_seconds = 10; # after we reduced the internal sleep
            period_seconds = 2;
            timeout_seconds = 5;
            success_threshold = 1;
            failure_threshold = 300;
          };
        };

        konduit-indexer = {
          depends_on = {
            "validate-dev-env".condition = "process_completed_successfully";
            "kupo-indexer".condition = "process_healthy";
          };
          command = "${konduit-indexer}/bin/konduit-indexer";
          log_location = "./.run/konduit-indexer.log";
          namespace = "indexers";
          schedule = {
            interval = "10s";
            run_on_start = true;
            max_concurrent = 1;
          };
        };

        publish-reference-script = {
          depends_on = {
            "validate-dev-env".condition = "process_completed_successfully";
            "initialize-testnet".condition = "process_healthy";
            "set-faucet-info".condition = "process_completed_successfully";
          };
          command = "${publish-reference-script}/bin/publish-reference-script";
          log_location = "./.run/publish-reference-script.log";
          namespace = "server";
        };

        konduit-server = {
          depends_on = {
            "validate-dev-env".condition = "process_completed_successfully";
            "initialize-testnet".condition = "process_healthy";
            "publish-reference-script".condition = "process_completed_successfully";
          };
          command = "${konduit-server}/bin/konduit-server";
          log_location = "./.run/konduit-server.log";
          namespace = "server";
        };
      };
  }

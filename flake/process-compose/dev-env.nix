{
  cardano-cli,
  cardano-node,
  cardonnay,
  coreutils,
  formats,
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

  konduit-server = writeShellApplication {
    name = "konduit-server";
    runtimeInputs = [];
    text = ''
      set -x
      # The server refuses to start without an adaptor signing key. It is a dev
      # secret, so it is not kept in the .env files of the repo.
      : "''${KONDUIT_SIGNING_KEY:?konduit-server needs KONDUIT_SIGNING_KEY}"

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

        konduit-server = {
          depends_on = {
            "validate-dev-env".condition = "process_completed_successfully";
            "initialize-testnet".condition = "process_healthy";
          };
          command = "${konduit-server}/bin/konduit-server";
          log_location = "./.run/konduit-server.log";
          namespace = "server";
        };
      };
  }

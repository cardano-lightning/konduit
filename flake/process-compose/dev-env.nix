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
      cargo run -p konduit-indexer --features=cli -- \
        index \
        --kupo-port "$KUPO_INDEXER_PORT" \
        --db-path "$KONDUIT_INDEXER_DB_PATH"
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
      };
  }

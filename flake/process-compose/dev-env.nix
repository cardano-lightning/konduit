{
  cardano-cli,
  cardano-node,
  cardonnay,
  coreutils,
  formats,
  lib,
  postgresql,
  sqitchPg,
  writeShellApplication,
  writeText,
}: let
  testnet-processes = import ./testnet-processes.nix {
    inherit lib coreutils writeShellApplication writeText formats cardonnay cardano-node cardano-cli;
  };
  postgres-processes = import ./postgres-processes.nix {
    inherit lib postgresql coreutils writeText writeShellApplication sqitchPg;
  };

  validate-dev-env = writeShellApplication {
    name = "validate-testnet-env";
    runtimeInputs = [coreutils];
    # Check if all variables are defined and if not stop the process.
    text = ''
      set -euo pipefail
      set -x
      : "''${TESTNET_DIR:?}"
      : "''${POSTGRES_DIR:?}"
      : "''${SQITCH_CHDIR:?}"
      : "''${CARDANO_NODE_NETWORK_ID:?}"
      : "''${CARDANO_NODE_SOCKET_PATH:?}"
      : "''${FAUCET_SKEY_FILE:?}"
      : "''${FAUCET_ADDR_FILE:?}"
      : "''${MARLOWE_PUBLISHING_INFO_FILE:?}"
    '';
  };
  marlowe-db = writeShellApplication {
    name = "marlowe-db";
    runtimeInputs = [sqitchPg];
    text = ''
      set -x
      function psql_with_args() {
        psql -v "ON_ERROR_STOP=1" "$@"
      }
      export PGDATA="$POSTGRES_DIR/pgdata"
      echo "CREATE DATABASE marlowe;" | psql_with_args -d postgres -p "$PGPORT" -h "localhost"
      echo "CREATE USER marlowe;" | psql_with_args -d postgres -p "$PGPORT" -h "localhost"
      echo "ALTER DATABASE marlowe OWNER TO marlowe;" | psql_with_args -d postgres -p "$PGPORT" -h "localhost"
      sqitch rebase db:pg://marlowe@localhost/marlowe --chdir "$SQITCH_CHDIR" || sqitch deploy db:pg://marlowe@localhost/marlowe --chdir "$SQITCH_CHDIR"
    '';
  };

  publish-marlowe = writeShellApplication {
    name = "publish-marlowe";
    runtimeInputs = [cardano-cli cardano-node coreutils];
    text = ''
      set -x
      cabal run marlowe-cli -- --conway-era \
        transaction publish \
        --testnet-magic "$CARDANO_NODE_NETWORK_ID" \
        --socket-path "$CARDANO_NODE_SOCKET_PATH" \
        --required-signer "$FAUCET_SKEY_FILE" \
        --change-address "$(cat "$FAUCET_ADDR_FILE")" \
        --permanently-without-staking \
        --out-tx-file publish-tx.json \
        --message-format json \
        --submit 120 2>/dev/null > "$MARLOWE_PUBLISHING_INFO_FILE"
    '';
  };

  marlowe-indexer = writeShellApplication {
    name = "marlowe-indexer";
    text = ''
      args=(
        --database-uri "postgresql://localhost:''${PGPORT:-15432}/marlowe"
        --verbose
      )
      exec cabal run marlowe-indexer -- "''${args[@]}"
    '';
  };
in
  (formats.yaml {}).generate "process-compose.yaml" {
    version = "0.5";
    log_location = ".pc.log";
    processes =
      testnet-processes
      // postgres-processes
      // {
        validate-dev-env = {
          namespace = "dev-env";
          log_location = "./.run/validate-dev-env.log";
          command = "${validate-dev-env}/bin/validate-testnet-env";
        };

        publish-marlowe = {
          namespace = "marlowe";
          log_location = "./.run/publish-marlowe.log";
          command = "${publish-marlowe}/bin/publish-marlowe";
          depends_on = {
            "validate-testnet-env".condition = "process_completed_successfully";
            "initialize-testnet".condition = "process_healthy";
            "set-faucet-info".condition = "process_completed_successfully";
          };
        };

        marlowe-db = {
          namespace = "marlowe";
          command = "${marlowe-db}/bin/marlowe-db";
          depends_on = {
            "validate-dev-env".condition = "process_completed_successfully";
            "postgres-server" = {
              condition = "process_healthy";
            };
          };
        };

        marlowe-indexer = {
          namespace = "marlowe";
          log_location = "./.run/marlowe-indexer.log";
          command = "${marlowe-indexer}/bin/marlowe-indexer";
          depends_on = {
            "marlowe-db".condition = "process_completed_successfully";
            "publish-marlowe".condition = "process_completed_successfully";
          };
        };
      };
  }

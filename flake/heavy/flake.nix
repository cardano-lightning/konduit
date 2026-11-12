{
  description = "Konduit Cardano/testnet development shell";

  inputs = {
    capkgs.url = "github:input-output-hk/capkgs/c9c88ae5e7595ba56bed7cf6e73a1d9727a6433e";
    cardano-node.url = "github:IntersectMBO/cardano-node/11.0.1";
    cardonnay-src = {
      url = "github:IntersectMBO/cardonnay?ref=v0.3.6";
      flake = false;
    };
    flake-parts.url = "github:hercules-ci/flake-parts";
    jailed-agents.url = "github:andersonjoseph/jailed-agents";
    kupo.url = "github:paluh/kupo/konduit-kupo";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    konduit.url = "github:cardano-lightning/konduit";
    konduit.inputs.nixpkgs.follows = "nixpkgs";
  };

  nixConfig = {
    extra-substituters = ["https://cache.iog.io"];
    extra-trusted-public-keys = ["hydra.iohk.io:f/Ea+s+dFdN+3Y/G+FDgSq+a5NEWhJGzdjvKNGv0/EQ="];
  };

  outputs = inputs @ {flake-parts, ...}:
    flake-parts.lib.mkFlake {inherit inputs;} ({lib, ...}: {
      systems = ["x86_64-linux" "aarch64-darwin"];

      perSystem = {
        pkgs,
        system,
        ...
      }: let
        cardonnay = pkgs.python313.pkgs.buildPythonApplication {
          pname = "cardonnay";
          version = "0.3.6";
          SETUPTOOLS_SCM_PRETEND_VERSION_FOR_CARDONNAY = "0.3.6";
          src = inputs.cardonnay-src;
          pyproject = true;
          build-system = with pkgs.python313.pkgs; [setuptools setuptools-scm];
          pythonRelaxDeps = ["setuptools"];
          nativeBuildInputs = with pkgs.python313.pkgs; [pythonRelaxDepsHook];
          dependencies = with pkgs.python313.pkgs; [supervisor click pygments pydantic filelock];
        };
        # Use the node version paired with the flake input. The older capkgs
        # package only forges protocol version 10, which cannot run a devnet
        # configured for Aiken 1.1.24's Plutus V3 Value builtins.
        cardano-cli = inputs.cardano-node.packages.${system}.cardano-cli;
        cardano-node = inputs.cardano-node.packages.${system}.cardano-node;
        kupo = inputs.kupo.packages.${system}.kupo;
        process-compose-yaml = pkgs.callPackage ./process-compose.nix {
          inherit cardonnay cardano-cli cardano-node kupo;
          curl = pkgs.curl;
        };
        process-compose-dev-env = pkgs.writeShellApplication {
          name = "process-compose-dev-env";
          text = ''
            ${pkgs.process-compose}/bin/process-compose up -f ${process-compose-yaml} -L "$RUN_DIR/process-compose-dev-env"
          '';
        };
      in {
        devShells.default = pkgs.mkShell {
          name = "konduit-iog-shell";
          packages = [
            cardano-cli
            cardano-node
            cardonnay
            kupo
            process-compose-dev-env
            pkgs.coreutils
            pkgs.curl
            pkgs.gcc
            pkgs.jq
            pkgs.libiconv
            pkgs.process-compose
          ];
          shellHook = ''
            echo 1>&2 "Welcome to the Konduit IOG development shell!"
            export ROOT_DIR="$(git rev-parse --show-toplevel)"
            export RUN_DIR="''${KONDUIT_RUN_DIR:-$ROOT_DIR/.run}"
            export TESTNET_DIR="$RUN_DIR/testnet"
            export CARDONNAY_TESTNET_ID=9
            export KUPO_INDEXER_DIR="$RUN_DIR/kupo-indexer"
            export KUPO_INDEXER_PORT=1442
            export KUPO_UTXO_DIR="$RUN_DIR/kupo-utxo"
            export KUPO_UTXO_PORT=1443
            export KONDUIT_INDEXER_DIR="$RUN_DIR/konduit-indexer"
            export KONDUIT_INDEXER_DB_PATH="$KONDUIT_INDEXER_DIR/konduit-indexer.db"
            export KONDUIT_INDEXER_DB_TIMEOUT_MS=120000
            export KONDUIT_INDEXER_DB_POLL_MS=1000
            export KONDUIT_INDEXER_DB_BUSY_TIMEOUT_MS=5000
            export KONDUIT_INDEXER_SCHEMA_DIR="$ROOT_DIR/packages/konduit/indexer/sqlite"
            export KONDUIT_SERVER_DIR="$RUN_DIR/konduit-server"
            export KONDUIT_SERVER_DB_PATH="$KONDUIT_SERVER_DIR/konduit-server.db"
            export KONDUIT_SERVER_HOST=127.0.0.1
            export KONDUIT_SERVER_PORT=5663
            export KONDUIT_SERVER_GATEWAY_PORT=5663
            export KONDUIT_SERVER_MERCHANT_PORT=5664
            export CLN_DEMO_CONFIG_DIR="$RUN_DIR/cln-demo"
            export KONDUIT_BLN_ENV_FILE="''${KONDUIT_BLN_ENV_FILE:-$ROOT_DIR/.env.bln}"
            export KONDUIT_FX_EVERY=10m
            source <(cardonnay control print-env -i "$CARDONNAY_TESTNET_ID" -w "$TESTNET_DIR")
            NODE_SOCKET_DIR="''${CARDANO_NODE_SOCKET_PATH%/*}"
            NODE_SOCKET_NAME="''${CARDANO_NODE_SOCKET_PATH##*/}"
            export CARDANO_NODE_CONFIG_PATH="$NODE_SOCKET_DIR/config-''${NODE_SOCKET_NAME%.socket}.json"
            export CARDANO_NODE_NETWORK_ID=42
            export KONDUIT_VALIDATOR_HASH="$(jq -r '.validators[0].hash' "$ROOT_DIR/packages/kernel/plutus.json")"
            export FAUCET_ADDR_FILE="$TESTNET_DIR/faucet.addr"
            export FAUCET_SKEY_FILE="$TESTNET_DIR/faucet.skey"
            export FAUCET_WALLET_FILE="$TESTNET_DIR/faucet.wallet"
            export PROCESS_COMPOSE_YAML=${process-compose-yaml}
            export PC_PORT_NUM=8100
          '';
        };
      };

      flake = let
        nixos = import ./nixos/outputs.nix {
          self = inputs.konduit;
          nixpkgs = inputs.nixpkgs;
        };
        requiredEnv = name: let
          value = builtins.getEnv name;
        in
          if value == ""
          then throw "${name} is not set"
          else value;
      in {
        nixosModules =
          nixos.nixosModules
          // {
            default = import ./nixos.nix inputs.konduit;
          };

        nixosConfigurations.preprod = nixos.mkPreprodDeployment {
          system = "x86_64-linux";
          cardanoNodeModule = inputs.cardano-node.nixosModules.cardano-node;
          cardanoCli = inputs.cardano-node.packages.x86_64-linux.cardano-cli;
          kupoPackage = inputs.kupo.packages.x86_64-linux.kupo;
          indexerPackage = inputs.konduit.packages.x86_64-linux.konduit-indexer;
          serverPackage = inputs.konduit.packages.x86_64-linux.konduit-server;
          hostSnapshot = requiredEnv "KONDUIT_PREPROD_DB";
          hostUid = lib.toInt (requiredEnv "KONDUIT_PREPROD_DB_OWNER_UID");
          hostEnvFile = requiredEnv "KONDUIT_SERVER_ENV_FILE";
        };
      };
    });
}

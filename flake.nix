{
  description = "Konduit: Cardano Lightning Network payments";

  inputs = {
    aiken.url = "github:aiken-lang/aiken";
    flake-parts.url = "github:hercules-ci/flake-parts";
    git-hooks-nix.url = "github:cachix/git-hooks.nix";
    git-hooks-nix.inputs.nixpkgs.follows = "nixpkgs";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-flake = {
      url = "github:juspay/rust-flake";
      inputs.rust-overlay.follows = "rust-overlay";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    treefmt-nix.url = "github:numtide/treefmt-nix";
    treefmt-nix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = inputs @ {flake-parts, ...}:
    flake-parts.lib.mkFlake {inherit inputs;}
    ({
      lib,
      self,
      ...
    }: {
      imports = [
        inputs.git-hooks-nix.flakeModule
        inputs.treefmt-nix.flakeModule
        inputs.rust-flake.flakeModules.default
        inputs.rust-flake.flakeModules.nixpkgs
      ];
      systems = ["x86_64-linux" "aarch64-darwin"];

      perSystem = {
        lib,
        config,
        inputs',
        pkgs,
        system,
        ...
      }: let
        shellHookCommon = ''
          ${config.pre-commit.installationScript}
          export RUST_SRC_PATH="${config.rust-project.toolchain}/lib/rustlib/src/rust/library";
        '';
        clang-unwrapped = pkgs.llvmPackages_latest.clang-unwrapped;
        wasm-pack = pkgs.callPackage ./flake/wasm-pack.nix {};
        packages =
          [
            inputs'.aiken.packages.aiken
            # JS
            pkgs.nodejs
            pkgs.typescript-language-server
            pkgs.yarn-berry
            pkgs.yarn-berry_4.yarn-berry-fetcher
            pkgs.yarn-bash-completion

            # RUST
            pkgs.openssl
            config.rust-project.toolchain
            wasm-pack
            clang-unwrapped
            pkgs.cargo-machete
            # TOOLS NEEDED BY SOME RUST BUILDS
            pkgs.perl
            pkgs.cmake
            pkgs.go
            pkgs.nasm
            # PRE-COMMIT
            pkgs.prek
            # UTILS
            pkgs.just
            # DOC BUILING
            pkgs.pandoc
            pkgs.d2
          ]
          ++ lib.concatMap (crate: crate.crane.args.nativeBuildInputs) (lib.attrValues config.rust-project.crates);

        devShell = {
          name = "konduit-shell";
          shellHook = ''
            echo 1>&2 "Welcome to the development shell!"
            ${shellHookCommon}
          '';

          packages =
            packages
            ++ [pkgs.process-compose];

          nativeBuildInputs =
            [config.treefmt.build.wrapper]
            ++ lib.concatMap (crate: crate.crane.args.nativeBuildInputs) (lib.attrValues config.rust-project.crates);

          buildInputs =
            [pkgs.libiconv]
            ++ lib.concatMap (crate: crate.crane.args.buildInputs) (lib.attrValues config.rust-project.crates);

          CC_wasm32_unknown_unknown = lib.getExe' clang-unwrapped "clang";
        };

        process-compose-dev-env-yaml = "";

        devShellExtras =
          devShell
          // {
            name = "konduit-shell-with-extras";

            shellHook = ''
              echo 1>&2 "Welcome to the development shell with extras!"

              ${shellHookCommon}

              # Most (not all yet) setup of the process-compose subprocesses is
              # done through the env variables. Please use this convention
              # so we can have more testing flexibility around the local dev env.

              export ROOT_DIR="$(git rev-parse --show-toplevel)"
              export RUN_DIR="''${KONDUIT_RUN_DIR:-$ROOT_DIR/.run}"

              # Vars required by testnet part of the process compose:

              export TESTNET_DIR="$RUN_DIR/testnet"

              export CARDONNAY_TESTNET_ID="9"

              export KUPO_INDEXER_DIR="$RUN_DIR/kupo-indexer"
              export KUPO_INDEXER_PORT=1442

              export KUPO_UTXO_DIR="$RUN_DIR/kupo-utxo"
              export KUPO_UTXO_PORT=1443

              export KONDUIT_INDEXER_DIR="$RUN_DIR/konduit-indexer"
              export KONDUIT_INDEXER_DB_PATH="$KONDUIT_INDEXER_DIR/konduit-indexer.db"
              # Read-only knobs for the integration suite's db-check layer
              # (packages/integration). The suite opens $KONDUIT_INDEXER_DB_PATH
              # read-only and asserts the indexer persisted each channel/step.
              # `auto` below is expressed as an unset variable: the checks run
              # whenever the db file exists, and skip with a printed reason when
              # it does not.
              # export KONDUIT_INDEXER_DB_CHECKS=1   # 1 | 0 | unset (auto)
              export KONDUIT_INDEXER_DB_TIMEOUT_MS=120000
              export KONDUIT_INDEXER_DB_POLL_MS=1000
              export KONDUIT_INDEXER_DB_BUSY_TIMEOUT_MS=5000
              export KONDUIT_INDEXER_SCHEMA_DIR="$ROOT_DIR/packages/konduit/indexer/sqlite"
              # Echo every konduit-cli / cardano-cli command line and every SQL
              # statement the integration suite runs.
              # export KONDUIT_E2E_DEBUG=1

              export KONDUIT_SERVER_DIR="$RUN_DIR/konduit-server"
              export KONDUIT_SERVER_DB_PATH="$KONDUIT_SERVER_DIR/konduit-server.db"
              export KONDUIT_SERVER_HOST="127.0.0.1"
              export KONDUIT_SERVER_PORT=5663
              export KONDUIT_SERVER_GATEWAY_PORT=5663
              export KONDUIT_SERVER_MERCHANT_PORT=5664
              export CLN_DEMO_CONFIG_DIR="$RUN_DIR/cln-demo"
              # The server refreshes its FX state on a timer, and a zero
              # interval makes it panic on start.
              export KONDUIT_FX_EVERY="10m"


              # This exposes CARDANO_NODE_SOCKET_PATH to the shell
              source <(cardonnay control print-env -i "$CARDONNAY_TESTNET_ID" -w "$TESTNET_DIR")

              # Removes the socket file name from the path, leaving only the directory
              NODE_SOCKET_DIR="''${CARDANO_NODE_SOCKET_PATH%/*}"
              NODE_SOCKET_NAME="''${CARDANO_NODE_SOCKET_PATH##*/}"
              export CARDANO_NODE_CONFIG_PATH="$NODE_SOCKET_DIR/config-''${NODE_SOCKET_NAME%.socket}.json"

              CONFIG_DIR=$(dirname "$CARDANO_NODE_CONFIG_PATH")
              # This flow assumed that we have already the testnet run directory which is not the case
              # for the initial setup. For now we will hardcode the network id to 42, but we should fix this in the future.
              # SHELLEY_GENESIS=$(jq -r .ShelleyGenesisFile "$CARDANO_NODE_CONFIG_PATH")
              export CARDANO_NODE_NETWORK_ID=42 # $(jq -r .networkMagic "$CONFIG_DIR/$SHELLEY_GENESIS")
              export KONDUIT_VALIDATOR_HASH="$(cat $ROOT_DIR/packages/kernel/plutus.json | jq -r '.validators[0].hash')"

              # This **will be** initialized by the testnet process compose when executed
              export FAUCET_ADDR_FILE="$TESTNET_DIR/faucet.addr"
              # Faucet skey in a cardano-cli JSON format.
              export FAUCET_SKEY_FILE="$TESTNET_DIR/faucet.skey"
              # The same as above key but in a raw 64 hex format
              export FAUCET_WALLET_FILE="$TESTNET_DIR/faucet.wallet"

              export PROCESS_COMPOSE_YAML=${process-compose-dev-env-yaml}
              # This var is looked up by the process compose itself.
              export PC_PORT_NUM=8100
            '';

            packages = devShell.packages;
          };
      in {
        rust-project = {
          crates = {
            konduit-indexer = {
              # The binary is behind the cli feature, so a plain build of the
              # crate would not produce it.
              crane.args.cargoBuildExtraArgs = "--features cli";
            };
            konduit-server = {
              crane = {
                args = {
                  nativeBuildInputs = [pkgs.pkg-config pkgs.openssl.dev];
                  buildInputs = [pkgs.openssl.dev];
                };
              };
            };
          };
        };
        treefmt = {
          projectRootFile = "flake.nix";
          flakeFormatter = true;
          # Generated files. Formatting these breaks pre-commit hooks.
          settings.excludes = ["treefmt.toml" ".pre-commit-config.yaml"];
          programs = {
            aiken.enable = true;
            alejandra.enable = true;
            prettier.enable = true;
            rustfmt.enable = true;
            taplo.enable = true;
          };
        };

        pre-commit = let
          nixPrekConfig = ".nix-prek-config.yaml";
          precommitConfig = ".pre-commit-config.yaml";
          treefmtConfig = "treefmt.toml";
        in {
          # clippy checks are failing `nix flake check`
          # However, they come from rust-flakes, and our implicit workspace
          # makes it awkward to turn these off
          check.enable = false;
          settings = {
            package = pkgs.prek;
            configPath = nixPrekConfig;
            hooks = {
              treefmt.enable = true;
              nix-sync = {
                enable = true;
                name = "nix-sync";
                description = "Copy nix-generated prek config to committed ${precommitConfig}. This strips the nixstore dependencies";
                entry = ''
                  sh -c '
                    if [ -f ${nixPrekConfig} ]; then
                      # PRE-COMMIT
                      grep -v "^#" ${nixPrekConfig} | jq ".repos[].hooks[].entry |= gsub(\"/nix/store/[^/]+/bin/\"; \"\")" > ${precommitConfig}
                      git add ${precommitConfig}
                      # TREEFMT
                      sed "s|/nix/store/[^/]*/bin/||g" ${config.treefmt.build.configFile} > treefmt.toml
                      git add ${treefmtConfig}
                    fi
                  '
                '';
                pass_filenames = false;
                always_run = true;
              };
              # Transitive deps mean default clippy ends up using a different cargo.
              my-clippy = {
                enable = true;
                name = "clippy";
                description = "Run clippy";
                entry = "${config.rust-project.toolchain}/bin/cargo-clippy -- --manifest-path Cargo.toml";
                pass_filenames = false;
              };
              cargo-machete = {
                enable = true;
                name = "cargo-machete";
                description = "Check for unused dependencies";
                entry = "${pkgs.cargo-machete}/bin/cargo-machete ./";
                files = "\\.toml$";
                pass_filenames = false;
              };
            };
          };
        };
        devShells = {
          default = pkgs.mkShell devShell;
        };
      };
    });
}

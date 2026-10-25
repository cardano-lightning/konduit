{
  description = "Konduit: A Cardano to Bitcoin Lightning Network pipe";

  inputs = {
    aiken.url = "github:aiken-lang/aiken";
    capkgs.url = "github:input-output-hk/capkgs";
    # Provides the NixOS module for the node the deployment guest runs.
    cardano-node.url = "github:IntersectMBO/cardano-node/11.0.1";
    cardonnay-src = {
      url = "github:IntersectMBO/cardonnay?ref=v0.3.6";
      flake = false;
    };
    flake-parts.url = "github:hercules-ci/flake-parts";
    git-hooks-nix.url = "github:cachix/git-hooks.nix";
    git-hooks-nix.inputs.nixpkgs.follows = "nixpkgs";
    jailed-agents.url = "github:andersonjoseph/jailed-agents";
    kupo.url = "github:paluh/kupo/konduit-kupo";
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

  # The cardano-node binaries in the preprod guest are substitutable from the
  # IOG cache, which turns the longest build of that guest into a download.
  # nix only honours the nixConfig of the top level flake, so the settings of
  # the cardano-node input have to be repeated here.
  nixConfig = {
    extra-substituters = ["https://cache.iog.io"];
    extra-trusted-public-keys = ["hydra.iohk.io:f/Ea+s+dFdN+3Y/G+FDgSq+a5NEWhJGzdjvKNGv0/EQ="];
  };

  outputs = inputs @ {flake-parts, ...}: let
    # Architectures the preprod guest can be built for.
    #
    # The guest is built for the architecture of the machine that evaluates
    # this flake, so that it can use hardware virtualization (KVM on Linux,
    # HVF on macOS). kupo's flake builds x86_64-linux only, so this list has
    # a single entry until kupo gains another architecture.
    #
    # To support another architecture: add it here, add it to `systems`
    # below (so the konduit indexer and server are built for it) and make
    # sure kupo builds for it.
    guestSystems = ["x86_64-linux"];
  in
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

        kupo = inputs'.kupo.packages.kupo;

        packages =
          [
            inputs'.aiken.packages.aiken
            kupo
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

        # A package to setup testnet in the dev shell extras
        cardonnay = pkgs.python313.pkgs.buildPythonApplication {
          pname = "cardonnay";
          version = "0.3.4";
          SETUPTOOLS_SCM_PRETEND_VERSION = "0.3.4";
          src = inputs.cardonnay-src;
          pyproject = true;
          build-system = with pkgs.python313.pkgs; [setuptools setuptools-scm];
          pythonRelaxDeps = ["setuptools"];
          nativeBuildInputs = with pkgs.python313.pkgs; [
            pythonRelaxDepsHook
          ];
          postPatch = ''
            # Reduce initial TX submission delay (safe for local testnets)
            # find src/cardonnay_scripts/scripts \
            #   -name 'common-start-*' -type f -exec \
            #   sed -i 's/readonly TX_SUBMISSION_DELAY=60/readonly TX_SUBMISSION_DELAY=20/' {} +
          '';
          dependencies = with pkgs.python313.pkgs; [
            supervisor
            click
            pygments
            pydantic
            filelock
          ];
        };
        cardano-cli = inputs.capkgs.packages.${system}.cardano-cli-input-output-hk-cardano-node-10-2-1-52b708f;

        cardano-node = inputs.capkgs.packages.${system}.cardano-node-input-output-hk-cardano-node-10-2-1-52b708f;

        process-compose-dev-env-yaml = pkgs.callPackage ./flake/process-compose/dev-env.nix {
          inherit cardonnay cardano-node cardano-cli kupo;
        };

        process-compose-dev-env = pkgs.writeShellApplication {
          name = "process-compose-dev-env";
          runtimeInputs = [];
          text = ''
            ${pkgs.process-compose}/bin/process-compose up -f ${process-compose-dev-env-yaml} -L "$RUN_DIR/process-compose-dev-env";
          '';
        };

        devShellExtraPackages =
          devShell.packages
          ++ [
            cardano-cli
            cardano-node
            cardonnay
            config.treefmt.build.wrapper
            kupo
            process-compose-dev-env
            pkgs.coreutils
            pkgs.gcc
            pkgs.libiconv
            pkgs.process-compose
            wasm-pack
          ];

        # Jail which is shared between a real opencode
        # agent and a bash jail sandbox used for testing the jailed env itself.
        commonJail = {
          baseJailOptions = let
            jail = inputs.jailed-agents.lib.${pkgs.system}.internals.jail;
          in [
            jail.combinators.network
            jail.combinators.time-zone
            jail.combinators.no-new-session
            jail.combinators.mount-cwd
            (jail.combinators.try-fwd-env "PKG_CONFIG_PATH")
            (jail.combinators.try-fwd-env "LD_LIBRARY_PATH")
            (jail.combinators.try-fwd-env "RUST_SRC_PATH")
            (jail.combinators.try-fwd-env "CARDANO_NODE_SOCKET_PATH")
            (jail.combinators.try-fwd-env "CARDANO_NODE_CONFIG_PATH")
            (jail.combinators.try-fwd-env "KONDUIT_VALIDATOR_HASH")
            (jail.combinators.try-fwd-env "CARDANO_NODE_NETWORK_ID")
            (jail.combinators.try-fwd-env "CARDONNAY_TESTNET_ID")
            (jail.combinators.try-fwd-env "FAUCET_ADDR_FILE")
            (jail.combinators.try-fwd-env "FAUCET_SKEY_FILE")
            (jail.combinators.try-fwd-env "FAUCET_WALLET_FILE")
            (jail.combinators.try-fwd-env "KONDUIT_INDEXER_DB_PATH")
            (jail.combinators.try-fwd-env "KONDUIT_INDEXER_DIR")
            (jail.combinators.try-fwd-env "KONDUIT_INDEXER_PORT")
            (jail.combinators.try-fwd-env "KUPO_INDEXER_DIR")
            (jail.combinators.try-fwd-env "KUPO_INDEXER_PORT")
            (jail.combinators.try-fwd-env "KUPO_UTXO_DIR")
            (jail.combinators.try-fwd-env "KUPO_UTXO_PORT")
          ];

          extraReadwriteDirs = [
            "~/projects/cl/konduit"
            "~/projects/cl/konduit-js"
            "~/projects/marlowe/marlowe-plutus/"
            "~/projects/marlowe/marlowe-deploy/"
            "/tmp"
            # "~/projects/cl/konduit/paluh/indexer"
            "~/.config/opencode"
            "~/.local/share/opencode"
            "~/.cache/opencode"
          ];
          extraPkgs =
            devShellExtraPackages
            ++ lib.concatMap (crate: crate.crane.args.nativeBuildInputs) (lib.attrValues config.rust-project.crates)
            ++ lib.concatMap (crate: crate.crane.args.buildInputs) (lib.attrValues config.rust-project.crates);
        };

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
              export RUN_DIR="$ROOT_DIR/.run"

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
              SHELLEY_GENESIS=$(jq -r .ShelleyGenesisFile "$CARDANO_NODE_CONFIG_PATH")
              export CARDANO_NODE_NETWORK_ID=$(jq -r .networkMagic "$CONFIG_DIR/$SHELLEY_GENESIS")

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

            packages =
              devShellExtraPackages
              ++ [
                (inputs.jailed-agents.lib.${pkgs.system}.makeJailedOpencode {
                  inherit (commonJail) baseJailOptions extraPkgs extraReadwriteDirs;
                })

                (inputs.jailed-agents.lib.${pkgs.system}.makeJailedOpencode {
                  name = "jailed-bash";
                  pkg = pkgs.bashInteractive;
                  inherit (commonJail) baseJailOptions extraPkgs extraReadwriteDirs;
                })
              ];
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
          extras = pkgs.mkShell devShellExtras;
        };
      };
      flake = let
        # The machine that evaluates this flake decides the architecture of
        # the guest. NixOS guests are always Linux, so a macOS host builds a
        # Linux guest of the same architecture.
        hostSystem = builtins.currentSystem;
        guestSystem = lib.replaceStrings ["-darwin"] ["-linux"] hostSystem;

        nixos = import ./flake/nixos/outputs.nix {
          self = self;
          nixpkgs = inputs.nixpkgs;
        };

        # The guest shares a preprod snapshot and the server secrets from the
        # host, so both are read from the environment and passed on as plain
        # strings: a Nix path would be copied into the store.
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
            default = import ./flake/nixos.nix self;
          };

        # A VM running cardano-node, Kupo, the konduit indexer and the konduit
        # backend against preprod. See ./flake/nixos/README.md.
        nixosConfigurations.preprod =
          if !(lib.elem guestSystem guestSystems)
          then
            throw ''
              konduit: there is no preprod guest for '${guestSystem}' (this machine is '${hostSystem}').

              Supported guest architectures: ${lib.concatStringsSep ", " guestSystems}.

              The guest is built for the architecture of the machine that evaluates
              this flake, so that it can use hardware virtualization. Supporting
              '${guestSystem}' needs:
                - kupo built for '${guestSystem}' (its flake builds x86_64-linux only)
                - '${guestSystem}' listed in `systems` in flake.nix, so that the
                  konduit indexer and the konduit server are built for it
            ''
          else
            nixos.mkPreprodDeployment {
              system = guestSystem;
              cardanoNodeModule = inputs.cardano-node.nixosModules.cardano-node;
              cardanoCli = inputs.cardano-node.packages.${guestSystem}.cardano-cli;
              kupoPackage = inputs.kupo.packages.${guestSystem}.kupo;
              indexerPackage = self.packages.${guestSystem}.konduit-indexer;
              serverPackage = self.packages.${guestSystem}.konduit-server;
              hostSnapshot = requiredEnv "KONDUIT_PREPROD_DB";
              hostUid = lib.toInt (requiredEnv "KONDUIT_PREPROD_DB_OWNER_UID");
              hostEnvFile = requiredEnv "KONDUIT_SERVER_ENV_FILE";
            };
      };
    });
}

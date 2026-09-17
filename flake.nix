{
  description = "Konduit: A Cardano to Bitcoin Lightning Network pipe";

  inputs = {
    aiken.url = "github:aiken-lang/aiken";
    capkgs.url = "github:input-output-hk/capkgs";
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

  outputs = inputs @ {flake-parts, ...}:
    flake-parts.lib.mkFlake {inherit inputs;}
    {
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
            pkgs.yarn
            pkgs.nodejs
            pkgs.typescript-language-server
            # RUST
            pkgs.openssl
            config.rust-project.toolchain
            wasm-pack
            clang-unwrapped
            pkgs.cargo-machete
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
          inherit packages;

          nativeBuildInputs =
            [config.treefmt.build.wrapper]
            ++ lib.concatMap (crate: crate.crane.args.nativeBuildInputs) (lib.attrValues config.rust-project.crates);

          buildInputs =
            [pkgs.libiconv]
            ++ lib.concatMap (crate: crate.crane.args.buildInputs) (lib.attrValues config.rust-project.crates);

          CC_wasm32_unknown_unknown = lib.getExe' clang-unwrapped "clang";
        };

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
          ];

          extraReadwriteDirs = [
            "~/projects/cl/konduit/paluh/indexer"
            "~/.config/opencode"
            "~/.local/share/opencode"
            "~/.cache/opencode"
          ];
          extraPkgs =
            packages
            ++ lib.concatMap (crate: crate.crane.args.nativeBuildInputs) (lib.attrValues config.rust-project.crates)
            ++ lib.concatMap (crate: crate.crane.args.buildInputs) (lib.attrValues config.rust-project.crates)
            ++ [
              config.treefmt.build.wrapper
              pkgs.libiconv
              pkgs.coreutils
              pkgs.gcc
            ];
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

        devShellExtras =
          devShell
          // {
            name = "konduit-shell-with-extras";

            shellHook = ''
              echo 1>&2 "Welcome to the development shell with extras!"

              ${shellHookCommon}

              export ROOT_DIR="$(git rev-parse --show-toplevel)"
              export RUN_DIR="$ROOT_DIR/.run"

              # Vars required by testnet part of the process compose:

              export TESTNET_DIR="$RUN_DIR/testnet"
              export CARDONNAY_TESTNET_ID="9"
              export CARDANO_NODE_NETWORK_ID=42

              export KUPO_INDEXER_DIR="$RUN_DIR/kupo-indexer"
              export KUPO_INDEXER_PORT=1442

              export KUPO_UTXO_DIR="$RUN_DIR/kupo-utxo"
              export KUPO_UTXO_PORT=1443

              export KONDUIT_INDEXER_DIR="$RUN_DIR/konduit-indexer"
              export KONDUIT_INDEXER_DB_PATH="$KONDUIT_INDEXER_DIR/konduit-indexer.db"


              # This exposes CARDANO_NODE_SOCKET_PATH to the shell
              source <(cardonnay control print-env -i "$CARDONNAY_TESTNET_ID" -w "$TESTNET_DIR")

              # Removes the socket file name from the path, leaving only the directory
              NODE_SOCKET_DIR="''${CARDANO_NODE_SOCKET_PATH%/*}"
              NODE_SOCKET_NAME="''${CARDANO_NODE_SOCKET_PATH##*/}"
              export CARDANO_NODE_CONFIG_PATH="$NODE_SOCKET_DIR/config-''${NODE_SOCKET_NAME%.socket}.json"

              export KONDUIT_VALIDATOR_HASH="$(cat $ROOT_DIR/packages/kernel/plutus.json | jq -r '.validators[0].hash')"

              # This **will be** initialized by the testnet process compose when executed
              export FAUCET_ADDR_FILE="$TESTNET_DIR/faucet.addr"
              export FAUCET_SKEY_FILE="$TESTNET_DIR/faucet.skey"

              export PROCESS_COMPOSE_YAML=${process-compose-dev-env-yaml}
            '';

            packages =
              devShell.packages
              ++ [
                cardano-node
                cardano-cli
                cardonnay
                process-compose-dev-env

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
      flake = {
        nixosModules.default = import ./flake/nixos.nix inputs.self;
      };
    };
}

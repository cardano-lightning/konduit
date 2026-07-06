{
  description = "Konduit: A Cardano to Bitcoin Lightning Network pipe";

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    git-hooks-nix.url = "github:cachix/git-hooks.nix";
    git-hooks-nix.inputs.nixpkgs.follows = "nixpkgs";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    treefmt-nix.inputs.nixpkgs.follows = "nixpkgs";
    aiken.url = "github:aiken-lang/aiken";
    rust-flake.url = "github:juspay/rust-flake/";
    capkgs.url = "github:input-output-hk/capkgs";
    jailed-agents.url = "github:andersonjoseph/jailed-agents";
    cardano-kupo.url = "github:paluh/cardano-kupo";
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
        clang-unwrapped = pkgs.llvmPackages_latest.clang-unwrapped;
        wasm-pack = pkgs.callPackage ./flake/wasm-pack.nix {};
        packages =
          [
            # aiken
            inputs'.aiken.packages.aiken
            # kupo
            inputs'.cardano-kupo.packages.kupo
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
              ${config.pre-commit.installationScript}
            echo 1>&2 "Welcome to the development shell!"
              export RUST_SRC_PATH="${config.rust-project.toolchain}/lib/rustlib/src/rust/library";
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
            "~/projects/cardano-lightning/konduit"
            "~/.config/opencode"
            "~/.local/share/opencode"
            "~/.cache/opencode"
          ];
          extraPkgs = packages;
        };
        devShellExtra =
          devShell
          // {
            name = "konduit-shell-with-extras";
            packages =
              devShell.packages
              ++ [
                inputs.capkgs.packages.${system}.cardano-cli-input-output-hk-cardano-node-10-2-1-52b708f

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
          extras = pkgs.mkShell devShellExtra;
        };
      };
      flake = {
        nixosModules.default = import ./flake/nixos.nix inputs.self;
      };
    };
}

{
  description = "Konduit: Cardano Lightning Network payments";

  inputs = {
    aiken = {
      url = "github:aiken-lang/aiken";
    };
    flake-parts.url = "github:hercules-ci/flake-parts";
    git-hooks-nix = {
      url = "github:cachix/git-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-flake = {
      url = "github:juspay/rust-flake";
      inputs.rust-overlay.follows = "rust-overlay";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
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
            # aiken = {
            #   enable = true;
            #   package = inputs'.aiken.packages.aiken;
            # };
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
              # # Transitive deps mean default clippy ends up using a different cargo.
              # FIXME :: Is this now sorted??
              # my-clippy = {
              #   enable = true;
              #   name = "clippy";
              #   description = "Run clippy";
              #   entry = "${config.rust-project.toolchain}/bin/cargo-clippy -- --manifest-path Cargo.toml";
              #   pass_filenames = false;
              # };
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

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
        self',
        inputs',
        pkgs,
        system,
        ...
      }: let
        clang-unwrapped = pkgs.llvmPackages_latest.clang-unwrapped;
        devShell = {
          name = "konduit-shell";
          shellHook = ''
            ${config.pre-commit.installationScript}
            echo 1>&2 "Welcome to the development shell!"
            export RUST_SRC_PATH="${config.rust-project.toolchain}/lib/rustlib/src/rust/library";
          '';
          packages =
            [
              inputs'.aiken.packages.aiken
              pkgs.yarn
              pkgs.nodePackages_latest.nodejs
              pkgs.typescript-language-server
              pkgs.openssl
              config.rust-project.toolchain
              pkgs.wasm-pack
              clang-unwrapped
              pkgs.just
            ]
            ++ lib.mapAttrsToList (_: crate: crate.crane.args.nativeBuildInputs) config.rust-project.crates;
          buildInputs =
            [
              pkgs.libiconv
            ]
            ++ lib.mapAttrsToList (_: crate: crate.crane.args.buildInputs) config.rust-project.crates;
          nativeBuildInputs = [
            config.treefmt.build.wrapper
          ];
          CC_wasm32_unknown_unknown = lib.getExe' clang-unwrapped "clang";
        };
        devShellExtra =
          devShell
          // {
            name = "konduit-shell-with-extras";
            packages =
              devShell.packages
              ++ [
                inputs.capkgs.packages.${system}.cardano-cli-input-output-hk-cardano-node-10-2-1-52b708f
              ];
          };
      in {
        rust-project = {
          src = ./rust;
          cargoToml = builtins.fromTOML (builtins.readFile ./rust/Cargo.toml);
          toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust/rust-toolchain.toml;
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
          programs = {
            prettier = {
              enable = true;
              settings = {
                printWidth = 80;
                proseWrap = "always";
              };
            };
            alejandra.enable = true;
            rustfmt.enable = true;
            aiken.enable = true;
          };
        };
        pre-commit.settings.hooks = {
          treefmt.enable = true;
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

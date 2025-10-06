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
        config,
        self',
        inputs',
        pkgs,
        system,
        ...
      }: {
        rust-project = {
          src = ./rust;
          cargoToml = builtins.fromTOML (builtins.readFile ./rust/Cargo.toml);
          toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust/rust-toolchain.toml;
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
        # NOTE: You can also use `config.pre-commit.devShell`
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = [
            config.treefmt.build.wrapper
            config.devShells.rust.nativeBuildInputs
          ];
          shellHook = ''
            ${config.pre-commit.installationScript}
            echo 1>&2 "Welcome to the development shell!"
          '';
          name = "konduit-shell";
          # Let's keep this "path discovery techinque" here for refernece:
          # (builtins.trace (builtins.attrNames inputs.cardano-addresses.packages.${system}) inputs.cardano-cli.packages)
          packages = [
            inputs'.aiken.packages.aiken
            pkgs.yarn
            pkgs.nodePackages_latest.nodejs
          ];
        };
      };
      flake = {};
    };
}

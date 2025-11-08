self: {
  lib,
  config,
  pkgs,
  ...
}: let
  inherit (lib) mkOption types mapAttrs';

  konduitAdaptorOptions = {name, ...}: {
    options = {
      domain = mkOption {
        type = types.str;
        default = name;
        description = "The domain to host the konduit adaptor";
      };

      env-file = mkOption {
        type = types.path;
        description = "An env file containing the konduit adaptor configuration";
      };

      useSSL = mkOption {
        type = types.bool;
        default = true;
        description = "Whether to use SSL for the adaptor";
      };

      flake = mkOption {
        type = types.attrs;
        default = self;
        description = "A Nix Flake for the konduit adaptor application";
      };
    };
  };

  cfg = builtins.trace (builtins.attrNames config) config.konduit-adaptors;
in {
  options = {
    konduit-adaptors = mkOption {
      type = types.attrsOf (types.submodule konduitAdaptorOptions);
      default = {};
      description = "Konduit adaptors to run";
    };
  };
  config = {
    http-services.proxied-services =
      mapAttrs'
      (name: konduit-adaptor: let
        flakePkgs = konduit-adaptor.flake.packages.x86_64-linux;
      in {
        name = "konduit-adatpor-${name}";
        value = {
          inherit (konduit-adaptor) domain;
          systemdConfig = port: {
            description = "Konduit adaptor (${name})";
            serviceConfig = {
              ExecSearchPath = "${flakePkgs.konduit-adaptor}/bin";
              DynamicUser = true;
              PrivateTmp = true;
              EnvironmentFile = "${konduit-adaptor.env-file}";
              ExecStart = "konduit-adaptor --path \${STATE_DIRECTORY}/db --port \"${toString port}\" --host \"127.0.0.1\"";
              StateDirectory = "konduit-adaptor-${name}";
              RuntimeDirectory = "konduit-adaptor-${name}";
            };
          };
        };
      })
      cfg;
  };
}

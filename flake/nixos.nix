self: {
  lib,
  config,
  pkgs,
  ...
}: let
  inherit (lib) mkOption types mapAttrs';

  konduitServerOptions = {name, ...}: {
    options = {
      domain = mkOption {
        type = types.str;
        default = name;
        description = "The domain to host the konduit server";
      };

      env-file = mkOption {
        type = types.path;
        description = "An env file containing the konduit server configuration";
      };

      useSSL = mkOption {
        type = types.bool;
        default = true;
        description = "Whether to use SSL for the server";
      };

      flake = mkOption {
        type = types.attrs;
        default = self;
        description = "A Nix Flake for the konduit server application";
      };
    };
  };

  cfg = builtins.trace (builtins.attrNames config) config.konduit-servers;
in {
  options = {
    konduit-servers = mkOption {
      type = types.attrsOf (types.submodule konduitServerOptions);
      default = {};
      description = "Konduit servers to run";
    };
  };
  config = {
    http-services.proxied-services =
      mapAttrs'
      (name: konduit-server: let
        flakePkgs = konduit-server.flake.packages.x86_64-linux;
      in {
        name = "konduit-adatpor-${name}";
        value = {
          inherit (konduit-server) domain;
          systemdConfig = port: {
            description = "Konduit server (${name})";
            serviceConfig = {
              ExecSearchPath = "${flakePkgs.konduit-server}/bin";
              DynamicUser = true;
              PrivateTmp = true;
              EnvironmentFile = "${konduit-server.env-file}";
              ExecStart = "konduit-server --path \${STATE_DIRECTORY}/db --port \"${toString port}\" --host \"127.0.0.1\"";
              StateDirectory = "konduit-server-${name}";
              RuntimeDirectory = "konduit-server-${name}";
            };
          };
        };
      })
      cfg;
  };
}

# Kupo as systemd services.
#
# The dev env (../process-compose/dev-env.nix) runs two Kupo processes which
# differ only in their flags:
#
#   kupo-indexer  pattern scoped to the konduit validator, polled by konduit-indexer
#   kupo-utxo     everything, pruned, for humans and ad-hoc queries
#
# So they are modelled as instances of one module, keyed by the name of the
# process they mirror. Every instance gets a unit and a readiness gate:
#
#   kupo-indexer.service + kupo-indexer-ready.service
#   kupo-utxo.service    + kupo-utxo-ready.service
#
# Kupo is `Type=simple` and does not notify, hence the gate: downstream services
# declare `requires = [ "<instance>-ready.service" ]` instead of racing Kupo.
{
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.services.kupo;
  inherit (lib) mkEnableOption mkIf mkOption types;

  # Importing the module without enabling an instance creates nothing.
  anyEnabled = any (instance: instance.enable) (lib.attrValues cfg.instances);

  instanceOptions = {name, ...}: {
    options = {
      enable = mkEnableOption "the Kupo instance ${name}";

      extraArgs = mkOption {
        type = types.listOf types.str;
        default = [];
        description = "Passed through to kupo. Prefer the typed options below.";
      };

      host = mkOption {
        type = types.str;
        default = "127.0.0.1";
        description = "Address the Kupo API binds to.";
      };

      logLevel = mkOption {
        type = types.enum ["Debug" "Info" "Notice" "Warning" "Error" "Off"];
        default = "Info";
        description = "Minimal severity of all Kupo log messages.";
      };

      matchPatterns = mkOption {
        type = types.listOf types.str;
        default = ["*/*"];
        description = "Patterns to match on, as a logical disjunction. Repeated --match.";
      };

      nodeConfigPath = mkOption {
        type = types.str;
        description = ''
          Path to the node configuration file. Kupo reads it for the Byron and
          Shelley genesis files it points at; both are resolved next to the
          configuration file itself.
        '';
      };

      nodeSocketPath = mkOption {
        type = types.str;
        description = "Path to the cardano-node socket. Kupo retries while the node is down.";
      };

      port = mkOption {
        type = types.port;
        default = 1442;
        description = "Port the Kupo API listens on.";
      };

      pruneUtxo = mkOption {
        type = types.bool;
        default = false;
        description = "Drop inputs from the index once spent instead of marking them spent.";
      };

      since = mkOption {
        type = types.str;
        default = "origin";
        description = ''
          Point to sync from on the first start. Use a slot/hash pair to index a
          window of the chain, "tip" to only follow the node from now on and
          "origin" to walk the whole chain.
        '';
      };
    };
  };
in {
  options.services.kupo = {
    instances = mkOption {
      type = types.attrsOf (types.submodule instanceOptions);
      default = {};
      description = "Kupo instances, keyed by the name of the process they mirror.";
    };

    package = mkOption {
      type = types.package;
      description = "Package providing bin/kupo.";
    };

    readyTimeoutSeconds = mkOption {
      type = types.ints.unsigned;
      default = 600;
      description = "How long the readiness gate waits for a connected Kupo.";
    };

    socketGroup = mkOption {
      type = types.str;
      default = "cardano-node";
      description = "Group that may connect to the node socket. Kupo is added to it.";
    };

    user = mkOption {
      type = types.str;
      default = "kupo";
      description = "User running all instances. Kupo databases are not shared.";
    };
  };

  config = {
    users.users.${cfg.user} = lib.mkIf anyEnabled {
      isSystemUser = true;
      group = cfg.user;
      extraGroups = [cfg.socketGroup];
    };
    users.groups.${cfg.user} = lib.mkIf anyEnabled {};

    systemd.services =
      lib.mapAttrs'
      (name: instance: let
        # systemd creates and owns /var/lib/<name>, so an instance never has
        # to mkdir and its database survives a restart.
        workDir = "/var/lib/${name}";
      in
        mkIf instance.enable {
          "${name}" = {
            description = "Kupo (${name})";
            wantedBy = ["multi-user.target"];
            after = ["network-online.target"];
            wants = ["network-online.target"];
            unitConfig.StartLimitIntervalSec = 0;
            serviceConfig = {
              User = cfg.user;
              Group = cfg.user;
              SupplementaryGroups = [cfg.socketGroup];
              StateDirectory = name;
              Restart = "always";
              RestartSteps = 10;
              RestartMaxDelaySec = "1h";
              ExecStart = lib.escapeShellArgs (
                [
                  "${cfg.package}/bin/kupo"
                  "--node-socket"
                  instance.nodeSocketPath
                  "--node-config"
                  instance.nodeConfigPath
                  "--workdir"
                  workDir
                  "--host"
                  instance.host
                  "--port"
                  (toString instance.port)
                  "--since"
                  instance.since
                  "--log-level"
                  instance.logLevel
                ]
                ++ lib.concatMap (pattern: ["--match" pattern]) instance.matchPatterns
                ++ lib.optional instance.pruneUtxo "--prune-utxo"
                ++ instance.extraArgs
              );
            };
          };

          # The readiness gate. `kupo health-check` exits 0 once the instance
          # reports a node connection, which is what konduit-indexer needs.
          #
          # PartOf (not BindsTo) so a restart of Kupo invalidates the gate
          # without the service requiring the gate: that would be a cycle.
          "${name}-ready" = {
            description = "Wait until Kupo (${name}) is connected to cardano-node";
            after = ["${name}.service"];
            partOf = ["${name}.service"];
            path = [
              cfg.package
              pkgs.coreutils
            ];
            serviceConfig = {
              Type = "oneshot";
              RemainAfterExit = true;
              ExecStart = pkgs.writeShellScript "kupo-${name}-ready" ''
                set -euo pipefail
                for _ in $(seq 1 ${toString cfg.readyTimeoutSeconds}); do
                  if ${cfg.package}/bin/kupo health-check \
                    --host ${instance.host} \
                    --port ${toString instance.port}; then
                    exit 0
                  fi
                  sleep 1
                done
                echo "kupo health-check for ${name} did not succeed" >&2
                exit 1
              '';
            };
          };
        })
      cfg.instances;
  };
}

# The konduit-indexer binary runs a single sync pass against Kupo and exits,
# so the dev env drives it with a process-compose schedule
# (../process-compose/dev-env.nix). Here that schedule is a systemd timer
# feeding a oneshot:
#
#   konduit-indexer.timer -> konduit-indexer.service
#
# The timer fires only when the unit is inactive, so passes never overlap
# (the dev env guards the same way with max_concurrent = 1).
{
  config,
  lib,
  ...
}: let
  cfg = config.services.konduit-indexer;
  inherit (lib) escapeShellArgs mkEnableOption mkIf mkOption types;

  # systemd creates and owns /var/lib/konduit-indexer, which is where the
  # SQLite file lives.
  dbPath = "/var/lib/konduit-indexer/konduit-indexer.db";
in {
  options.services.konduit-indexer = {
    enable = mkEnableOption "the konduit indexer";

    extraArgs = mkOption {
      type = types.listOf types.str;
      default = [];
      description = "Passed through to konduit-indexer.";
    };

    interval = mkOption {
      type = types.str;
      default = "10s";
      description = "Delay between two sync passes. Mirrors the process-compose schedule.";
    };

    kupoHost = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Host of the Kupo instance the indexer reads from.";
    };

    kupoInstance = mkOption {
      type = types.str;
      default = "kupo-indexer";
      description = ''
        Name of the services.kupo instance to wait for. Its readiness gate
        (`<kupoInstance>-ready.service`) gates every sync pass.
      '';
    };

    kupoPort = mkOption {
      type = types.port;
      default = 1442;
      description = "Port of the Kupo instance the indexer reads from.";
    };

    maxRetries = mkOption {
      type = types.ints.unsigned;
      default = 5;
      description = "Rollback reconciliations before a pass errors out.";
    };

    package = mkOption {
      type = types.package;
      description = "Package providing bin/konduit-indexer.";
    };
  };

  config = mkIf cfg.enable {
    users.users.konduit-indexer = {
      isSystemUser = true;
      group = "konduit-indexer";
    };
    users.groups.konduit-indexer = {};

    systemd.services.konduit-indexer = {
      description = "Konduit indexer sync pass";
      after = ["${cfg.kupoInstance}-ready.service"];
      requires = ["${cfg.kupoInstance}-ready.service"];
      serviceConfig = {
        Type = "oneshot";
        User = "konduit-indexer";
        Group = "konduit-indexer";
        StateDirectory = "konduit-indexer";
        ExecStart = escapeShellArgs (
          [
            "${cfg.package}/bin/konduit-indexer"
            "index"
            "--kupo-host"
            cfg.kupoHost
            "--kupo-port"
            (toString cfg.kupoPort)
            "--db-path"
            dbPath
            "--max-retries"
            (toString cfg.maxRetries)
          ]
          ++ cfg.extraArgs
        );
      };
    };

    systemd.timers.konduit-indexer = {
      description = "Run the konduit indexer sync pass every ${cfg.interval}";
      wantedBy = ["timers.target"];
      timerConfig = {
        Unit = "konduit-indexer.service";
        OnBootSec = cfg.interval;
        OnUnitActiveSec = cfg.interval;
        AccuracySec = "1s";
      };
    };
  };
}

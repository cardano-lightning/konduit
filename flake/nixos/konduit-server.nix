# The konduit backend (packages/konduit/server).
#
# The server reads everything from its environment: the cardano-sdk arguments
# expose it through clap, and the BLN and FX clients through their own
# namespaced flags. Options here only wire the values in; anything the server
# understands can still be overridden through `environment` (checked in last,
# so it wins) or through the `environmentFile` the secrets are kept in.
#
# Not wired here on purpose:
#
#   KONDUIT_SIGNING_KEY  adaptor signing key, also the administrating wallet
#   KONDUIT_HOST_ADDRESS address hosting the konduit reference script
#
# Both are deployment secrets, so they stay in the env file.
{
  config,
  lib,
  ...
}: let
  cfg = config.services.konduit-server;
  inherit (lib) escapeShellArgs mkEnableOption mkIf mkOption optionalAttrs types;

  # systemd creates and owns /var/lib/konduit-server, which is where the sled
  # database lives.
  dbPath = "/var/lib/konduit-server/konduit-server.db";

  cardanoEnv =
    {
      KONDUIT_CARDANO_BACKEND = cfg.cardano.backend;
    }
    // optionalAttrs (cfg.cardano.nodeSocketPath != null) {
      KONDUIT_NODE_SOCKET = cfg.cardano.nodeSocketPath;
    }
    // optionalAttrs (cfg.cardano.nodeNetworkMagic != null) {
      KONDUIT_NODE_NETWORK_MAGIC = toString cfg.cardano.nodeNetworkMagic;
    }
    // optionalAttrs (cfg.cardano.blockfrostProjectId != null) {
      KONDUIT_BLOCKFROST_PROJECT_ID = cfg.cardano.blockfrostProjectId;
    }
    // optionalAttrs (cfg.cardano.utxorpcUri != null) {
      KONDUIT_UTXORPC_URI = cfg.cardano.utxorpcUri;
    }
    // optionalAttrs (cfg.cardano.network != null) {
      KONDUIT_NETWORK = cfg.cardano.network;
    };

  # The BLN client is mandatory and only knows LND and a mock backend.
  blnEnv =
    optionalAttrs cfg.bln.mock {
      BLN_MOCK = "true";
    }
    // optionalAttrs (cfg.bln.baseUrl != null) {
      LND_BASE_URL = cfg.bln.baseUrl;
    }
    // optionalAttrs (cfg.bln.macaroon != null) {
      LND_MACAROON = cfg.bln.macaroon;
    }
    // optionalAttrs (cfg.bln.blockTime != null) {
      BLN_BLOCK_TIME = cfg.bln.blockTime;
    };

  # The FX client refuses to start without a source, and refuses to tick on a
  # zero interval, so `every` is always set.
  fxEnv =
    {
      FX_EVERY = cfg.fx.every;
    }
    // optionalAttrs (cfg.fx.baseCurrency != null) {
      FX_BASE_CURRENCY = cfg.fx.baseCurrency;
    }
    // optionalAttrs (cfg.fx.ada != null) {
      FX_ADA = toString cfg.fx.ada;
    }
    // optionalAttrs (cfg.fx.bitcoin != null) {
      FX_BITCOIN = toString cfg.fx.bitcoin;
    }
    // optionalAttrs (cfg.fx.coinGeckoToken != null) {
      FX_COIN_GECKO_TOKEN = cfg.fx.coinGeckoToken;
    }
    // optionalAttrs cfg.fx.coinGeckoPublic {
      FX_COIN_GECKO = "true";
    }
    // optionalAttrs cfg.fx.binance {
      FX_BINANCE = "true";
    }
    // optionalAttrs cfg.fx.kraken {
      FX_KRAKEN = "true";
    };

  fxConfigured =
    (cfg.fx.ada != null && cfg.fx.bitcoin != null)
    || cfg.fx.coinGeckoPublic
    || (cfg.fx.coinGeckoToken != null)
    || cfg.fx.binance
    || cfg.fx.kraken;
in {
  options.services.konduit-server = {
    bln = {
      baseUrl = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Base URL of the LND REST API.";
      };
      blockTime = mkOption {
        type = types.nullOr types.str;
        default = "10m";
        description = "BLN block time, as a humantime duration.";
      };
      macaroon = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "LND macaroon in hex format. Keep it in the env file.";
      };
      mock = mkOption {
        type = types.bool;
        default = false;
        description = "Use the mock BLN backend instead of LND.";
      };
    };

    cardano = {
      backend = mkOption {
        type = types.enum ["node" "blockfrost" "utxorpc"];
        default = "node";
        description = "Cardano data source. A backend only reads its own options.";
      };
      blockfrostProjectId = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Blockfrost project id. The network is inferred from its prefix.";
      };
      network = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Public network name (mainnet, preprod, preview). Only read by the utxorpc backend.";
      };
      nodeNetworkMagic = mkOption {
        type = types.nullOr types.ints.unsigned;
        default = null;
        description = "Network magic of the node the server talks to.";
      };
      nodeSocketPath = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Path to the cardano-node socket.";
      };
      utxorpcUri = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Endpoint of a UTxO RPC (Dolos) node.";
      };
    };

    enable = mkEnableOption "the konduit backend server";

    environment = mkOption {
      type = types.attrsOf types.str;
      default = {};
      description = ''
        Extra environment variables. Values defined here are applied last, so
        they win over the ones this module derives from the options.
      '';
    };

    environmentFile = mkOption {
      type = types.nullOr types.path;
      default = null;
      example = "/mnt/konduit/server.env";
      description = ''
        File with environment variables, read just before the server starts.
        systemd applies it after Environment=, so it wins over everything this
        module sets. This is where the signing key and the host address live.
      '';
    };

    extraArgs = mkOption {
      type = types.listOf types.str;
      default = [];
      description = "Passed through to konduit-server.";
    };

    fx = {
      ada = mkOption {
        type = types.nullOr types.float;
        default = null;
        description = "Fixed price of one ADA in the base currency.";
      };
      baseCurrency = mkOption {
        type = types.nullOr types.str;
        default = "eur";
        description = "Currency the FX client quotes in.";
      };
      binance = mkOption {
        type = types.bool;
        default = false;
        description = "Quote through Binance.";
      };
      bitcoin = mkOption {
        type = types.nullOr types.float;
        default = null;
        description = "Fixed price of one BTC in the base currency.";
      };
      coinGeckoPublic = mkOption {
        type = types.bool;
        default = false;
        description = "Quote through the public CoinGecko API.";
      };
      coinGeckoToken = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "CoinGecko API token. Keep it in the env file.";
      };
      every = mkOption {
        type = types.str;
        default = "10m";
        description = ''
          How often the FX state is refreshed, as a humantime duration. The
          server ticks on this interval, so it must not be zero.
        '';
      };
      kraken = mkOption {
        type = types.bool;
        default = false;
        description = "Quote through Kraken.";
      };
    };

    host = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Address the HTTP server binds to.";
    };

    logLevel = mkOption {
      type = types.str;
      default = "info";
      description = "env_logger filter, passed as RUST_LOG.";
    };

    openFirewall = mkOption {
      type = types.bool;
      default = false;
      description = "Open the port in the firewall.";
    };

    package = mkOption {
      type = types.package;
      description = "Package providing bin/konduit-server.";
    };

    port = mkOption {
      type = types.port;
      default = 5663;
      description = "Port the HTTP server listens on.";
    };

    socketGroup = mkOption {
      type = types.str;
      default = "cardano-node";
      description = "Group that may connect to the node socket. The server user is added to it.";
    };

    user = mkOption {
      type = types.str;
      default = "konduit-server";
      description = "User running the server.";
    };
  };

  config = mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.cardano.backend != "node" || (cfg.cardano.nodeSocketPath != null && cfg.cardano.nodeNetworkMagic != null);
        message = "services.konduit-server.cardano.{nodeSocketPath,nodeNetworkMagic} are required by the node backend.";
      }
      {
        assertion = cfg.cardano.backend != "blockfrost" || cfg.cardano.blockfrostProjectId != null;
        message = "services.konduit-server.cardano.blockfrostProjectId is required by the blockfrost backend.";
      }
      {
        assertion = cfg.cardano.backend != "utxorpc" || (cfg.cardano.utxorpcUri != null && cfg.cardano.network != null);
        message = "services.konduit-server.cardano.{utxorpcUri,network} are required by the utxorpc backend.";
      }
      {
        assertion = cfg.bln.mock || (cfg.bln.baseUrl != null && cfg.bln.macaroon != null);
        message = "services.konduit-server.bln.{baseUrl,macaroon} are required unless bln.mock is set.";
      }
      {
        assertion = fxConfigured;
        message = ''
          services.konduit-server.fx needs a source: either ada and bitcoin for
          fixed rates, or coinGeckoPublic, coinGeckoToken, binance or kraken.
        '';
      }
    ];

    users.users.${cfg.user} = {
      isSystemUser = true;
      group = cfg.user;
      extraGroups = [cfg.socketGroup];
    };
    users.groups.${cfg.user} = {};

    networking.firewall.allowedTCPPorts = mkIf cfg.openFirewall [cfg.port];

    systemd.services.konduit-server = {
      description = "Konduit backend server";
      wantedBy = ["multi-user.target"];
      after = ["network-online.target"];
      wants = ["network-online.target"];
      unitConfig.StartLimitIntervalSec = 0;
      serviceConfig = {
        User = cfg.user;
        Group = cfg.user;
        SupplementaryGroups = [cfg.socketGroup];
        StateDirectory = "konduit-server";
        Restart = "always";
        RestartSteps = 10;
        RestartMaxDelaySec = "1h";
        Environment =
          {
            RUST_LOG = cfg.logLevel;
            KONDUIT_DB_PATH = dbPath;
            KONDUIT_SERVER_HOST = cfg.host;
            KONDUIT_SERVER_PORT = toString cfg.port;
          }
          // cardanoEnv
          // blnEnv
          // fxEnv
          // cfg.environment;
        EnvironmentFile = mkIf (cfg.environmentFile != null) cfg.environmentFile;
        ExecStart = escapeShellArgs (["${cfg.package}/bin/konduit-server"] ++ cfg.extraArgs);
      };
    };
  };
}

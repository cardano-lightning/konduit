# The deployment guest. Not a service module: it returns a nixosSystem.
# Service modules stay in kupo.nix, konduit-indexer.nix and konduit-server.nix.
#
#   nixos.mkPreprodDeployment {
#     system = "x86_64-linux";
#     cardanoNodeModule = inputs.cardano-node.nixosModules.cardano-node;
#     cardanoCli = inputs.cardano-node.packages.x86_64-linux.cardano-cli;
#     kupoPackage = inputs.kupo.packages.x86_64-linux.kupo;
#     indexerPackage = inputs.self.packages.x86_64-linux.konduit-indexer;
#     serverPackage = inputs.self.packages.x86_64-linux.konduit-server;
#     hostSnapshot = "/abs/path/to/preprod/db";
#     hostUid = 1000;
#     hostEnvFile = "/abs/path/to/server.env";
#   };
{
  self,
  nixpkgs,
}: {
  cardanoNodeModule,
  cardanoCli,
  kupoPackage,
  indexerPackage,
  serverPackage,
  hostSnapshot,
  hostUid,
  hostEnvFile,
  # The architecture of the guest. The caller derives it from the machine
  # that evaluates the flake, so that the VM can use hardware virtualization.
  system,
}: let
  inherit (nixpkgs) lib;
  # Preprod network magic.
  networkMagic = 1;
  socketPath = "/run/cardano-node/node.socket";
  socketGroup = "cardano-node";
  databasePath = "/mnt/preprod-db";
  secretsPath = "/mnt/konduit";

  # The VM runner runs on the host, not in the guest. nixpkgs defaults
  # `virtualisation.host.pkgs` to the guest's package set, which would build
  # a runner that cannot run on this machine.
  hostPkgs = nixpkgs.legacyPackages.${builtins.currentSystem};

  # The konduit validator as published on chain. kupo-indexer matches on it, so
  # this is the same hash the dev env exports as KONDUIT_VALIDATOR_HASH.
  validatorHash = (builtins.fromJSON (builtins.readFile "${self}/packages/kernel/plutus.json")).validators [0].hash;

  # Preprod has two years of chain behind it. Indexing all of it into Kupo is
  # not what this guest is for: it starts from the tip of the snapshot and
  # follows along. Use "origin", or a "<slot>.<hash>" point, to index a window.
  since = "tip";
in
  nixpkgs.lib.nixosSystem {
    inherit system;
    modules = [
      cardanoNodeModule
      (import ./kupo.nix)
      (import ./konduit-indexer.nix)
      (import ./konduit-server.nix)
      (
        {
          config,
          lib,
          pkgs,
          ...
        }: let
          # Kupo reads the node configuration to learn the network parameters: it
          # takes the Byron and Shelley genesis files from it and resolves them
          # next to the configuration file itself. The cardano-node module keeps
          # its configuration in the store under a generated name, so Kupo gets a
          # configuration with just those two keys, pointing at the same genesis
          # files as the node.
          nodeEnvironment = config.services.cardano-node.environments.${config.services.cardano-node.environment};
          kupoNodeConfig = pkgs.writeText "konduit-node-config.json" (builtins.toJSON {
            ByronGenesisFile = nodeEnvironment.nodeConfig.ByronGenesisFile;
            ShelleyGenesisFile = nodeEnvironment.nodeConfig.ShelleyGenesisFile;
          });
        in {
          assertions = [
            {
              assertion = builtins.hasAttr "ByronGenesisFile" nodeEnvironment.nodeConfig && builtins.hasAttr "ShelleyGenesisFile" nodeEnvironment.nodeConfig;
              message = "The cardano-node environment ${config.services.cardano-node.environment} does not reference the genesis files Kupo needs.";
            }
          ];

          environment.systemPackages = [cardanoCli];
          networking.enableIPv6 = false;
          # The VM runner is run-<hostName>-vm.
          networking.hostName = "preprod";
          services.getty.autologinUser = "root";
          system.stateVersion = "25.11";
          users.mutableUsers = false;
          users.users.root.password = "root";
          users.users.cardano-node = {
            # The snapshot is shared from the host over 9p, which passes the host
            # uid through, so the node has to own the very same uid in here.
            uid = lib.mkForce hostUid;
            extraGroups = ["users"];
          };

          virtualisation.vmVariant.virtualisation = {
            # The runner and QEMU belong to the host, not to the guest.
            host.pkgs = hostPkgs;
            # qemu_kvm is Linux only and only carries the host's own system
            # emulator; the plain qemu package carries all of them. nixpkgs
            # runs it with HVF acceleration on macOS.
            qemu.package =
              if
                hostPkgs.stdenv.hostPlatform.isLinux
                && hostPkgs.stdenv.hostPlatform.qemuArch == pkgs.stdenv.hostPlatform.qemuArch
              then hostPkgs.qemu_kvm
              else hostPkgs.qemu;
            cores = 4;
            diskSize = 64 * 1024; # 64 GB
            memorySize = 16 * 1024; # 16 GB
            forwardPorts = [
              {
                from = "host";
                host.port = 11442;
                guest.port = 1442;
              } # kupo-indexer
              {
                from = "host";
                host.port = 11443;
                guest.port = 1443;
              } # kupo-utxo
              {
                from = "host";
                host.port = 18090;
                guest.port = 5663;
              } # konduit-server
            ];
            sharedDirectories = {
              # hostSnapshot and hostEnvFile are strings on purpose: a Nix path
              # would be copied into the store, and the preprod database is far
              # too big for that.
              preprod-db = {
                source = hostSnapshot;
                target = databasePath;
              };
              konduit-secrets = {
                source = lib.dirOf hostEnvFile;
                target = secretsPath;
                mountOptions = ["ro"];
              };
            };
          };

          services.cardano-node = {
            enable = true;
            environment = "preprod";
            databasePath = databasePath;
            hostAddr = "0.0.0.0";
            socketPath = _: socketPath;
          };

          systemd.services.cardano-node.serviceConfig = {
            UMask = "0007";
            KillSignal = "SIGINT";
            # Default ~90s then SIGKILL, which deletes the marker.
            TimeoutStopSec = "infinity";
          };

          # cardano-node is Type=simple and does not sd_notify, so this oneshot is
          # the gate: everything downstream requires it instead of racing the node.
          systemd.services.cardano-node-ready = {
            description = "Wait until cardano-cli query tip succeeds";
            after = ["cardano-node.service"];
            bindsTo = ["cardano-node.service"];
            path = [
              cardanoCli
              pkgs.coreutils
            ];
            environment = {
              CARDANO_NODE_SOCKET_PATH = socketPath;
              CARDANO_NODE_NETWORK_ID = toString networkMagic;
            };
            serviceConfig = {
              Type = "oneshot";
              RemainAfterExit = true;
              ExecStart = pkgs.writeShellScript "cardano-node-ready" ''
                set -euo pipefail
                for _ in $(seq 1 600); do
                  if cardano-cli query tip \
                    --testnet-magic ${toString networkMagic} \
                    --socket-path ${socketPath}; then
                    exit 0
                  fi
                  sleep 1
                done
                echo "cardano-cli query tip did not succeed" >&2
                exit 1
              '';
            };
          };

          services.kupo = {
            package = kupoPackage;
            # The group the cardano-node module gives its socket.
            inherit socketGroup;
            instances = {
              kupo-utxo = {
                enable = true;
                nodeConfigPath = "${kupoNodeConfig}";
                nodeSocketPath = socketPath;
                port = 1443;
                matchPatterns = ["*/*"];
                pruneUtxo = true;
                inherit since;
                logLevel = "Error";
              };
              kupo-indexer = {
                enable = true;
                nodeConfigPath = "${kupoNodeConfig}";
                nodeSocketPath = socketPath;
                port = 1442;
                matchPatterns = ["${validatorHash}/*"];
                inherit since;
                logLevel = "Debug";
              };
            };
          };

          # Kupo retries the node socket on its own, but starting it only once the
          # node answers keeps the boot sequence readable.
          systemd.services = {
            kupo-utxo = {
              after = ["cardano-node-ready.service"];
              requires = ["cardano-node-ready.service"];
            };
            kupo-indexer = {
              after = ["cardano-node-ready.service"];
              requires = ["cardano-node-ready.service"];
            };
          };

          services.konduit-indexer = {
            enable = true;
            package = indexerPackage;
            kupoHost = "127.0.0.1";
            kupoInstance = "kupo-indexer";
            kupoPort = 1442;
            interval = "10s";
          };

          services.konduit-server = {
            enable = true;
            package = serverPackage;
            # Bound on every interface so the forwarded port on the host reaches
            # it; the firewall is opened for it below.
            host = "0.0.0.0";
            openFirewall = true;
            port = 5663;
            inherit socketGroup;
            # Holds KONDUIT_SIGNING_KEY and KONDUIT_HOST_ADDRESS.
            environmentFile = "${secretsPath}/${lib.baseNameOf hostEnvFile}";
            cardano = {
              backend = "node";
              nodeSocketPath = socketPath;
              nodeNetworkMagic = networkMagic;
            };
            # No LND in this guest.
            bln.mock = true;
            fx = {
              every = "10m";
              # Fixed rates keep the guest deterministic and offline. Point the
              # server at a live source through the env file instead.
              ada = 0.6;
              bitcoin = 60000.0;
            };
          };
        }
      )
    ];
  }

{
  lib,
  cardonnay,
  cardano-node,
  cardano-cli,
  coreutils,
  formats,
  writeText,
  writeShellApplication,
}:
(formats.yaml {}).generate "process-compose.yaml" {
  version = "0.5";
  processes = import ./testnet-processes.nix {
    inherit lib coreutils writeShellApplication writeText formats cardonnay cardano-node cardano-cli;
  };
}

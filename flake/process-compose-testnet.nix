{
  lib,
  coreutils,
  writeText,
  writeShellApplication,
  formats,
  cardonnay,
  cardano-node,
  cardano-cli,
}:
(formats.yaml {}).generate "process-compose.yaml" {
  version = "0.5";
  processes = import ./process-compose/testnet-processes.nix {
    inherit lib coreutils writeShellApplication formats cardonnay cardano-node cardano-cli;
  };
}

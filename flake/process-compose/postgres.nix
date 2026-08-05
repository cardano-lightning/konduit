{
  lib,
  coreutils,
  formats,
  postgresql,
  writeText,
  writeShellApplication,
  sqitchPg,
}:
(formats.yaml {}).generate "process-compose.yaml" {
  version = "0.5";
  processes = import ./postgres-processes.nix {
    inherit lib postgresql coreutils writeText writeShellApplication sqitchPg;
  };
}

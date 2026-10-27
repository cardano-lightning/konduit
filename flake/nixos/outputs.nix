# Imported from flake.nix:
#   nixos = import ./flake/nixos/outputs.nix { self = self; nixpkgs = inputs.nixpkgs; };
{
  self,
  nixpkgs,
}: {
  nixosModules.kupo = import ./kupo.nix;
  nixosModules.konduit-indexer = import ./konduit-indexer.nix;
  nixosModules.konduit-server = import ./konduit-server.nix;

  mkPreprodDeployment = import ./preprod-deployment.nix {inherit self nixpkgs;};
}

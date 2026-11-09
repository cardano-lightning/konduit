# Imported from the heavy flake with its application flake and nixpkgs inputs.
{
  self,
  nixpkgs,
}: {
  nixosModules.kupo = import ./kupo.nix;
  nixosModules.konduit-indexer = import ./konduit-indexer.nix;
  nixosModules.konduit-server = import ./konduit-server.nix;

  mkPreprodDeployment = import ./preprod-deployment.nix {inherit self nixpkgs;};
}

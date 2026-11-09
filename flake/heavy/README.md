# Heavy Cardano development flake

This flake contains the Cardano node/CLI, Kupo, Cardonnay, and process-compose
environment, as well as the NixOS preprod VM configuration. Its dependencies
are kept separate from the default repository development shell.

From the repository root, start the local testnet environment with:

```sh
nix --extra-experimental-features 'nix-command flakes fetch-closure' develop path:./flake/heavy
process-compose-dev-env
```

The first invocation may ask you to trust the IOG binary cache configured by
this flake.

For details on the preprod guest, see
[`nixos/README.md`](nixos/README.md).

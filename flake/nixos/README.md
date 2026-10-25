# NixOS deployment

The dev env ([../process-compose/dev-env.nix](../process-compose/dev-env.nix))
runs a local testnet through `cardonnay` and the konduit services next to it.
This directory is the same set of services as systemd units, plus a VM that
runs them against **preprod** instead of a local testnet.

## Modules

| Module                | Units                                                                |
| --------------------- | -------------------------------------------------------------------- |
| `kupo.nix`            | `kupo-utxo.service`, `kupo-indexer.service` + `-ready.service` gates |
| `konduit-indexer.nix` | `konduit-indexer.service` (oneshot) + `konduit-indexer.timer`        |
| `konduit-server.nix`  | `konduit-server.service`                                             |

They are exported as `nixosModules.kupo`, `nixosModules.konduit-indexer` and
`nixosModules.konduit-server`, and are wired together by
`preprod-deployment.nix`, which is exported as
`nixosConfigurations.preprod`.

Each unit mirrors a process of the same name in the dev env. The two
deviations from a straight mapping are worth knowing about:

- **Readiness gates.** Kupo is `Type=simple` and does not notify, so every
  instance gets a `<name>-ready.service` oneshot. `kupo health-check` exits 0
  once the instance reports a node connection, and downstream services declare
  `requires = [ "<name>-ready.service" ]` instead of racing Kupo. The node
  itself gets the same treatment through `cardano-node-ready.service` in the
  deployment, since `cardano-cli query tip` is the only reliable "the node is
  up" signal.

- **The indexer is scheduled, not continuous.** `konduit-indexer index` runs a
  single sync pass and exits, which the dev env expresses with a
  `schedule.interval = "10s"`. Here it is a timer feeding a oneshot, which
  also keeps passes from overlapping.

## The guest

`preprod-deployment.nix` is a plain function returning a `nixosSystem`, in the
same style as `marlowe-plutus/nix/nixos/test-deployment.nix`. It runs:

```
cardano-node (preprod, from the shared snapshot)
  └── cardano-node-ready.service
        ├── kupo-utxo.service        # */*, pruned,      :1443
        └── kupo-indexer.service     # <validator>/*    :1442
              └── kupo-indexer-ready.service
                    └── konduit-indexer.service   # every 10s
        └── konduit-server.service   # :5663
```

Things worth reviewing:

- **The snapshot is shared from the host.** `KONDUIT_PREPROD_DB` points at a
  preprod database prepared on the host and mounted at `/mnt/preprod-db` over
  9p. It is the directory cardano-node runs on, so it is what ends up as its
  `--database-path`: the one holding `immutable/`, `ledger/`, `volatile/` and
  `protocolMagicId`. It is passed as a plain string on purpose: a Nix _path_
  would be copied into the store, and the database is far too big for that.
  `cardano-node` runs as `KONDUIT_PREPROD_DB_OWNER_UID` inside the guest so it
  owns the shared files.

- **Kupo gets its own node config.** Kupo reads the node config for the Byron
  and Shelley genesis files it points at and resolves them next to the config
  itself. The cardano-node module writes its config into the store under a
  generated name, so the deployment hands Kupo a two key config referencing the
  very same genesis files.

- **Both Kupo instances start `--since tip`.** Preprod is two years of chain;
  the guest follows the snapshot forward instead of walking it. Use `origin`
  or a `<slot>.<hash>` point when a test needs history.

- **Secrets stay out of Nix.** `KONDUIT_SERVER_ENV_FILE` points at an env file
  on the host, shared read-only into the guest. It has to define at least:

  ```
  KONDUIT_SIGNING_KEY=<adaptor signing key, also the admin wallet>
  KONDUIT_HOST_ADDRESS=<address hosting the konduit reference script>
  ```

  Anything else the server understands can go there too. Note that systemd
  reads `EnvironmentFile` after `Environment=`, so it wins over the values the
  module derives. Do not set `KONDUIT_NETWORK` there for the node backend: it
  is parsed even though the backend ignores it.

  The guest uses the mock BLN backend and fixed FX rates, so the e2e flow needs
  neither an LND nor a price feed. Both are options of `services.konduit-server`
  for a deployment that wants them.

## Guest architecture

The guest is built for the architecture of the machine that evaluates the
flake, so that it can use hardware virtualization (KVM on Linux, HVF on
macOS). The architectures we can build it for are listed in `guestSystems` in
`flake.nix`. That is `x86_64-linux` only today, because kupo's flake builds
`x86_64-linux` only. On a machine we cannot build a guest for, the
evaluation fails with an error rather than falling back to another
architecture.

To support another architecture:

1. make kupo build for it,
2. add it to `guestSystems` in `flake.nix`,
3. add it to `systems` in `flake.nix`, so that the konduit indexer and the
   konduit server are built for it as well.

The VM runner and QEMU are built for the host rather than for the guest
(`virtualisation.host.pkgs`), so the runner is usable on a machine that is
not of the guest's architecture.

## Running it

Nothing here has been built or booted yet. Once the snapshot is prepared:

```sh
# the preprod database owned by the uid the guest runs cardano-node as
export KONDUIT_PREPROD_DB=/path/to/preprod/db
export KONDUIT_PREPROD_DB_OWNER_UID=$(id -u)

# env file with KONDUIT_SIGNING_KEY and KONDUIT_HOST_ADDRESS
export KONDUIT_SERVER_ENV_FILE=/path/to/server.env

# build the guest (cardano-node is not substitutable, this is a long build)
nix build .#nixosConfigurations.preprod.config.system.build.vm

# boot it
./result/bin/run-preprod-vm
```

The environment variables above are read at evaluation time, so they have to be
exported before every `nix build`.

`run-preprod-vm.sh` in the repository root wraps this up, with the same
variables and a few conveniences:

```sh
./run-preprod-vm.sh --eval      # evaluate the configuration only
./run-preprod-vm.sh --dry-run   # show what would be built
./run-preprod-vm.sh             # build the guest and boot it
./run-preprod-vm.sh --console   # boot it headless, serial console in the terminal
```

From the host, once the guest is up:

| Service        | URL                            |
| -------------- | ------------------------------ |
| konduit-server | `http://127.0.0.1:18090/info/` |
| kupo-indexer   | `http://127.0.0.1:11442`       |
| kupo-utxo      | `http://127.0.0.1:11443`       |

The guest autologins `root` on the serial console, so
`systemctl status kupo-indexer konduit-indexer konduit-server` and
`kupo health-check --port 1442` are available there.

The node in the guest is 11.0.1 from the `cardano-node` flake input, while the
dev env runs the 10.2.1 build from `capkgs`. That is deliberate: the guest
should run the node release rather than a pinned one.

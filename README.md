<p align="center"><a href="https://konduit.channel" target="_blank"><img src="./.assets/logo.png" alt="konduit-logo" width="150" height="150" ></a></p>

# Konduit

> A payment network built on Cardano channels.

This project is incubating. Expect sharp edges.

## Vision

An ada holder can have the same experience as a BLN user when paying merchants.

See the [design](./docs/design/) for more details.

## Structure

- [`./connector`](./connector/) - API query layer for Cardano.
- [`./docs`](./docs/) - Meeting notes, adrs, design docs.
- [`./kernel`](./kernel/) - Konuit kernel aka on-chain code. An Aiken repo.
- [`./rust`](./rust/) - Tools for konduit. A rust workspace.

## API Reference(s)

- [Rust & WASM API Reference](https://cardano-lightning.github.io/konduit/rust)

## Contributing

TBC.

## Developing

### Using nix

For the base devel:

```
$ nix develop
```

For something extra, e.g. local deployment using process-compose, cardano-cli, jailed-agents/opencode use:

```
$ nix develop .#extras --impure
```

The `--impure` flag allows nix to read `KONDUIT_EXTRA_OPENCODE_RW_DIRS` which should be `:` separated list of directories that you want to be able to read/write from the jailed agents (beside some default ones: ~/.config/opencode, ~/.local/share/opencode, ~/.cache/opencode).

## Integration testing

In the `./packages/integration` you can find a test suite which executes a few e2e scenarios. In order to run the tests multiple services need to be running together with a local cardano node. The easiest way is to use mentioned above `nix develop .#extras --impure` and then run the following command:

In one shell:

```shell
$ nix develop .#extras --impure
$ process-compose-dev-env
```

In another shell:

```shell
$ nix develop .#extras --impure
$ cd packages/integration
$ yarn install
$ vitest --disableConsoleIntercept  --run
```

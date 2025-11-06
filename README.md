# Konduit

> A Cardano to Bitcoin Lightning Network pipe.

⚠️This project is a WIP. Use at own risk. ⚠️

## Vision

An ada holder can have the same experience as a BLN user when paying merchants.

See the [requirements](./docs/design/requirements.md) for full details.

## Repo Org

- [`./docs/`](./docs/) - Meeting notes, adrs, design docs
- [`./kernel/`](./kernel/) - Konuit kernel aka on-chain code. An Aiken repo.
- [`./rust/`](./rust/) - Tools for konduit. A rust workspace

## Contributing

TBC.

## Developing

For the base devel:

```
$ nix develop
```

For something extra, e.g. cardano-cli, use:

```
$ nix develop .#extras
```

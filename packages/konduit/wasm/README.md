# Konduit-wasm

A WASM-friendly API for Konduit.

## Pre-requisite

### wasm-pack

```console
cargo install wasm-pack
```

### wasm32 rustool target

```console
rustup target add wasm32-unknown-unknown
```

### WebAssembly/binaryen

For optimized release builds: see
[WebAssembly/binaryen](https://github.com/WebAssembly/binaryen).

## Compiling browser bundles

```console
make
```

This builds:

- `konduit-wasm-nightly-core`: core types only
- `konduit-wasm-nightly-black-box`: the full API, including `Konduit`

Both bundles include generated TypeScript declarations.

## Example

- [node.js](./examples/node.js/README.md)
- [browser](./examples/browser/README.md)

Opening a channel requires its immutable asset identity. Amounts are raw units;
the minimum-Ada reserve is not part of channel capacity:

```js
const ada = AssetId.ada();
await konduit.openChannel(tag, 10_000_000n, ada);

const usdm = AssetId.native(
  "c48cbb3d5e57ed56e276bc45f99ab39abe94e6cd7ac39fb402da47ad",
  "0014df105553444d",
);
await konduit.openChannel(tag, 10_000_000n, usdm);
```

`ChannelOutput.asset` returns the selected identity and
`ChannelOutput.totalAmount` returns raw selected-asset units.

## Documentation

```console
npx typedoc
npx serve docs
```

And then, visit http://localhost:3000/modules/wasm_bindgen

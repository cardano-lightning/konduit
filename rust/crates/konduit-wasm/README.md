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

## Compiling node.js & browser

```console
make
```

### Browser only

```console
make browser
```

This produces JavaScript & WASM files under `./konduit-wasm-browser`

### Node.js only

```console
make nodejs
```

This produces JavaScript & WASM files under `./konduit-wasm-nodejs`

## Example

- [node.js](./examples/node.js/README.md)
- [browser](./examples/browser/README.md)


## Documentation

```console
npx typedoc
npx serve docs
```

And then, visit http://localhost:3000/modules/wasm_bindgen

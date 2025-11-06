# Konduit-wasm

A WASM-friendly API for Konduit.

## Pre-requisite

### cargo make

```console
cargo install cargo-make
```

### wasm32 rustool target

```console
rustup target add wasm32-unknown-unknown
```

### WebAssembly/binaryen

For optimized release builds: see [WebAssembly/binaryen](https://github.com/WebAssembly/binaryen).

## Compiling

```console
cargo make dev

# or for release

cargo make release
```

This produces JavaScript & WASM files under `./pkg`

## Example

### open

Using the `nodejs` target.

```js
const konduit = require("./konduit_wasm.js");
const assert = require("node:assert");

const new_config = (deposit) => new konduit.OpenConfig(
  Buffer.from("00000000000000000000000000000000000000000000000000000000", "hex"),
  deposit,
  Buffer.from("0000000000000000000000000000000000000000000000000000000000000001", "hex"),
  Buffer.from("0000000000000000000000000000000000000000000000000000000000000002", "hex"),
  "abcd",
  24n * 3600n * 1000n,
);

const fuel_input = new konduit.Input(
  Buffer.from("0000000000000000000000000000000000000000000000000000000000000001", "hex"),
  0n,
);

const fuel_output = konduit.Output.new(
  "addr1v8u3ufhjht4q5kd6pvccqcdj69qfazjzz5j74m8dhwyfjqsw5339n",
  10000000n,
);

const resolved_inputs = konduit.ResolvedInputs.empty()
  .append(new konduit.ResolvedInput(fuel_input, fuel_output));

try {
  konduit.open(
    new_config(1234n),
    konduit.ProtocolParameters.mainnet(),
    konduit.NetworkId.mainnet(),
    resolved_inputs,
    fuel_input,
  );
  assert.fail("should have failed to open with insufficient funds");
} catch (e) {
  assert.equal(e, "insufficiently provisioned output(s): not enough lovelace allocated");
}

const open = konduit.open(
  new_config(2000000n),
  konduit.ProtocolParameters.mainnet(),
  konduit.NetworkId.mainnet(),
  resolved_inputs,
  fuel_input,
);

console.log(open.toString());
```

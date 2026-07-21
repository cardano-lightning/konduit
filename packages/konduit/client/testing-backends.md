# testing backends

Three `Backend` impls share one test suite (`backend_test_suite.rs`):
add a case once, every backend gets it.

- `InMemory` — native, plain `cargo test`.
- `FileBackend` — native, plain `cargo test` (human-readable JSON file).
- `IdbBackend` — wasm + real IndexedDB, needs a headless browser (below).

## Native (InMemory, FileBackend)

```
cargo test -p konduit-client
```

## Wasm / Idb

### Cargo

Modifications are required in `.cargo/config.toml` and `Cargo.toml`.

### NixOS one-time setup

Headless Firefox/`geckodriver` are prebuilt non-Nix binaries; `nix-ld`
lets them run:

```nix
programs.nix-ld.enable = true;
programs.nix-ld.libraries = with pkgs; [ glib nspr nss dbus xorg.libxcb ];
```

`sudo nixos-rebuild switch`. If a run then fails with a _new_
`libXXX.so: cannot open shared object file`, add `pkgs.XXX` to the list
and rebuild again — this list isn't guaranteed exhaustive for every
Firefox/geckodriver version.

Chrome/`chromedriver` was tried but abandoned.
"ChromeDriver started successfully" with `http status: 404` /
`invalid session id`. Apparently a long known bug.

### Run

```
wasm-pack test --headless --firefox --no-default-features --features idb,json
```

Always pass `--no-default-features` for wasm builds: `konduit-client`'s
`default` feature pulls in `tokio/full` via `cli`, which doesn't compile
on `wasm32-unknown-unknown` (`mio` has no wasm support).

## TODO

When stable add to CICD.

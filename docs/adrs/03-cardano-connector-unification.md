---
title: Cardano Connector Unification
status: proposed
authors: @KtorZ
date: 2026-02-24
tags:
  - connector
  - interfaces
  - coding practice
---

## Context

The current status quo around connectors is both hard to understand and present
unnecessary duplication and coupling. Part of the confusion comes from:

- [./rust/crates/cardano-connect](https://github.com/cardano-lightning/konduit/tree/245854ba784c06dbc8e71b06b43fb0abfe50ed51/rust/crates/cardano-connect):
  the trait definition which provides a Rust API to other crates for interacting
  with Cardano.

- [./rust/crates/cardano-connect-blockfrost](https://github.com/cardano-lightning/konduit/tree/245854ba784c06dbc8e71b06b43fb0abfe50ed51/rust/crates/cardano-connect-blockfrost):
  an implementation of the connector trait that is done through a direct
  blockfrost integration.

- [./connector](https://github.com/cardano-lightning/konduit/tree/245854ba784c06dbc8e71b06b43fb0abfe50ed51/connector)
  being an implementation of the _server side_ of a connector, which happens to
  be using Blockfrost and Koios behind the scene. The main reason for its
  existence in comparison to a direct Blockfrost integration is precisely to
  avoid exposing our Blockfrost credentials in a client application.

- [./rust/crates/cardano-connect-wasm](https://github.com/cardano-lightning/konduit/tree/245854ba784c06dbc8e71b06b43fb0abfe50ed51/rust/crates/cardano-connect-wasm):
  a client for the aforementionned connector server; which happens to be
  wasm-compatible (and more specifically, can only compile to wasm). Although
  the wasm bits are a bit orthogonal here. We could easily have the same client
  work through a 'standard' http client, and use it in the client, for example.

In the end, the direct Blockfrost client is okay for local development, but not
really sound for anything deployed and handed over to end users as it exposes
Blockfrost credentials. Having those clients as completely separate crates
provides little benefits, though. They can simply live as modules within the
main connector crate, and associated to specific feature flags.

## Decision

Unify, clarify and tidy the definition and usage of the cardano-connector(s)
crates and modules.

1. Provide an isomorphic HTTP client tailored to our needs, and which would
   roughly ressemble what's currently defined under
   [`http_client`](https://github.com/cardano-lightning/konduit/blob/245854ba784c06dbc8e71b06b43fb0abfe50ed51/rust/crates/cardano-connect-wasm/src/http_client.rs).
   To make it isomorphic, we can selectively use `gloo_net` or `reqwest` based
   on the target platform.

2. Rename the top-level `connector` project as `cardano-connector-server`.

3. Rename `cardano-connect` as `cardano-connector-client`

4. Move `cardano-connect-blockfrost` under `cardano-connector-client`, available
   through a `blockfrost` feature flag. Refactor to make use of the
   `http_client`.

5. Move `cardano-connect-wasm` under `cardano-connector-client`, available by
   defaut but selectively picking a wasm-compatible or not http client.

> [!NOTE]
>
> `gloo_net` does not support setting binary blobs as request bodies. And
> looking at the repository, the project is part of a larger framework that
> doesn't seem much maintained. Plus, the `gloo_net` bits are only a thin
> wrapper around `web_sys` native browser requests, which do support binary
> payloads. So it probably makes sense to internalize this dependency fully,
> prune out the bits we don't need (e.g. websocket support). That can be done
> within the isomorphic `http_client`.

## Discussion, Counter and Comments

## Consequences

In addition, to validate those changes:

- [ ] We should introduce more e2e tests, even if no fully automated (e.g. may
      still require to start and launch components independently) to at least
      test the non-wasm flow between the connector server and client.
- [ ] Give a try to
      [`wasm-pack test --headless`](https://drager.github.io/wasm-pack/book/commands/test.html)
      to see if we can manage to also automatically test the wasm bits.
- [ ] Make sure that interfaces are at least internally documented (i.e.
      `cargo docs`), but also seek to document (i.e. open api) the connector's
      server interface.

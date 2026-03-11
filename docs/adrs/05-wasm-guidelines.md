---
title: "WASM Guidelines"
author: @KtorZ
date: 2026-03-07
status: accepted
tags:
  - wasm
  - guidelines
---

## Context

Working with a Rust codebase, we've found it relevant to make the most of it.
Rust is particularly well-suited for WASM through libraries and tools such as
cargo, [wasm-bindgen](https://wasm-bindgen.github.io/wasm-bindgen/) and
[wasm-pack](https://drager.github.io/wasm-pack/).

While those tools make it easier to produce a WASM API for a client app, there
are a few pitfalls that may make the experience quite awful. This document
describes guidelines and conventions used in Konduit to make working with WASM a
joy.

## Decision

1. We use `wasm-pack` to produce wasm bundles for the browser. Example usages
   are available in our
   [Makefile](https://github.com/cardano-lightning/konduit/blob/399d18258021b21addf17be12223116fe009c8be/rust/crates/konduit-wasm/Makefile#L8-L14).

2. All wasm-specific code is located under
   [`konduit-wasm`](../../rust/crates/konduit-wasm). By default, the crate
   exposes minimal bindings for some core Konduit types. More complex APIs are
   then exposed behind feature flags, such as the `black-box-api`, which
   provides a complete encapsulation of an adaptor and a consumer l1 and l2
   clients, as well as a rudimentary wallet.

   This allows us to keep our main Rust code wasm-free and Rust-idiomatic,
   rather than being altered just to please some WASM API.

3. Some dependencies are prohibited in the lower parts of the codebase
   (`cardano-sdk`, `konduit-data`, etc) so that those crates remain easily
   cross-platform. In particular, we note that:
   - time manipulations and durations should come from `web-time` and not from
     `std::time`. The former re-exports the latter when compiling for non-wasm
     targets, and exports a web-compatible API when targeting wasm.
   - network effects shall be abstracted over traits with platform-specific
     implementations that can be chosen on the far edge of the system (e.g.
     `konduit-server`, `konduit-cli`, `konduit-wasm`). So far, we've only had
     need for an HTTP client, which is abstracted as
     [`http-client`](https://github.com/cardano-lightning/konduit/blob/399d18258021b21addf17be12223116fe009c8be/rust/crates/http-client/src/http_client.rs)

4. In general, it isn't necessary to worry about JavaScript objects' allocation
   and deallocation. This is correctly handled by the combo wasm-pack &
   wasm-bindgen, which produces wrapped objects that automatically `.free()`
   them when they go out of scope.

5. As a consequence, one should avoid taking ownership of values on the
   wasm-bindgen exposed API and only ever take references or values that
   implement `Copy`. Take, for example, the following:

   ```rs
   // DON'T
   #[wasm_bindgen]
   fn do_something(foo: Foo) -> SomeValue { ... }
   ```

   ```js
   const foo = new Foo();
   wasm.do_something(foo);
   console.log(foo.toString());
   ```

   On the JavaScript side, the `foo.toString()` will trigger a null-pointer
   exception because `do_something` has effectively consumed `foo`. Since
   there's no borrow-checker in JavaScript, this is a nice footgun. In general,
   we shall therefore prefer using references on the Rust side and explicitly
   cloning data when needed.

   ```rs
   // DO
   #[wasm_bindgen]
   fn do_something(foo: &Foo) -> SomeValue { ... }
   ```

6. To keep the code Rust free of WASM concerns, we only export thin proxies
   through `wasm_bindgen` instead of their respective Rust equivalent. For
   example:

   ```rs
   #[wasm_bindgen]
   #[repr(transparent)]
   #[derive(Debug, Clone, Copy)]
   /// A network identifier to protect against misuse of addresses or transactions on the wrong network.
   pub struct NetworkId(cardano_sdk::NetworkId);

   impl ::konduit_wasm::wasm::WasmProxy for NetworkId {
       type T = cardano_sdk::NetworkId;
   }

   impl ::std::ops::Deref for NetworkId {
     type Target = cardano_sdk::NetworkId;
     fn deref(&self) -> &Self::Target {
       &self.0
     }
   }

   impl ::std::borrow::Borrow<cardano_sdk::NetworkId> for NetworkId {
     fn borrow(&self) -> &cardano_sdk::NetworkId {
       &self.0
     }
   }

   impl ::std::convert::From<NetworkId> for cardano_sdk::NetworkId {
     fn from(proxy: NetworkId) -> Self {
       proxy.0
     }
   }

   impl ::std::convert::From<cardano_sdk::NetworkId> for NetworkId {
     fn from(inner: cardano_sdk::NetworkId) -> Self {
       Self(inner)
     }
   }
   ```

   This allows us to use both types almost interchangeably at the boundaries
   between Rust and WASM, while attaching a WASM-specific API on the proxy
   without polluting the underlying type. Conveniently, we provide a simple
   macro to declare new proxies and their associated trait instances:

   ```rs
   ::konduit_wasm::wasm_proxy! {
     #[derive(Debug, Clone, Copy)]
     #[doc = "A network identifier to protect misuses of addresses or transactions on a wrong network."]
     NetworkId => cardano_sdk::NetworkId
   }
   ```

   > [!NOTE]
   >
   > Doc comments `///` don't play well with the macro; so use the
   > `#[doc = "..."]` macro attribute to attach documentation to the proxy. This
   > is picked up by our CI pipeline when building the WASM/JS API reference.

7. Rust-exported types to wasm cannot have generic parameters nor lifetimes. For
   generics, we can simply monomorphize types as needed and export one type per
   generic instantiation. For example:
   - [ShelleyAddress](https://github.com/cardano-lightning/konduit/blob/399d18258021b21addf17be12223116fe009c8be/rust/crates/konduit-wasm/src/wasm/shelley_address.rs#L12)
   - [Hash28](https://github.com/cardano-lightning/konduit/blob/399d18258021b21addf17be12223116fe009c8be/rust/crates/konduit-wasm/src/wasm/hash28.rs)
   - [Hash32](https://github.com/cardano-lightning/konduit/blob/399d18258021b21addf17be12223116fe009c8be/rust/crates/konduit-wasm/src/wasm/hash32.rs)

   For lifetimes, use owned values when possible or rely on `Rc` if sharing a
   reference is really needed. For example:
   - [Connector](https://github.com/cardano-lightning/konduit/blob/399d18258021b21addf17be12223116fe009c8be/rust/crates/konduit-wasm/src/wasm/connector.rs#L7)

8. Rust uses `snake_case` while JavaScript usually uses `camelCase`. So, every
   exported WASM function shall use the `js_name` macro field from
   `wasm_bindgen` to define a JavaScript idiomatic name in camelCase for
   functions:

   ```rs
   #[wasm_bindgen(js_name = "toString")]
   ```

9. To avoid weird cyclic dependencies when calling functions on a proxied type,
   we religiously prefix all WASM-exported Rust methods with `_wasm`. We use
   `js_name` anyway to control their JavaScript name. This allows us to
   disambiguate which object is targeted by a method that would exist both on
   the proxy and its proxied type. For example:

   ```rs
   #[wasm_bindgen(js_name = "toString")]
   pub fn _wasm_to_string(&self) -> String {
     self.to_string()
   }
   ```

10. We expose a `wasm::Result<T>` type in `konduit-wasm`, which is suitable for
    `wasm_bindgen` and plays nicely with `anyhow::Result` and the `?` operator.

11. `konduit-wasm` also exposes a very useful `enable_logs_and_panic_hook`
    global method, which allows:
    1. Propagating Rust logs from the `log` crate (e.g. `log::info`) to the
       browser console, with a configurable minimum severity.
    2. Installing a panic hook to turn Rust panics into proper JavaScript
       errors.

    It is therefore highly recommended to call this function early in the
    JavaScript stack to improve WASM debugging.

## Discussion, Counter and Comments

- The specific needs of each application for WASM are hard to anticipate and
  must seemingly be tailored to each downstream consumer. It is preferable to
  expose multiple WASM APIs, each behind a feature flag, so they can be
  selectively enabled. This is the case of the now 'black-box' API used by
  Ferret.

## Consequences

- All crates but `konduit-wasm` have been cleaned up of any wasm-specific code.
- We diligently rely on `web_time` instead of `std::time` in the codebase, which
  selects the right implementation based on the implementation target.

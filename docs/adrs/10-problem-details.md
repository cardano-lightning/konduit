---
title: "Problem Details"
author: "@waalge"
date: 2026-05-26
tags:
  - error handling
  - http
  - cbor
---

## Context

We need a consistent error response format across HTTP APIs.
[RFC 9457](https://www.rfc-editor.org/rfc/rfc9457.html), Problem Details for
HTTP APIs) provides a well-specified, interoperable structure for this.

This came about during a broader exploration of how to version APIs and their
error contracts.

## Decision

We will adopt RFC 9457 as our standard error response format, implemented in a
`problem-details` family of crates under `packages/util/problem-details/`.

The crate family will be thinly sliced:

- `wire`: types and encoding only — safe for wasm/client targets
- `derive`: proc macros for implementing the wire traits
- `server`: server-side integrations (actix, etc) behind feature flags

## Decent, counter, and comments

- **Comment**: Thin slicing ensures client (wasm) code cannot accidentally pull
  in server-side dependencies. `wire` carries no server deps by construction.

- **Comment**: We hope this structure — a `wire` crate plus an optional `server`
  integration crate — serves as a blueprint for future feature crates that need
  to straddle client and server targets.

## Status

Proposed

## Consequences

- **Positive**: Consistent, spec-backed error responses across all HTTP APIs.
- **Positive**: Clean target boundary — wasm consumers depend only on `wire`.
- **Negative**: Small overhead of an additional crate family for what might
  otherwise be inline error types.

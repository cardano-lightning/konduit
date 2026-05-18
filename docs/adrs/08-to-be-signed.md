---
title: "To be signed"
author: "@you"
date: 2026-01-01
tags:
  - tbs
  - consumer
  - cbor
---

## Context

The current implementation, the To-Be-Signed (TBS) bytes are constructed by
concatenation: `tag + cbor.encode(stuff)`. The Tag consists of user-defined
bytes, and stuff represents the data structure being signed.

This approach is almost surely fine; yet it is not _obviously_ fine. It
introduces a potential "footgun": two pairs of `(tag, stuff)` that result in the
same TBS.

We need clarity when using the key to sign new types, such as client-to-server
auth.

## Decision

Any TBS is valid CBOR.

There is a single registry of TBS forms.

We assume plutus compat encoding for arrays (ie indefinite when non-empty).

We recommend off-chain usage include domain labels as a first entry.

### Registry

The current registry consists of Cheques and Squashes (or bodies thereof).

As CDDL:

```cddl
channel_tag = bytes
index = uint
amount = uint
lock = bytes .size 32
timeout = uint ; posix time in milliseconds
excludes = [* index] ; 0 or more
```

#### Cheque

```cddl
cheque_body = [channel_tag, [index, amount, lock, timeout]]
```

#### Squash

```cddl
squash_body = [channel_tag, [amount, index, excludes]]
```

#### Reg

```cddl
token = bytes .size 32
adaptor_key = bytes .size 32
reg_body =    ["REGISTER_TOKEN", channel_tag, [token, adaptor_key]]
```

## Decent, counter, and comments

Comment: This approach mimics the Sig_structure found in the COSE (RFC 9052)
standard without requiring the full overhead of the COSE library.

Counter: Wrapping in an array adds a few bytes of overhead (the CBOR array
header). It is mildly more expensive on-chain. However, for the security
guarantees provided, this overhead is negligible.

Comment: Using a registry helps with documentation and long-term maintenance, as
it forces developers to explicitly declare new signable types.

## Status

Proposed

## Consequences

Positive: Make explicit and obvious how we avoid TBS collisions.

Negative: This is a breaking change for any existing signatures created using
the previous concatenation method.

Neutral: Requires developers to update the registry when adding new features
that require signing.

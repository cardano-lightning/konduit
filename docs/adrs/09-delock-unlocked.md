---
title: "De-lock Unlocked"
author: "@you"
date: 2026-05-18
tags:
  - cheque
  - cbor
---

## Context

Currently `Unlocked` includes both the lock and the secret. This is superfluous.

## Decision

The new data structure will be:

```
locked = [[index, amount, timeout, lock], signature]
unlocked = [index, amount, timeout, secret, signature]
effective_tbs = cbor([tag, [index, amount, timeout, hash(secret)]])
```

In the case of `respond`, we keep `Cheque` to be a wrapper of the two variants.

## Decent, counter, and comments

## Status

proposed

## Consequences

Changes to on and off-chain code. Specifically Adaptor's code, not Consumer's.

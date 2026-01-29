---
title: "Fix add spamming"
status: proposed
authors: "@waalge"
date: 2025-01-01
tags:
  - Exploit fix
---

## Context

Both Consumer and Adaptor can spend an opened channel UTXO, by an add and sub
step respectively. Two parties attempting to spend the same UTXO hits
contention, where one transaction will fail.

In the current design there is no "rate limiting" mechanism. It is possible for
Consumer to DoS Adaptor, by repeatedly submitting add steps, preventing Adaptor
from submitting a sub.

The impact is most severe when Adaptor has an unlocked approaching timeout. A
successful DoS results in a loss of principal of the associated cheque. Ballpark
figures on the cost of an attack are:

- 0.4 ada for single add step
- 3 transaction a minute, to cause sufficient issues
- 400 minutes is the expected time delta

Which works out at 500 ada. However, an attacker could attack many channels
simultaneously. A single tx with 10 steps may cost only 1 ada. Furthermore, an
attacker could exploit a period of _temporal divergence_ between Cardano and
Bitcoin bitcoin networks. This shorten the time required to fund the DoS.

A note on Temporal divergence. Temporal divergence is a divergence in time
between Cardano's Plutus (posix time) vs BLN's commitments that use bitcoin
block height. Bitcoin blocks are produced via a process that can be modelled as
an exponential distributions. Divergence can occur simply because this is a
random process, although its more likely to be significant due to external
factors such as a sudden drop in hashing power. Detailed calculations are
elsewhere.

## Decision

### Overview

There are two proposals to introduce rate limiting: time based, or count based.

#### Time based

Opened stage has `add_at : Option<Int>`. This records the upper bound of the
last transaction if it contained an add step of the channel.

Logic:

- On add step, if `Some(ub_prev) = opened_curr.add_at` then transaction lower
  bound `lb_curr > ub_prev + TIME_BUFFER`. In either case, continuing output has
  `opened_cont.add_at = Some(ub_prev)`
- On sub step, continuing output has `opened_cont.add_at = None`

#### Count based

Opened stage has `add_count : Int`. This records the number of times an add has
been executed since last sub.

Logic:

- On add step `opened_cont.add_count = opened_curr.add_count + 1`, and
  `opened_cont.add_count < MAX_ADD_COUNT`
- On sub step, continuing output has `opened_cont.add_count = 0`

### Rationale

Either solution looks like it would address the issue identified. Neither
solution looks to present significant advantages over the other.

#### Time based

Pros:

- Time based is most explicitly "rate limiting" in the sense identified.

Cons:

- requires additional context at the logic level, namely upper and possibly also
  lower bounds of the validity range. Currently no single step requires both
  upper and lower bounds.
- validation with time is much more annoying to test.
- There are cases it will reject which will be legitimate. For example: Consumer
  tops up by 50 ada, but accidentally does 5ada. They must then wait, say,
  another 25 minutes (20 minute transaction upper bound + 5 time buffer), to fix
  this.

#### Count based

Pros:

- avoids handling time
- seems to prevent the attack as described, without preventing safe behaviour,
  like two adds in quick succession.

Cons:

- Only indirectly addresses the problem identified. The greater `MAX_ADD_COUNT`
  is chosen to be, the less this is a clear cut solution.
- No data to inform choice of `MAX_ADD_COUNT`.
- Its possible the there are cases where Consumer wants to exceed
  `MAX_ADD_COUNT`, especially if chosen too low.

## Discussion, Counter and Comments

### Comments

This came about as a result of work on the adaptor risk assessment, where the
attack vector was quantified.

Its proposed that choosing and implementing a patch for this be suspended until
the current version is approaching MVP.

### Considered Alternatives

The are more complex solutions that use ideas from both. This could, for
example, allow 5 submissions within a given window, before requiring either a
sub, or a call-off period. This just seems too complicated to be worth the LoC.

## Consequences

Both proposed patches will require changes to the spec, kernel, tx-builder. In
the case of time based, additional options might need to be exposed in the CLI
tool.

---
title: "Peer Protocol"
---

# Overview

The _Peer Protocol_ refers to off-chain state maintained by participants, and
the messages exchanged between them. The Bitcoin Lightning Bolt to which this
document is most closely analogous is Bolt 2, the peer protocol.

# Design

## Overview

The protocol adopts a request-response behaviour, driven by Consumer. This
choice is based in part on the assumption that Consumer's client is a mobile app
and is not in general reachable, while Adaptor is running an HA server.

There is a further asymmetry between Consumer and Adaptor. Adaptor collects
evidence of funds owed from the L1, while Consumer recovers anything left over.
Consumer can safely defer their source of truth to Adaptor.

In the current version we stick to HTTP, with no assumption on transport layer.
Messages are json encoded. Bytes, such as verification keys, are first hex
encoded. Bodies that are signed, ie cheque bodies and squash bodies, are first
encoded as Plutus data and then hex encoded. These choices are selected for
simplicity in implementation and debugging.

A second version should consider alternatives. For example, they should be
contrasted with
[Bolt 8](https://github.com/lightning/bolts/blob/master/08-transport.md)
specifying the use of UDP and noise, and bespoke message formats
[Bolt 1](https://github.com/lightning/bolts/blob/master/01-messaging.md).

For both participants managing channels is, for the most part, embarrassingly
parallel. The effective ID of a channel is the **keytag**, which is the
concatenation of `add_vkey` (AKA consumers verificiation key) and `tag`. Recall
their is a subtlely here in that we cannot assume that this is unique for L1
component.

## Peer protocol stages

Written from the perspective of Adaptor, we have the following stages. A stage
can be understood as the presence or not of:

- a retainer, ie the an L1 channel with ammeanable value, datum, _etc_.
- a receipt.

### No retainer, no receipt

This is the beginning: There is no relationship between Consumer and Adaptor.

Consumer first learns of Adaptor's service (out-of-band). Consumer can then
query for Adaptor's info ie conditions of acceptable channel. It includes
information on the channel conditions Adaptor finds acceptable, for example the
`sub_vkey` and `close_period`. Consumer opens a channel with acceptable values.

Adaptor is watching the L1. From the persepective of the Adaptor, there is still
no relationship with Consumer until the confirmation of the L1 channel.

Adaptor does not handle cheques at this stage. On confirmation, Adaptor now has
a retainer, but no receipt.

### Retainer, no receipt

Adaptor awaits the null squash from Consumer. The null squash acts as their
initial handshake. Adaptor does not handle cheques at this stage. On receiving a
squash, Adaptor has a receipt.

### Retainer, receipt

Adaptor does handle cheques at this stage, and has confidence that payments made
are recoverable from retainer. Whenever they choose, Adaptor submits subs, using
the latest receipt to recover funds owed. In particular, they must use unlockeds
not squashed before the timeout.

Recall Adaptor must not submit a respond step if there is no longer a retainer.
There is a subtlety here. Adaptor only considers retainers from _confirmed_ L1
channels, whereas txs are submitted against tip which may or may not be
considered confirmed. Obviously if a tx is successful, then tautologically the
inputs were confirmed, but this is only leared later. There are different ways
to safely handle this:

- Verify that, were the current UTXO set confirmed, then there would be a
  retainer of at least as much claimable value as the one currently in state.
- Suspend the handling of cheques, at least temporarily.

Adaptor periodically syncs with the L1, updating the retainer where appropriate.
If adaptor sees there are no L1 channels that provide as retainers, then the
retainer is set to none.

### No retainer, receipt

Adaptor has channel with no retainer, and so can no longer handle cheques. Once
Adaptor has confirmed the receipt is exhausted against available L1 channels,
they may safely forget it.

As there is no preventing "mimic" channels, there is no preventing an L1 channel
appearing at tip mimicking one that has already ended. From Adaptors
perspective, there is nothing unsafe here even if the receipt is forgotten. From
Consumers perspective, they have re-used a tag and, unless they absolutely know
what they are doing, their funds are at risk.

# Details

The protocol is described in terms of requests, their content, the responses
based on conditions. Anywhere it reads words to the effective of "participant
must" it should be understood that this is to keep Actor safe, and has no
bearing on the the other participant's safety.

## State

Each participant must keep their respective signing key safe. Loosing the
signing key may lead to locked funds; exposing a key may lead to a third party
taking funds.

### Consumer state

To be safe, Consumer needs to persist nothing more than their signing key. All
state required to recover their funds can be determined from the L1, and the
derived verification key. That is, a re-indexing of the Konduit script address
filtering on `add_vkey` will recover all l1 channels belonging to Consumer.

To practically interact with Adaptor, they should persist Adaptor information
including location and channel tag.

To be safe, while not syncing with the L1, Consumer must record all tags used.
Unless they absolutely know what they are doing, they must not re-use a tag.

All other persisted state should be treated as informational.

### Adaptor state

Adaptor maintains a list of Channels they are or have been engaged with.

A channel has the following form

```rust
struct Channel {
    keytag: Keytag,
    retainer: Option<Retainer>,
    receipt: Option<Receipt>,
    aux: Aux
}

struct Retainer {
    amount: u64,
    subbed: u64,
    used: Vec<Used>,
}

struct Receipt {
    squash: Squash,
    cheques: Vec<Cheque>,
}

enum Cheque {
    Unlocked({ body, signature, secret })
    Locked( { body, signature } )
}

struct Aux {
    is_active: bool,
}
```

Adaptor must ensure state is persisted.

## L1 sync

Consumer submits an open transaction. Consumer then awaits Adaptor to confirm
that an L1 channel exists.

Adaptor periodically syncs their local state with their view of the L1 tip. More
precisely, they filter tip on all utxos at the script address with valid value
and datum, and matching `sub_vkey` ie their L1 channels. Adaptor must have
confidence the transaction is beyond a rollback.

Once an opened L1 channel is considered confirmed at tip, it can be considered a
retainer of the channel. If there is more than one, then the L1 channel with
most "sub-able" amount (with respect to the L2 channel state) is recorded. If
there is no opened L1 Channel for the keytag, then the `channel.l1` is `None`.

Caculating most sub-able is subtle, and must handle the possibility of mimics,
and other divergences of state. It works as follows. Filter only well-formed L1
channels in `Opened` stage. If there is a receipt then set

```rust
let squash_amount = receipt.squash.body.amount;
let cheques = receipt.cheques();
```

Else take these to be `0` and `[]` respectively. Calculate the potentially
sub-able funds. Note that this requires both filtering cheques that unused, and
filtering useds that are not squashed.

```rust
let used_indexes = channel.useds.iter().map(|i| i.index).collect();
let unused = cheques.iter()
    .filter(|i| !used_indexes.has(i.index()))
    .map(|i| i.amount())
    .sum();
let unsquashed = channel.useds.iter()
    .filter(|i| !used_indexes.has(i.index()))
    .map(|i| i.amount).sum();
let owed_total = squash_amount + unused + unsquashed;
let owed_rel = owed_total.saturated_sub(channel.subbed)
let sub = cmp::min(owed_rel, channel.amount)
sub
```

If there are two L1 channels with the same sub-able amount, then choose the one
with greatest `channel.amount`. That is, take the max on tuples
`(sub, channel.amount)`.

In "normal" operation, the L1Channel state is result of "normal" transactions.
However, this is not enforced and must not be assumed. It is possible to reach a
state in which the retainer has a `used`, that does not correspond to an
unlocked, and possibly even a locked. Regardless, the formula should result in
the most sub-able retainer, and this includes these "normally" absurd scenarios.

FIXME :: It is not clear what should happen in a scenario in which there is a
potential retainer that has excessive `useds`. Perhaps L1 channels should be
filtered on `useds` being known or squashed.

## Endpoints

The base url should indicate the version of peer protocol that is supported.

### No auth

#### Get info

Consumer gets info on conditions of channel with Adaptor.

```rust
struct InfoResponse {
    adaptor : VerificationKey,
    close_period: TimeDelta,
    fee: u64,
}
```

### With auth

In this version it is TBC whether or not auth is required. It could use simply
HTTP basic auth with

```
client-id: keytag, client-secret-key: signature
```

where the signature is formed from the Consumer key and the message

```rust
let message = "LET ME IN!".to_vec().concat(sub_vkey)
```

Invalid signatures need not be responded to. If the keytag exists in Adaptor's
database, then auth is permitted.

#### Post squash

Consumer posts a squash. In particular, in the "initial handshake" Consumer
sends Adaptor null squash. Adaptor must not handle a cheque prior to receiving a
squash.

If Consumer wishes to recover state, they should submit a squash.

A squash response is 204 if everything is up-to-date. A squash response is 200
with instructions on submitting an up-to-date squash. Past a certain staleness
(for example, too many used cheques), the adaptor must not handle any more
cheques.

```rust
struct SquashRequest {
    squash : Squash,
}

struct SquashResponse {
    target : SquashBody,
    squash : Squash,
    unlockeds : Vec<Locked ...? TBD >,
}
```

On receiving a squash, Adaptor must:

1. Verify squash is well-formed.
1. Verify channel exists, has and `is_active`.
1. If there is no current receipt, make a new receipt with squash, continue
1. Else if the squash is not a valid bump from current squash and cheques,
   continue
1. Else upsert squash and drop cheques now squashed, continue

Adaptor should then respond with the appropriate response.

A valid bump is computed as follows:

1. The amount is greater or equal to current value
1. Squash

On receiving the 200 response, Consumer must verify:

1. Squash is well-formed.
1. Unlockeds are well-formed.
1. Target is result of squashing.

The final of these requires the following steps.

1. `target > squash.body` with the partial order endowed on squash bodies
1. All unlockeds have been squashed
1. `target.amount <= squash.body.amount + sum(unlockeds)`

Consumer may wish to verify that unlockeds that have timed out have indeed been
used. This is safety critical only if there is some out of band dependency
indicating whether or not a payment went through. FIXME :: this should be a
feature: Consumer set locked to be "wont squash".

On verification, Consumer should then form the new squash (ie sign it), and
repeat until they receive a 204. Note that there is a possibility of a cycle
here if something goes wrong. Steps should be taken to prevent an endless loop
or spamming.

#### Post quote

Consumer posts a quote for pay. That is, they ask Adaptor "how much would it
cost to pay the following payment request?". The payment request may be a
[Bolt 11]() invoice, or payment request details provided manually. Feature
support for manual will be incremental, starting with the minimum required for
successful payments.

The response consists of a `ChequeBody` Adaptor deems servicible, and
information on the fee. Note that Consumer should perform their own calculation
on the effective fee of the quote.

There are many reasons a quote may fail, such as no route funds or insufficient
available funds.

```rust
Enum QuoteRequest {
    Manual(PaymentRequest),
    Bolt11(String, Option<Amount>),
}

struct QuoteResponse {
    index: u64,
    amount: ChequeBody,
    timeout_delta: TimeDelta,
}
```

On receiving a quote request Adaptor must verify that it is servicible:

1. Verify channel exists, `is_active`, and has a retainer.
1. Verify channel has capacity: amount is available, and unsquashed cheques is
   less than maximum.

Adaptor should do this with a rough fee estimate prior to forwarding the request
to BLN. Adaptor must then verify that a payment is still servicible with the
actual fee (technically still an estimate).

If the BLN route is found, and payment and fee is servicible, adaptor should
construct a `QuoteResponse`. Note that the amount is calculated from the BLN
response, known exchange rates and Adaptor's own fee. The timeout is an estmate
of the BLN timeout (converted from Bitcoin blocks to time delta in) plus
Adaptor's own timeout requirement. The index must:

- not be squahed
- not be the index of an existing locked or unlocked cheque
- not be used in current retainer

It should be the next unused integer, and can replace an existing quote if that
is deemed stale.

On receiveing a quote response, Consumer must:

1. Verify the amount and lock align with the payment request
1. Verify the time delta is acceptable and compute the timeout.

Note that although Consumer computes timeout, there should be little confidence
that a cheque will be servicible if there is a long delay between quote and pay.

##### Post pay

Consumer accepts a quote and wishes to pay.

```rust
struct PayRequest = {
    quote : QuoteRequest,
    cheque : Cheque,
}

struct PayResponse = SquashResponse
```

We need not assume that the pay follows a quote. However, for Consumer to
attempt a pay without quote seems imprudent in that it is unlikely Consumer will
correctly estimate a servicible cheque, that isn't also over generous in amount
and timeout.

On receiving a pay request Adaptor must verify (probably again) that it is
servicible as in a quote. Moreover, Adaptor must:

1. Locked is well-formed.
1. Verify locked index is not squashed, and in not already in use.
1. Insert locked into cheques.
1. Route the payment with computed fee limit set and timeout set.

If BLN responds successfully and quickly:

1. Verify secret is correct.
1. Bump the `Locked` to an `Unlocked` cheque.

Adaptor should then respond to Consumer with the squash response. Consumer
should proceed as per a squash.

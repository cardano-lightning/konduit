---
title: "Konduit: Core Processes"
author: waalge
---

# Intro

This document details the processes of Konduit.

Konduit allows Cardano users, hereby referred to as Consumers, to pay Merchants
over the Bitcoin Lightning Network (BLN). BLN is an existing protocol, with
existing adoption. Merchants who accept BLN payments will accept payments via
Konduit with no further action required. In fact, they won't even know Consumer
is paying via Konduit. Konduit is enabled by Adaptors that route payments from
Cardano to Bitcoin.

## Principle Process

The Principle Process of Konduit is as follows:

- Merchant generates a BLN invoice
- Consumer scan a BLN invoice
- Consumer requests quotes (ie conditions of servicing the invoice including
  cost) from their partnered Adaptors
- Consumer selects a quote, and commits to pay Adaptor
- Adaptor commits to pay merchant via BLN
- Adaptor provides proof of payment
- Consumer finalizes the payment

On the happy path, the payment section resolves very quickly, often < 1s. On the
unhappy path where the payment fails to fully resolve, the protocol extends the
guarantees of BLN that all users will eventually claim funds rightfully theirs.

All other processes described are to facilitate the Principle Process.

## Lighting Network Topology

Fundamentally, lightning networks consist of many two party channels. What
enables it to act as a network, is that a participant with two channels may
commit to pay one on the condition of commitment to being paid on another.
Cryptography magic makes these commitments bindings. There are additional
components to, for example, finding routes from one member of the network to
another.

Konduit channels provide the same guarantee: commitment to pay, realized on
proof of onward payment made. A key difference is that Konduit channels are
_unidirectional_: Consumer can pay via the channel, but not be paid. This
limitation greatly simplifies other aspects of the protocol.

## Routing payments

```d2
direction: right
vars: {
d2-config: {
layout-engine: elk
pad: 5
}
}
Consumers: { icon: https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/smartphone.svg }
Adaptors: { icon: https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/server.svg }
BLN: { icon: https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/bitcoin.svg }
Merchants: { icon: https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/shopping-cart.svg }

Consumers -> Adaptors: "Konduit\nChannel"
Adaptors -> BLN: "BLN\nChannel"
BLN -> Merchants: "BLN\nChannel"
```

# Channel lifecycle

Konduit Channels are **retained** by UTXO at a Konduit script address. The UTXO
both locks funds and maintains the Channel's on-chain state. We view the UTXO as
a state machine:

```d2
direction: right
vars: {
  d2-config: {
    layout-engine: elk
    pad: 5
  }
}

o: ""{
  shape: circle
  width: 20; height: 20
}

x: ""{
  shape: circle
  width: 20; height: 20
}

# Transitions
o -> Opened: open

Opened -> Opened: add, sub
Opened -> Closed: close
Closed -> Responded: respond
Closed -> x: elapse
Responded -> Responded: unlock, expire
Responded -> x: end
```

We refer to the nodes as **stages** and arrows as **steps**.

Consumer can `open` a channel with Adaptor. The `open` locks Consumer funds on
the L1. These funds underwrite the payments Adaptor makes on the behalf of
Consumer on BLN.

On an `Opened` channel:

- Adaptor can `sub` owed funds
- Consumer can `add` more funds

Consumer can `close` the channel. This indicates to Adaptor they no longer wish
to use the channel.

Adaptor `respond`s to a `Closed` channel. They are permitted to pull all owed
funds, while leaving locked all unresolved funds. Unresolved funds correspond to
commitments made but for which the outcome is unknown.

On an `Responded` channel:

- Adaptor can `unlock` funds if a commitment payment goes through
- Consumer can `expire` funds if the commitment expires. They reclaim all funds
  not still locked.
- In the case there are no unexpired commitments remaining, Consumer `end`s the
  channel. They reclaim all remaining funds.

If the case Adaptor fails to `respond` within a given time window, then Consumer
`elapses` the channel. Consumer reclaims all funds in the channel.

# Adaptor

Adaptor may refer to either the person, or entity, or to the service the person
or entity maintains. The core part of the service is a server that repsonds to
requests from Consumers.

## Adaptor Init

To init a service, the following must be configured:

- [ ] Running of a BLN node, or access to a running node, together with open
      channels with spendable liquidity
- [ ] Access to pricefeed data
- [ ] Access to exchange services, specifically from Ada to BTC.
- [ ] Funding a cardano fuel wallet. This simply pays for txs fees and provides
      collateral
- [ ] Cardano Tx creating a reference script for Konduit

Once these have been established, a Konduit adaptor server can be configured and
run.

## Adaptor Ops

During operation adaptor can carry out different operations.

### Liquidity

- pull: Submit Cardano transaction(s) removing owed funds from channel UTXOs.
  The output address may be the Adaptors fuel wallet, an exhange address, or
  otherwise.
- push: Increase spendable liquidity by BLN node. Either increasing liquidity in
  existing channels or opening additional channels.
- mv: pull and push via an exchange, or other service provider.

### Consumers

- ammend: ammend channel state. In particular, can mark as `is_active = false`
  to discontinue service of the channel.

### Db

- prune: drop all entries pertaining to channels where no further fund will be
  pulled. In particular, channels without backing, channels that are responded
  with no unexpired.
- stash: backup the db

# Consumer

Consumer may refer to either the person, or entity or the application driving
their part of the Konduit protcol.

## Open

Before `open`ning a channel, Consumer needs fund on cardano in order to lock
funds up to back payements and pay transaction fees.

An `open` locks funds at a Konduit script address. A prioiri all locked funds
belong to, and are claimable by, Consumer.

More precisely a single transaction may open multiple channels, and can be done
in conjuction with other steps.

## Opened

While channel is opened, all funds Adaptor cannot prove they are owed belong to,
and are claimable by, the Consumer.

## Adaptor Rm

###

## Adaptor Service lifecycle

Adaptor simply inits their service, manages while they wish to provide service,
and then rm the service.

```d2
direction: right
vars: {
  d2-config: {
    layout-engine: elk
    pad: 5
  }
}

# States
o: "" { shape: circle; width: 15; height: 15 }
x: "" { shape: circle; width: 15; height: 15 }

# Minimal Lifecycle
o -> Service: init
Service -> Service: manage
Service -> x: rm
```

## Adaptor init

```d2
direction: right
vars: {
  d2-config: {
    layout-engine: elk
    pad: 5
  }
}

Adaptor: {
  icon: https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/server.svg
}
Cardano: {
  icon: https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/layers.svg
}
BLN: {
  icon: https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/bitcoin.svg
}

shape: sequence_diagram
Adaptor -> Cardano: fuel\nwallet
Adaptor -> Cardano: reference\nscript
Adaptor -> BLN: open\nchannel

Adaptor."Configure &\nrun server"
```

## Adaptor manage

```d2
direction: right
vars: {
  d2-config: {
    layout-engine: elk
    pad: 5
  }
}

# Define Actors with Icons
Adaptor: {
  icon: https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/server.svg
}
Cardano: {
  icon: https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/layers.svg
}
BLN: {
  icon: https://raw.githubusercontent.com/lucide-icons/lucide/main/icons/bitcoin.svg
}

shape: sequence_diagram
Adaptor -> Cardano: fuel\nwallet
Adaptor -> Cardano: reference\nscript
Adaptor -> BLN: open\nchannel

Adaptor."Configure &\nrun server"
```

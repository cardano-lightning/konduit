---
title: Roles
---

The following are the different users to which the document refers.

# Consumer

"Consumer" is the principle target of the application and akin as to the user of
a typical traditional application. In Konduit, Consumer interfaces via a mobile
application and is able to pay for goods and services in a manner
indistinguishable from users of Bitcoin Lightning. Consumer can manage their
channels, and review their usage.

The primary target for Consumer is anyone currently holding Ada, and has some
participation in the network. They have a mobile running a popular OS, and that
supports third party applications. On the product adoption curve, they will be
innovators or early adopters, with some understanding of crypto currencies but
are not necessarily technical. The secondary target is anyone using crypto and
wants a point of sale solution with their crypto.

# Adaptor

"Adaptor" runs (some of) the "back-end services" of Konduit, along side a BLN
node. All Cardano based channels consist of one consumer and one node operator.
A node operator accepts payment in the Cardano channel and forwards payment on
to BLN, in exchange for a fee.

The primary target Adaptor is either:

- An SPO, or similar: they run Cardano related infrastructure
- A BLN node operator with some inclination to support Cardano

Adaptor understands crypto currencies and are technically capable with respect
to infrastructure. They can confidently plumb docker images.

A note on the choice of name "Adaptor". Rejected alternatives: "Bridge" has
baggage; "Connector" used in other contexts eg "Cardano connector"; "Router" is
used in BLN to mean a similar thing, but clearly not managing channels between
chains. The spelling is the British English variant - this despite spelling
default to American English. The key reason is that "Adaptor" starts with "Ada".

# Dev

Konduit does not intend to be a complete, and standalone piece of software. The
project output must also cater to developers to inspire, contribute, maintain,
and extend the project.

The primary target dev is a dev with some exposure to the Cardano ecosystem.
They may have limited or deep experience of the mechanics and components of
Cardano. A secondary target dev is a dev with experience in the general crypto
ecosystem, not including Cardano.

# Marketer

"Marketers" is anyone wishing to understand the value of Konduit, but is neither
a direct user of the software, nor a developer or deeply technically inclined.
The project output shall include material to address their wants, such as
supporting documentation and usage metrics.

The primary target marketer are those in BD in a Cardano focused entity, such as
CF.

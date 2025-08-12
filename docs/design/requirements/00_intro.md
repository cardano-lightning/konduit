---
title: "Konduit Requirements"
authors:
  - "@waalge"
created-at: 2025-08-01
status: draft
---

## Intro

This document describes the requirements of Konduit. Konduit is only the working
title of the project. Konduit brings the capability of Bitcoin Lightning to
Cardano users.

This document is intended to be a resource for all stakeholders: business,
product, dev, and user. It is a public document.

### Purpose

Principal product purpose (PPP): Konduit shall allow ada holders to have an
indistinguishable experience to those of BLN users with respect to merchant
payment.

A fundamental high level business goal: drive user adoption of the Cardano
Blockchain via compelling utility. Bridge to other crypto ecosystem. Konduit is
one project aimed at addressing this goal.

The Bitcoin Lightning Network (BLN) has shown remarkable utility as e2e payment
solution. A BLN user can a pay a merchant via the scanning of a QR code and
consenting to a payment. The process has a UX similar to that of other payment
solutions such as contactless in terms of UX/UI (ease of use, time to
completion).

### Scope

The scope of the project includes all that is necessary to achieve the PPP,
while also being safe and sufficiently ergonomic. Ergonomic, for users,
maintainers, and future developers. This document aims to make concrete the
"safe and sufficiently ergonomic" behavior.

The project is also a show piece of the potential integrations between Cardano
and Bitcoin. The project's success is in part judged on the (positive) attention
it receives in the wider ecosystem, and whether it is credited with catalysing
other initiatives which themselves bring compelling utility to Cardano users. It
needs to also demonstrate financial sustainability...

### Jargon

See the [glossary](../../glossary.md) (TBC)

- BLN - Bitcoin Lightning Network
- BD - Business Development
- CF - Cardano foundation
- SPO - Stake Pool Operator
- L1 - Cardano Ledger
- OOB - Out Of Band
- tx - Blockchain transaction

### ToC

1. Overview:
   1. [Roles](./11_roles.md)
   1. [Scenarios](./12_scenarios.md)
   1. [Assumptions and dependencies](./13_assumptions.md)
1. [Architecture](./20_architecture.md) TODO!
1. Function
   1. [App](./31_app.md) TODO!
   1. L1 Liaison
   1. Konduit Operator Node
   1. Price Feed

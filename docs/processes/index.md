---
title: "Konduit Protocol: Technical Briefing for Legal & Finance"
author: "@waalge"
---

## I - Introduction

This document provides a comprehensive overview of the Konduit protocol,
specifically addressing the requirements for internal risk assessment and
external legal opinion.

Konduit is a Cardano-backed extension of the Bitcoin Lightning Network (BLN). It
allows Consumers to open and maintain payment channels on Cardano that can route
payments to Merchants via the BLN.

Pay is the principal process of Konduit. BLN is an existing e2e payment
solution. It enables users to pay via generating and scanning QR codes. It
boasts near instant settlement and very low fees. Konduit Consumers access the
same ecosystem with the same benefits, without leaving Cardano.

### I.A - Key Actors

1. **Consumer**: The party locking assets on Cardano to facilitate payments.
1. **Adaptor**: A specialized channel participant existing on both Cardano and BLN.
  They facilitate the cross-layer exchange by accepting payments on Cardano
  backed by atomic proof of routing on BLN.
1. **Merchant**: The ultimate recipient of value, accepting payment via BLN. Note
  that the protocol is designed such that the Merchant interacts strictly with
  the BLN and requires no specific knowledge of Konduit.

### I.B - Core Concept

Konduit operates as a non-custodial coordination layer. It uses the Cardano UTXO
ledger to enforce payment obligations, ensuring that value only moves when
cryptographic "Secrets" are revealed, equivalent to the atomicity of BLN's
Hashed Timelock Contracts (HTLCs).

The protocol is asset-agnostic, supporting any Cardano native asset as the
primary unit of account.

## II - Protocol

### II.A - Channel lifecycle

A Konduit channel lifecycle is manifest in the state of a UTXO at a Konduit
script address on Cardano, acting as a state maching. Transitions are **steps**
between **"stages** and correspond to a spend transaction. Note that a single
transaction may step many channels. Only Consumer and Adaptor can step a
channel. All steps are unilateral and subject to script logic, which treats each
channel independently.

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

### II.B - Pay

Pay is the principal process of Konduit.

#### II.B.1 - Standard path

The following outlines the standard sequence involved in a pay.

```d2
vars {
    d2-config {
        layout-engine: elk
    }
}

shape: sequence_diagram

Consumer: Consumer
Adaptor: Adaptor
BLN: BLN
Merchant: Merchant

Merchant -> Consumer: invoice
Consumer -> Adaptor: quote?
Adaptor -> BLN: route?
BLN -> Adaptor: routes
Adaptor -> Consumer: quote
Consumer -> Adaptor: cheque
Adaptor -> BLN: htlc
BLN -> Merchant: htlc
Merchant -> BLN: secret
BLN -> Adaptor: secret
Adaptor -> Consumer: secret
Consumer -> Adaptor: squash
```

#### II.B.2 - Deviations

The following flowchart goes through the possible deviations in the standard
sequence of a pay.

```d2
direction: down

vars {
  d2-config {
    layout-engine: elk
  }
}

# Decision Nodes
D0: "[Consumer]\nInvoice Valid?" {shape: diamond}
D1: "[Consumer]\nAdaptor Online?" {shape: diamond}
D2: "[Adaptor]\nServe Quote?" {shape: diamond}
D3: "[Adaptor]\nRoute Inbound Failed?" {shape: diamond}
D4: "[Consumer]\nAccept Quote?" {shape: diamond}
D5: "[Adaptor]\nServe Pay?" {shape: diamond}
D6: "[Merchant]\nRoute Outbound Failed?" {shape: diamond}
D7: "[Merchant]\nSecret Revealed?" {shape: diamond}
D8: "[Adaptor]\nNo Squash?" {shape: diamond}

F1: "Fail: pre-commitment" {style.fill: "#f2dede"}
F2: "Fail: post-commitment.\nMerchant unpaid" {style.fill: "#f2dede"}
F3: "Fail: post-commitment.\nMerchant paid" {style.fill: "#f2dede"}
S1: "Ok" {style.fill: "#dff0d8"}
S2: "Ok via L1 tx" {style.fill: "#dff0d8"}

D0 -> F1: "No (Invalid)"
D0 -> D1: "Yes"

D1 -> F1: "No (Offline)"
D1 -> D2: "Yes"

D2 -> F1: "No (Refusal)"
D2 -> D3: "Yes"

D3 -> F1: "Yes"
D3 -> D4: "No (Provide Quote)"

D4 -> F1: "No (User Exit)"
D4 -> D5: "Yes"

D5 -> F1: "No (Refusal)"
D5 -> D6: "Yes"

D6 -> F2: "Yes (Await Timeout)"
D6 -> D7: "No"

D7 -> F3: "No (Await Timeout)"
D7 -> D8: "Yes (Secret Revealed)"

D8 -> S2: "Yes (Force Submit)"
D8 -> S1: "No (Squash Success)"

```

## III - Matters legal & financial

### III.A - Non-Custodial Confirmation

Konduit is strictly non-custodial. The Adaptor provides routing services but
possesses no discretionary control over Consumer assets.

1. **Script-Governed Assets**: The source of truth for all funds is a Cardano script
  address (UTXO). Funds are held in a locked state and can only be released if
  the script's logic is satisfied.
1. **Cryptographic Enforcement**: The Adaptor cannot claim assets without presenting
  a Secret[^def:secret] (pre-image). This Secret is only obtainable if the Merchant has been
  paid on the BLN side, creating a hard link between delivery and payment.
1. **Exchange of commitments**: The consumer does not hand-over funds directly
  to the adaptor. It only provides a commitment (a.k.a a cheque[^def:cheque])
  that can later be exchanged for a new agreed state or used directly on-chain
  alongside the corresponding secret acting as a proof of payment.

### III.B - Finality of Settlement

The protocol ensures that engagement can be terminated by either party at any
time, with a definitive path to finality that requires no subjective
arbitration.

1. **The Invitation to Finalize**: A close step initiated by the Consumer is a formal
  invitation for final settlement. It does not permit the immediate claiming of
  funds but signals the start of the resolution phase.
1. **The Respond Mechanism**: Following a close, the Adaptor performs a respond step
  to report final provably owed funds as well as all "Pending Cheques"[^def:pending] — payments
  currently in transit where a Secret is not yet known but the payment has not
  expired.
1. **Deterministic Resolution**: Settlement is finalized strictly by cryptographic
  evidence. Everything not provably owed to the Adaptor (via a Secret or a
  signed Squash) is returned to the Consumer. There is no contestation period;
  the math governs the distribution.

### III.C - Liability and Recourse

Legal and Finance require certainty regarding "worst-case" scenarios, such as
participant inactivity.

1. **Adaptor Inactivity**: If the Adaptor fails to respond to a closure within the
  defined close_period, the Consumer has the unilateral right to elapse the
  channel, reclaiming 100% of the remaining balance.
1. **Consumer Inactivity**: The Consumer is not required to be online for the Adaptor
  to secure their funds. The Adaptor can "sub"[^def:sub-add] (redeem) funds on-chain using
  individual Unlocked Cheques[^def:unlocked] if the Consumer is unavailable to sign a
  cumulative Squash[^def:squash].
1. **Unilateral Exit**: All state transitions on-chain are unilateral. Neither party
  can "trap" the other in an indeterminate state.
1. **Blockchain Assumptions**: The protocol relies on standard blockchain liveness
  and security assumptions. Timeout parameters are fixed a priori, and
  participants consent to these durations—and their associated risks—upon
  channel opening.

### III.D - Clarity on Proof

The "Receipt" is the primary legal and technical evidence of a completed
transaction.

1. **Atomic Proof**: An Unlocked Cheque (Cheque \+ Secret) serves as self-contained,
  atomic proof that a transaction was successfully routed. This proof is a
  functional key that the script must accept during settlement.
1. **Cumulative Debt Compression**: The "Squash" acts as a running ledger of debt. By
  signing a Squash, the Consumer acknowledges the total amount owed, allowing
  the Adaptor to optimize L1 costs by postponing redemption.

### III.E - Regulatory Categorization

From a regulatory standpoint, Konduit functions as a neutral state-management
protocol.

1. **No Intermediary Discretion**: The Adaptor acts as a relay that is
  programmatically reimbursed.
1. **Liquidity Commitment Risk**: The Adaptor bears the financial risks associated
  with the liquidity lockup required for BLN routing. This includes potential
  exposure to relative currency volatility and time-unit mismatches in expiry
  windows between layers.

## IV - Glossary

[^def:cheque]: **Cheque**: A signed off-chain payload containing HTLC commitment details.
[^def:unlocked]: **Unlocked Cheque**: A cheque combined with its corresponding Secret, constituting atomic proof of payment.
[^def:pending]: **Pending Cheque**: A payment for which the timeout has not passed and the Secret is unknown.
[^def:squash]: **Squash**: A cryptographically signed aggregate of processed payments. It includes an "exclude" list—an implementation detail used to prevent unresolved/pending cheques from becoming a bottleneck in the settlement flow.
[^def:secret]: **Secret**: The cryptographic pre-image used to unlock an HTLC on BLN and subsequently claim funds from the Konduit script.
[^def:sub-add]: **Sub/Add**: Primary L1 operations used to adjust the collateralized balance. A sub operation realizes accrued debt to date without resetting the Squash index.

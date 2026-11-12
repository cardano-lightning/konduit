# Konduit

> a platform for htlc compat outbound routing

## Lightning

Lightning networks consist of participants (nodes) joined by two party channels (edges).

A channel backs payments from one participant of the channel to the other.
The backing corresponds to a state on some L1 that guarantees future redemption of owed funds.
When receiving a commitment to pay (ie something redeemable) from a payer, the payee must verify that amount being committed is **backed**.
A channel may be bi or unidirectional: whether commitments can go in both directions.

Commitments come in two sorts: conditional and unconditional.
Typically, commitments are first made conditionally and then revised to unconditional.
If payer declines to make the revision, then the conditional commitment, together with proof the condition was met, is used to redeem funds from backing.
Typically, commitments **timeout** so if the condition is not met by some deadline, then the commitment is forever unredeemable.

Payments on a lightning network between two participants not sharing a channel require a route along edges.
Each hop on the route consists of the two stage commitment conversation described above.
The condition is proof of payment before some deadline.

Direction is relative to a participant: a payment arrives **inbound** and leaves **outbound**.
We may describe the payer as upstream of the payee, and the payee as downstream of the payer.
Each intermediary in the route must verify that the inbound commitment is backed and covers the outbound commitment.
A consequence is that commitment deadlines are longer in upstream than downstream.

## HTLC compat

In konduit, we call commitments **cheques** because they look like cheques: cheque number, amount, and date.
They also bear the condition, which we call a **lock**.
Proof of payment is the presentation of the associated **secret**.

A lock is sha256 of its secret. Both are 32 bytes.
Bitcoin Lightning calls the pair the hash and the preimage of an HTLC.

In a standard HTLC route, a single lock is used across all commitments.
Thus, the revealing of the secret is the presentation of proof of payment.
A route participant learns this from their downstream channel partner either
by an L2 (aka offchain) message, or by seeing the claim tx on the L1 claiming the associated funds.

Denomination (aka backing currency) and the clock are each hop's own.
These do not have to be identical across the route: each node is responsible for verifying
that the inbound commitment "covers" the outbound commitment to their satisfaction.

Konduit's inbound hops are backed by Cardano native assets. Konduit channels are unidirectional, mono-asset.
The clock is as understood by plutus and the cardano network: Posix.

BLN channels are bitcoin backed and the clock is Bitcoin Block time.
The router must ensure that inbound covers outbound assessing possible risks.

## Konduit as a platform

Konduit handles the inbound verification, and liaises with "outbound rails".
An outbound rail speaks to which ever network the payer wishes to take.
Every rail is outbound, so the bare term is unambiguous below.

The L1 mechanics of konduit are fixed: commitments are locked and unlocked cheques (and squashes) backed by wellformed utxos governed by the validator.
Inbound is therefore always a konduit channel, and inbound verification is konduit's alone.
Outbound belongs to a rail: its network, its addressing, and its settlement are the rail's own.
Konduit is outbound agnostic.

## Interfaces

Two interfaces, and only one of them is open.

Payer to konduit is given: invoice, an inbound conditional commitment, a returned secret, squash.
This is the existing cheque protocol, taken as fixed, and set out below for the correspondence with the rail interface.

Konduit to a generic outbound rail is the interface to specify.
It is the minimal set of demands each side makes of the other.
Each demand earns its place by holding for every outbound network.

### Consumer API

This is the external surface: HTTP, content negotiated.
The rail trait below is internal and in process. Konduit maps between the two, and that mapping is the only place the two meet.
Authenticated per channel by keytag. The rail is in the path, so Consumer picks it per payment.
Channel operations are the same shape under a reserved `konduit` name, since they hold irrespective of outbound rail.

| route                      | Consumer sends             | konduit returns                                              |
| -------------------------- | -------------------------- | ------------------------------------------------------------ |
| `GET /info`                | -                          | asset class, configured rails, konduit's fee and time margin |
| `POST /pay/{rail}/quote`   | amount, instruction        | fee and timeout: the rail's, plus konduit's                  |
| `POST /pay/{rail}/pay`     | locked cheque, instruction | the secret, pending, or problem details                      |
| `GET /pay/konduit/receipt` | -                          | latest receipt                                               |
| `POST /pay/konduit/squash` | signed squash              | `SquashStatus`                                               |

Quote and pay are the rail's operations seen from the other side, with konduit's fee and time margin added on the way out and stripped on the way in.
Reveal is absent: konduit needs it to build its own receipt, Consumer does not. Consumer resolves the inbound leg alone, which is squash.

1. **Cover.** The cheque's amount covers the quoted amount and konduit's fee; its timeout exceeds the quoted timeout and konduit's margin. Short of that the commitment is not made, and no rail component is called.
2. **Lock.** The cheque bears the payee's lock. Konduit takes the lock from the cheque, never from the instruction, which it does not parse. A lock the instruction disagrees with is the rail's `FailedPre`.
3. **Resolution.** A secret is returned and Consumer squashes. Pending is polled on squash status, where the secret arrives as an unlocked cheque in the proposal. A failure leaves the cheque to elapse.

An inbound routing peer uses the same shape. It differs in authentication and in holding its own channel, not in the operations.

### Rail's own

A rail owns its network: the route, the hop count, its own fees, its retries, its settlement.
Do whatever here.
The instruction is part of that: it reaches konduit as bytes, and a rail parses and validates it, since whether some string is a payable BLN invoice is a BLN question.
Konduit asks only that the outcome be reported in the terms below.

### Konduit role

Konduit verifies inbound; a failure here calls no rail component.

1. **Quote**: forward the request with its instruction; add fee and time margin to the reply.
2. **Commit**: verify the inbound commitment is claimable upto amount, lock, timeout; forward these net of fee and time margin, with the instruction.
3. **Reveal**: fetch downstream secrets for resolution (squash) and redemption. Only pending and post-commitment failures need a reveal.

A commit the rail declines leaves the inbound cheque live until its timeout. Consumer carries that liquidity cost on a failed payment.

### Rail role

1. **Quote**: establish downstream fee and margin.
2. **Commit**: ensure the outbound commitment has (probably) cost no more than the amount; (probably) timeout earlier than the timeout; and an (effectively) equal lock.
   The lock is the idempotency key: repeated commits under one lock pay at most once.
   Report one of: **success**; **failed pre-commitment**, nothing committed outbound; **pending**, committed and unresolved; **failed post-commitment**. Respond pending once unresolved beyond an Adaptor configured timeout.
   "Probably" because currency and clock belong to the outbound channel and may vary from konduit's on Cardano. We mean with very very high probability, the tolerance being Adaptor configuration.
   "Effectively" because the convention may change. What matters is that a downstream redemption or resolution requires a reveal of the secret.
3. **Reveal**: handle delayed resolution. Pull is required; push is optionally configured. A rail caches a revealed secret at least as long as the corresponding commitment timeout.

### Denomination

A konduit instance supports a single asset class.
Every amount across the boundaries is a number in that asset class, unless declared otherwise.

Fx introduces issues around fluctuating currency movements.
A rail is configured to fetch its relevant exchange rate from that asset to whatever is required.

Each rail receives a converter for its configured instance/outbound asset pair.
The converter supports both directions: instance amount to outbound amount,
and outbound amount to instance amount. Each conversion takes and returns a
`u64` amount and is fallible (`Result<u64, ConversionError>`); for example,
conversion fails when no usable rate data is available or the result cannot
be represented. Since amounts are integral, converting in both directions
rounds and is not guaranteed to recover the exact starting amount. Rails
must account for this rounding when checking that an outbound commitment is
covered by its inbound amount.

In future, perhaps a full fx service, configured to convert any to any.

### Rail choice

Adaptor picks which rails it supports and configures each; Consumer picks one per payment from those.
Selection is structural, by path, eg `/pay/cln/commit`. `konduit` is reserved for what no rail owns.
`AdaptorInfo` carries the asset class and the configured rails, so Consumer sees the set it picks from.
One rail per name: a rail manages any multi-instance notion internally.

### Trait

A rail is an in process trait impl, compiled in and selected by configuration at startup.
Whether it then speaks to a remote node over its own transport is inside the hole above.

Konduit supplies the inbound constraints, the rail owns everything after them, so constraints come first in argument order.
Each operation takes its own rail defined instruction type: a rail quotes on less than it commits on, and konduit names neither.
An instruction that fails to deserialize is a `FailedPre` raised by the erasing layer, so no rail hand rolls it.

```rust
pub struct Constraints {
    pub amount: u64,        // in the instance asset, net of konduit's fee
    pub timeout: Duration,  // net of konduit's time margin
    pub lock: Lock,
}

pub struct Quoted {
    pub fee: u64,
    pub timeout: Duration,
}

pub enum Outcome {
    Success(Secret),
    Pending,
    FailedPre(PreProblem),
    FailedPost(PostProblem),
}

pub enum Resolution {
    Success(Secret),
    Pending,
    Unknown,
    Failed(PostProblem),
}

pub trait Rail {
    /// Identifier, and the config key. Konduit serves it at `/pay/{NAME}/`.
    const NAME: &'static str;
    type QuoteInstruction: for<'de> Deserialize<'de>;
    type CommitInstruction: for<'de> Deserialize<'de>;

    async fn quote(&self, amount: u64, instruction: Self::QuoteInstruction) -> Result<Quoted, QuoteProblem>;
    async fn commit(&self, inbound: Constraints, instruction: Self::CommitInstruction) -> Outcome;
    async fn reveal(&self, lock: &Lock) -> Resolution;
}
```

Commit has no error: determinacy belongs in `Outcome`, not in an arm of `Result`.
An error read as a non-payment would chase a payment that was never sent, and encumber the consumer for nothing.
A rail internal fault therefore reports `Pending`, which is the only honest answer when it does not know.
Quote keeps an error because a failed quote commits nothing.

`Resolution` is narrower than `Outcome`: nothing fails before sending once sent.
`Unknown` answers a lock the rail never committed, or one whose cache has since expired.
Konduit reads `Unknown` past the commitment timeout as failed, and before it as a rail fault.

Two associated types and `async fn` are not dyn compatible, so the registry holds an erasing layer:
an object safe trait over bytes and boxed futures, which deserializes each instruction and raises `bad-instruction` itself.

### Problem

The enums are the trait's, and carry no HTTP.
Slug and status are the boundary's mapping, applied by konduit as a problem leaves over HTTP, per [ADR 10](./docs/adrs/10-problem-details.md). Slugs are stable across rails.
The sets are distinct, so they are separate types: what blocks a quote also blocks a commit before anything is sent, and what fails after is of another kind entirely.

`QuoteProblem`, also every way a commit fails before sending:

| slug                  | status | meaning                                                        |
| --------------------- | ------ | -------------------------------------------------------------- |
| `bad-instruction`     | 400    | instruction failed to deserialize; raised by the erasing layer |
| `unroutable`          | 422    | instruction parsed, no route to it                             |
| `amount-out-of-range` | 422    | amount outside what the network carries                        |
| `rail-unavailable`    | 503    | the rail's own backend is unreachable                          |
| `fx-unavailable`      | 503    | no rate for the instance asset                                 |

`PreProblem` is a `QuoteProblem` or `insufficient-margin` (422): the constraints cannot cover the rail's own margin.
Commit alone raises it, since quote is what proposes the margin.

`PostProblem`:

| slug                | status | meaning                                                   |
| ------------------- | ------ | --------------------------------------------------------- |
| `outbound-rejected` | 502    | the outbound network refused or failed the payment        |
| `outbound-timeout`  | 504    | the outbound commitment timed out unresolved; reveal only |

## Decided

- **Being the ultimate payee gets no special treatment.** Fee and time margin are added just the same,
  since they pay for the inbound verification konduit does either way, and that work does not shrink
  because the route ends here. So there is no terminal discount to implement, and no edge case to
  accommodate: a direct pay is the ordinary path with no onward rail, and `/pay/konduit/pay` is its home.
  What remains true is that the path can only carry the decision while Consumer knows it is paying
  konduit — where the instruction is encrypted, or otherwise unparsable until a rail has it, terminality
  is not visible at request time.
- **Lock alignment is a rail's question, not konduit's.** Konduit takes the lock from the cheque and
  compares it to nothing. Whether an instruction names something that disagrees with that lock is a fact
  about the rail's own network — only the rail can read the instruction at all — so it is the rail's
  `FailedPre`.

## Task

`wire/commit.rs` is already this shape for one rail, with `Outbound.key: VerifyingKey` the one place a rail's addressing has reached konduit.

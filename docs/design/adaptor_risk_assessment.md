---
title: Adaptor Risk Assessment
version: 0
date: 2026-01-21
status: draft
---

# Executive Summary

This report identifies and evaluates the risks associated **Adaptor**'s role.
TODO!!

---

# Context & Scope

Adaptor facilitates payments based on the belief that funds owed can be
successfully redeemed. While the design of the **Konduit** protocol provides a
basis for this belief, it does not render the Adaptor’s position risk-free.
Adaptor must remain clear-eyed regarding their risk.

- **System Description:** Adaptor enables Cardano ADA holders to pay Bitcoin
  Lightning invoices using the Konduit protocol.
- **Scope:** Security of the Konduit Plutus scripts, liquidity management on
  BLN, and operational integrity of the Adaptor node.
- **Risk Appetite:** Principal loss only in black swan scenario; control over
  max losses in the face of market volatility.

The assessment follows the structure of [ISO 31000:2018][iso-31000-preview],
section 6.4:

- **Identify:** The events that are root causes and catalysts.
- **Analyse:** The impacts of these events.
- **Evaluate:** The prioritization of these risks against business thresholds.

[iso-31000-preview]:
  https://cdn.standards.iteh.ai/samples/65694/60673072317a4b96bd36efb910b68926/ISO-31000-2018.pdf

# Context and Scope

## System Description

Adaptor is a cross-chain payment service that enables Cardano ada holders to pay
Bitcoin Lightning invoices and other payment requests. Konduit, the protocol
through which the service operates, is a trust-minimized framework with its
integrity kernel running on Cardano and utilizing Hashed Time-Locked Contracts
(HTLCs) on the Bitcoin Lightning Network.

## Assessment Scope

The boundary of this assessment includes:

- Cardano: The Konduit kernel, and the underlying network.
- Bitcoin: BLN routing, and liquidity management
- Internal operations: key management, and digital infrastructure
- External dependencie: Market volatility, gateways, _etc_.

## Risk Appetite

Our evaluations are based on the assumption that Adaptor has the following risk
appetite:

- Principal Protection: Zero tolerance for loss of principal, except in defined
  "Black Swan" systemic failures of the underlying L1 networks.
- Volatility Management: Strict control over maximum exposure during
  "vulnerability windows" to ensure market volatility does not result in
  realized loss.

However, this document should give Adaptor complete context to configure their
service to match their risk appetite.

## Assessment Methodology

This report follows the iterative process defined in ISO 31000:2018, Section 6.4
(Risk Assessment). We break the process into three distinct phases:

- Identify: Finding, recognizing, and describing the events that act as root
  causes or catalysts for uncertainty and/or loss.
- Analyze: Whether the risks are Agency-Driven vs. Structural their respective
  difficulty or likelihood, and potential of componding effects.
- Evaluate: Comparing analyzed results against risk appetite to prioritize
  treatment and determine the acceptability of residual risks.

---

## Indentify

We identify the risks in terms of source, event, and potential impact. Konduit
has two fundamental dependecies: [Cardano](#ID-C) and [Bitcoin](#ID-B). Exposure
to Bitcoin is predominantly via Bitcoin Lightning Network (BLN). Adaptor also
has [Internal Operations](#ID-I) and [External Dependencies](#ID-E) sources of
risk.

### Cardano {#ID-C}

#### Konduit Validator Exploit {#ID-C-KON}

**Source.** Logic bug or zero-day vulnerability in the Konduit Plutus validator
script.

**Event.** Bypass of authorization checks on locked Cardano assets. The assets
belong to Consumer and Adaptor, with the exact allocation depending on L2 state.

**Impacts.** Unauthorized drainage of ada; loss of principal.

#### Network Failure {#IC-C-NET}

**Source.** Critical vulnerability in the Ouroboros consensus mechanism, a
zero-day flaw in the Plutus interpreter, or a catastrophic failure of the
underlying cryptographic primitives or another dependency such as NTP.

**Event.** Loss of ledger integrity, multi-epoch chain rollbacks (deep forks),
or the invalidation of core security assumptions (e.g. double-spending).

**Impacts.** Absolute loss of asset value; total failure of channel redemption;
permanent service inoperability and systemic loss of principal.

#### Network Fork {#ID-C-FORK}

**Source.** Probabilistic finality inherent to the Cardano Ouroboros consensus
mechanism, where divergent chain "tines" exist before reaching settlement depth.

**Event.** A chain rollback where a block containing a Konduit transaction is
discarded in favor of a competing chain branch.

**Impacts.** Temporary state inconsistency; potential transaction invalidation;
extended "vulnerability window" for temporal divergence.

#### Settlement Latency {#ID-C-SET}

**Source.** There are several sources:

- Congestion: Excessive network-wide transaction volume exceeding block space
  capacity.
- Contention: UTXO-level competition where multiple transactions (e.g., Consumer
  Add vs. Adaptor Sub) attempt to spend the same input simultaneously.
- Adversarial Spamming: Targeted "Add-spamming" by a Consumer to intentionally
  conflict with Adaptor sub and respond steps.

**Event.** Delay in the inclusion of critical transactions (Sub or Respond)
within the required protocol timebounds (Timeout or Close Period respectively).

**Impacts.** Liquidity lock-up with potential loss of principal. For sub, loss
of principal is bound by Unlocked amount; for respond, loss of principal is
total owed.

### Bitcoin {#ID-B}

#### Route Discovery Failure {#ID-B-DIS}

**Source.** No route found to destination. Insufficient liquidity on available
route. Parameter constraints ie large payment amounts or strict fee/timelock
limits that prune all available paths.

**Event.** Adaptor is unable to find route to service Consumer's invoice.

**Impacts.** Effective service unavailability; loss of service fees. There is
zero principal risk, and no funds are committed.

#### Route Resolution Delay {#ID-B-DEL}

**Source.** Intermediary node latency, possibly adversarial.

**Event.** Adaptor receives secret long after the commitment.

**Impacts.** Extended exposure to market volatility ie extended "vulnerability
window", potentially in exploitable manner.

#### Route Resolution Timeout {#ID-B-TIME}

**Source.** Intermediary node failure; relative clock inconsistency; "Griefing"
or "Jamming" where attacker aims to exhaust channel slots (max 483 HTLCs) and or
liquidity.

**Event.** Commitment fails to settle (claim) in a timely manner. Also known as
a "Stuck HTLC".

**Impacts.** Capital Immobilization: Stuck funds cannot be used for other
payments until the timeout. This increases risk of
[route discovery failures](#ID-B-DIS).

#### Locked Bitcoin {#ID-B-LOCK}

**Source.** BLN requires bitcoin is locked during channel lifetime, including a
waiting period (typically 2016 blocks) on an uncooperative channel closure.

**Event.** Inability to liquidate or rebalance holdings due to protocol-enforced
lock-in periods.

**Impacts.** Market risk: prolonged exposure to bitcoin volatility.

#### Temporal divergence {#ID-B-DIV}

**Source.** Slow bitcoin block production, possibly caused by a sudden drop of
hashing power.

**Event.** Bitcoin time slows relative to posix time used by Cardano.

**Impacts.** Adaptor commitments are time denominated in Bitcoin blocks, yet
consumer commitments are in posix time. If temporal divergence is severe enough,
and there is [Route Resolution Delay](#ID-B-DEL), Adaptor can lose principal
associated to payment.

### Internal Operations {#ID-I}

#### Cryptographic Key Compromise {#ID-I-KEY}

**Source.** Unauthorized access to device(s) holding keys, for example because
of a misconfiguration.

**Event.** Attacker accesses Adaptor (signing) key

**Impacts.** Total loss of owed funds, and funds controlled by the signing key.

#### Data Loss {#ID-I-DATA}

**Source.** Database or device failure, possibly by a mishandling of resources.
Leading to a loss of receipts.

**Event.** The loss of the receipts.

**Impacts.** Loss of total loss of funds owed since previous redemption, or
latest backup. This can be orthogonal to [Service Degradation](#ID-I-SERV) if
state stored via a resync with chain or other backup.

#### Service Degradation {#ID-I-SERV}

**Source.** Internal API bottlenecks, failure of the Adaptor’s Cardano indexer,
or local Bitcoin node desynchronization.

**Event.** Adaptor service may remain "online" but is unable to monitor the
chain or submit transactions.

**Impacts.** Loss of business: Consumer finds alternative. Potential loss of
capital: in the case retainer closed and elapsed before intervention. Potential
exacerbating of the "vulnerability window," increasing the financial loss from
price drift.

### External Dependencies {#ID-E}

#### Market Volatility {#ID-E-VOL}

**Source.** Global macroeconomic shifts, regulatory announcements, or systemic
"black swan" events in the cryptocurrency sector.

**Event.** A fall in ada-btc pair that exceeds the anticipated threshold used by
Adaptor's pricing and risk models. Specifically during a "vulnerability window".

**Impacts.** Loss of funds not yet exchanged; particular funds commited but not
resolved (stuck).

#### Oracle Data Error {#ID-E-ORA}

**Source.** Outage, or otherwise inaccurate value the ada-btc pair.

**Event.** Stale or incorrect price data used to calculate cost of handling
payment.

**Impacts.** Exposure to loss to the Adaptor, and potentially exploitable
(attacker financial gain from arbritrage).

#### Exchange/Gateway Failure {#ID-E-GATE}

**Source.** Technical outages, service termination or other.

**Event.** Inability to execute rebalancing trades, query market depth (see
[Oracle Data Error](#ID-E-ORA)), access call options.

**Impacts.** Liquidity inbalance leading to [Service Degradation](#ID-I-SERV).
Greater exposure to [Market Volatility](#ID-E-VOL). Inability to hedge.

---

## Analyze {#AN}

### Method

We analyze the identified risks using two distinct **tracks**:

- **Agency-Driven.** Risks that require a sentient actor to initiate an event.
  These are analyzed based on the Technical Competency (expertise required) and
  the Resource Commitment (capital, hardware, or transaction fees) necessary to
  execute the exploit. Note: We assume that "cost" is relative; an actor may be
  motivated by profit, competitive sabotage, or purely disruptive "griefing."
- **Structural.** Risks inherent to the protocol architecture, blockchain
  dependencies, or market volatility. These are analyzed by estimating their
  Likelihood of occurrence within defined windows of operation.

For consider the risks _atomically_ in terms of:

- Impact Surface: A description of the potential impact (_eg_ principal drainage
  vs. liquidity lock-up) and the degree to which the state is recoverable.
- Exploitation Potential: If a structural risk manifests (_eg_ a network fork),
  can an attacker actively intervene to maximize Adaptor's loss or their own
  profit?

We then consider compounding effects: scenarios in which more than one risk is
at play.

### Register

| Ref ID                      | Risk Title          | Track      | Competency / Likelihood | Impact Surface       |
| :-------------------------- | :------------------ | :--------- | :---------------------- | :------------------- |
| **[AN-C-VAL](#AN-C-VAL)**   | Validator Exploit   | Agency     | High Competency         | Total Principal Loss |
| **[AN-C-SPAM](#AN-C-SPAM)** | Add-Spamming        | Agency     | Low Competency          | Liquidity Lock-up    |
| **[AN-B-DIV](#AN-B-DIV)**   | Temporal Divergence | Structural | Low Probability         | Principal Loss (Arb) |

### Atomic Analysis

We consider each risk identified, mostly on its own.

#### [AN-C-VAL] Konduit Validator Exploit {#AN-C-VAL}

[ID-C-VAL](#ID-C-VAL)

**Track.** Agency-Driven

**Analysis.** An attacker seeks to identify a flaw in the validator logic to
perform an "unauthorized spend". The attacker has "complete information": the
scripts and source code are public. While the codebase has received expert
scrutiny, it has not yet undergone a formal third-party audit. Severities range
from high "Global Drainage" (total bypass) to lower, for example,
"Partner-Level" exploits (bypassing logic but still requiring a participant
signature).

**Technical Competency.** High. Requires a sophisticated understanding of the
Aiken smart contract language and the Cardano EUTxO ledger model.

**Resource Commitment.** Low. Execution requires only standard L1 transaction
fees; the primary cost is the researcher's time.

**Exploitation Potential.** Absolute in high-severity cases.

**Impact Surface.** Critical. Potential for total drainage of Konduit TVL and
state is non-recoverable.

**Adversarial Maximization.** Extreme. In the high severity an attacker can
drain the entire TVL in a short space of time.

#### [AN-C-NET] Cardano Network Failure {#AN-C-NET}

[ID-C-NET](#ID-C-NET)

**Track.** Structural

**Analysis.** This risk refers to a systemic collapse of the underlying Cardano
ledger's security assumptions. Unlike a script-level bug, this is a "black swan"
event where the infrastructure itself fails. This could stem from a mathematical
flaw in the Ouroboros Praos/Genesis consensus, a critical bug in the
Haskell-based node implementation (e.g., the ledger state machine or Plutus
interpreter), or a compromise of foundational cryptographic primitives (e.g.,
Ed25519 or VRF). In such an event, the ledger may suffer deep forks, multi-epoch
rollbacks, or a complete halt.

**Likelihood.** Very Low. Ouroboros is a peer-reviewed, provably secure
consensus mechanism. A failure would require either a breakthrough in
cryptanalysis against established primitives or a catastrophic oversight in a
heavily audited, formal-methods-driven codebase.

Cardano has had a consistent uptime in the modern era. The closest there has
been to a network failure was the 2025/11/21 event caused by a transaction that
some node versions deemed valid, while others deemed invalid. Network was
technically operating within specification although in a degraded state.
Intevention by engineering teams and network operators was swift, and the
network recovered.

**Exploitation Potential.** High, although it depends on the manifestation of
the failure. It potentially provides a window for Agency-Driven exploitation. An
attacker could observe a network instability and attempt "double-spend" attacks
by broadcast-racing transactions on competing forks, or by manipulating L2 state
redemptions during the period of ledger inconsistency. That said, an exploit of
this sort is non-trivial and if Adaptor is also aware of network instability,
they can restrict service accordingly.

**Impact Surface.** Systemic. Total and potentially permanent failure of the
Adaptor service. Impact includes absolute loss of asset value on-chain.
Recoverability is entirely dependent on a coordinated social-layer intervention
(e.g., a hard fork or snapshot) by the broader Cardano community.

**Adversarial Maximization.** Possibly high. If the failure of the network is
such that Adaptor is unaware of the failure, and continues of operate as normal,
an existing Consumer could spend all funds with a channel without Adaptor able
to redeem the funds.

#### [AN-C-FORK] Cardano Network Fork {#AN-C-FORK}

[ID-C-FORK](#ID-C-FORK)

**Track.** Structural.

**Analysis.** This risk arises from the probabilistic finality of the Ouroboros
consensus. While immutable finality ($k=2160$ blocks, $\approx$ 12 hours) offers
absolute security, platforms typically adopt a shallower "Operational Finality"
(15–30 blocks, $\approx$ 5–10 mins) to meet consumer UX expectations
[^confirmation-time].

[^confirmation-time]:
    [earnpark.com](https://earnpark.com/en/posts/how-long-for-cardano-transactions-to-finalize-complete-guide/#how-many-confirmations-does-cardano-need).
    Retreived 2025-01-24

This creates a **vulnerability window** where the Adaptor may treat a Cardano
"Retainer" UTXO as confirmed and handle a payment, only for the Cardano
transaction to be invalidated by a chain rollback.

The historical event of **2025/11/21** proved that this risk is not purely
theoretical. This resulted in a 14.5-hour chain partition. The "healthy" chain
eventually overtook the "poisoned" chain (pig & chicken), forcing a rollback
that exceeded the standard 30-block confirmation threshold.

**Likelihood.** Low. Deep forks are still rare even considering the 2025/11/21.
It does demonstrate that logic bugs in node software can disrupt the
mathematical probability of finality.

**Exploitation Potential.** High, although as in previous section, it requires
Consumer to act faster than Adaptor can react.

**Impact Surface.** Critical. Total loss of the principal of payments in which
retainers are rolledback.

**Adversarial Maximization.** Significant. During periods of observed network
instability, an attacker can flood the Adaptor with requests to exploit the
shallow 15-30 block confirmation window before the Adaptor's automated
monitoring can trigger a service halt.

#### [AN-C-SET] Settlement Latency {#AN-C-SET}

[ID-C-SET](#ID-C-SET)

**Track.** Agency-driven & structural.

**Analysis.** Latency manifests through two distinct mechanisms:

- **Structural (congestion):** Systemic state where block resource demand
  exceeds supply. This affects both the sub and respond steps.
- **Agency-driven (contention):** An attacker spends the channel UTXO via
  "add-spamming" at a cadence high enough to conflict with Adaptor's sub step
  (not relevant only for respond steps).

In non-urgent scenarios, this results in redemption delays and an expanded
vulnerability window. In urgent scenarios, prolonged latency can lead to a total
loss of principal if the respond transaction (vulnerable only to structural
congestion) fails to land before the close period expires (default 1 day).

**Likelihood.** Moderate. Historically, Cardano has experienced severe
congestion during major dApp launches.

**Technical competency.** Low to moderate. Targeted contention for the sub step
requires basic mempool monitoring. While cardano has no official fee market,
transaction favoritism by specific SPOs (stake pool operators) is technically
feasible and has been observed in the wild.

**Resource commitment.** Moderate to low (depending on duration). The cost of
attack is the sum of transaction fees. At a ballpark of ~0.4 ada per
transaction, a requirement of 3 transactions/minute costs ~1.2 ada/minute.

- **Ballpark estimate:** ~500 ada for the expected 400-minute window (subject to
  [temporal divergence](#AN-B-DIV)).
- **Efficiency note:** An attacker can combine tens of channels into a single
  transaction for ~1.0 - 1.5 ada, reducing the "per-channel" cost to ~100 ada.

**Exploitation potential.** High (sub) / Low (respond). For the sub step, the
target is static and easily identified. For the respond step, exploitation is
only possible opportunistically by waiting for structural congestion, as active
contention is precluded by the protocol state.

**Impact surface.** Critical. Results in a total loss of principal owed if the
respond, or that pertaining to unlockeds for sub, transaction fails to land
before the elapse at, or timeout, respectively.

#### [AN-B-DIS] Route Discovery Failure {#AN-B-DIS}

[ID-B-DIS](#ID-B-DIS)

**Track.** Structural.

**Analysis.** Discovery failure occurs at the pathfinding stage: the Adaptor
node explores its local view of the gossip network but cannot construct a valid
sequence of hops that satisfy payment amount, fee limits, and timelock
constraints.

Success rates for commercial-grade nodes are high (>97%), but these stats are
sensitive to the "hub-and-spoke" nature of lightning. Reliability for Adaptor is
specifically sensitive to:

- **Payment size:** Larger amounts (> 0.01 btc) prune the available graph as
  fewer channels maintain high local balances.
- **Path length:** Each additional hop increases the chance of hitting a "dead
  end" due to stale gossip data.
- **Node connectivity:** As a commercially driven provider, Adaptor's
  reliability will likely exceed "random" node averages (which can be
  significantly lower) but remain susceptible to 1-3% baseline network failure
  rates.

**Likelihood.** Moderate to low. While generally reliable, failure is a
persistent background reality of decentralized routing. Success is highly
context-dependent based on destination and amount.

**Exploitation potential.** Low. Discovery failure is a passive event. An
attacker cannot easily force a discovery failure for a specific payment unless
they control the majority of routes to the destination, which is prohibitively
expensive.

**Impact surface.** Operational. Impact is limited to service unavailability and
loss of potential fees. There is **zero principal risk**; no funds are committed
or locked if a route cannot be discovered.

#### [AN-B-DEL] Route Resolution Delay {#AN-B-DEL}

[ID-B-DEL](#ID-B-DEL)

**Track.** Agency-driven & structural.

**Analysis.** Delay occurs when a payment is "in-flight" (HTLCs are committed)
but the secret is not revealed by the recipient for a prolonged period. Adaptor
is subject to the default limit of BLN, 2016 blocks[^max-lock], expected time of
two weeks. The primary causes are:

- **Structural:** Intermediary node downtime, network instability, or nodes
  routing via slow anonymity layers (e.g., Tor-only hops adding >10s latency).
- **Agency-driven:** An adversarial recipient (or intermediary) intentionally
  holds the secret until just before the HTLC timeout. In BLN this is known as a
  "griefing attack".

[^max-lock]: [Bolts](https://github.com/lightning/bolts/pull/1086)

**Exploitation potential.** High. The secret is controlled by the recipient.
Payments can be exploited as call options: attacker makes a payment to own
address with a long time out. If (and only if) ada-btc goes down in that period,
then the attacker reveals the secret completing the payment at advantageous
rates.

**Impact surface.** Operational & financial. Extended vulnerability window.
Adaptor is forced to hold a stale position, potentially leading to margin
erosion.

#### [AN-B-TIME] Route Resolution Timeout {#AN-B-TIME}

[ID-B-TIME](#ID-B-TIME)

**Track.** Agency-driven & structural.

**Analysis.** A timeout occurs when a committed payment fails to resolve within
the required timeframe, sometimes referred to as "stuck". The primary causes
are:

- **Structural:** An intermediary node on the path goes offline after the HTLC
  is locked but before the secret (preimage) is propagated back.
- **Agency-driven:** An attacker intentionally initiates payments to themselves
  through Adaptor's channels, deliberately withholding the secret to "slow-jam"
  slots. Each lightning channel has a structural limit of 483 concurrent HTLC
  slots; an attacker can render a channel unusable by filling these slots with
  low-value, stuck payments.

**Technical competency.** Low to moderate. Jamming requires basic knowledge of
lightning onion routing and the ability to run a node that refuses to resolve
incoming HTLCs.

**Resource commitment.** Low (liquidity-based). Slot based jamming can be done
with many small amounts (dust-limit). Attacker also needs enough Konduit
channels to do this. Konduit channels have, intentionally a much lower capacity
of unsquashed cheques, 10.

**Exploitation potential.** High. BLN channels have a fixed, public limit of 483
slots. An attacker can observe Adaptor's node capacity and calculate the
resources needed to jam the service.

**Impact surface.** Operational (capital immobilization). Stuck funds cannot be
used for other payments, increasing the likelihood of discovery failures. In
extreme cases, channel jamming leads to total service denial. While principal is
eventually returned via timeout, the opportunity cost and liquidity pressure are
significant.

#### [AN-B-DIV] Temporal Divergence {#AN-B-DIV}

[ID-B-DIV](#ID-B-DIV)

**Track.** Structural.

**Analysis.** Temporal divergence occurs when the stochastic nature of bitcoin
block production causes the network clock to drift behind the deterministic
cardano POSIX slots. While bitcoin targets a 10-minute mean, block intervals
follow a Poisson process (Erlang distribution).

As shown in [Appendix A](#APP-TP), there is a non-negligible statistical
probability that the time required to produce a fixed number of blocks exceeds
the expected average. For a standard 144-block HTLC (intended to represent 24
hours):

- There is a 10% probability the blocks take >26.8 hours.
- There is a 1% probability the blocks take >28.4 hours.

If the bitcoin block production is slow, and Adaptor is utilizing a 1-day (1440
minute) close period on cardano, the bitcoin HTLC may remain valid after the
cardano-side refund has already become accessible to the consumer. This risk is
exacerbated by systemic drops in global hashrate, which shift the distribution
mean beyond the IID assumptions used in standard modeling.

**Likelihood.** Moderate. Based on Erlang distribution modeling, "slow" windows
that exceed 24 hours for 144 blocks occur with enough frequency to be considered
a persistent operational reality rather than a "black swan".

**Exploitation potential.** High. An attacker does not need to influence the
bitcoin hashrate; they only need to observe a manifest "stall". By withholding
the secret until the final possible bitcoin block, they can wait for the cardano
POSIX timer to expire. If the 144th block lands after the cardano timeout, the
attacker can claim the bitcoin secret while simultaneously reclaiming their ada
principal.

**Impact surface.** Critical. Results in a total loss of principal associated
with the payment. The structural mismatch between block-time and POSIX-time
allows for a risk-free "double-spend" by the consumer during divergence events.

#### [AN-I-KEY] Cryptographic Key Compromise {#AN-I-KEY}

[ID-I-KEY](#ID-I-KEY)

**Track.** Agency-driven.

**Analysis.** Risk of unauthorized access to signing keys. Vectors include:

- **Host breach:** OS-level compromise or remote access.
- **Configuration:** Weak permissions or secrets leaked in logs/backups.
- **Supply chain:** Malicious dependencies exfiltrating signing keys.

**Technical competency.** Moderate to high. This type of compromise is akin to a
general server breach.

**Resource commitment.** Variable. Ranges from low-cost scans for
misconfigurations to high-capital targeted zero-day or social engineering
attacks.

**Exploitation potential.** High. Post-access exploitation is trivial,
constructing transactions draining all controlled funds.

**Impact surface.** Critical. Total loss of all Adaptor funds.

#### [AN-I-DATA] Data Loss {#AN-I-DATA}

[ID-I-DATA](#ID-I-DATA)

**Track.** Structural.

**Analysis.** Risk of database corruption or hardware failure leading to the
loss of local state, specifically receipts. Primary causes include:

- **Hardware failure:** Storage media degradation or file system corruption
- **Operational mishandling:** Improper database migrations or accidental
  deletion of state files.

Without receipts Adaptor cannot prove funds owed, rendering them unclaimable.

Adaptor can restore service by using the last available state either by scanning
the chain or restore from backup. The maximum loss is capped by Adaptor’s
"Global Owed Threshold", the maximum exposure permitted before a redemption
transaction is triggered.

**Likelihood.** Low. While hardware failure is a statistical certainty over long
horizons, redundant infrastructure and high-frequency backups mitigate the
probability of a data-gap occurring.

**Exploitation potential.** Low. Data loss is a passive structural failure.
Inducing targeted local database corruption remotely is non-trivial.

**Impact surface.** High. Total loss of funds owed since the last successful
redemption or backup.

#### [AN-I-SERV] Service Degradation {#AN-I-SERV}

[ID-I-SERV](#ID-I-SERV)

**Track.** Structural.

**Analysis.** Risk of Adaptor service not functioning as intended despite being
"online". Primary vectors include:

- **Indexer lag:** Cardano indexer falls behind tip, missing critical channel
  state transitions.
- **Desynchronization:** Local BLN/bitcoin node loses peers, preventing active
  HTLC monitoring.
- **Resource bottlenecks:** Internal API exhaustion or memory leaks stalling
  automated processing.

Prolonged degradation may cause sub, respond, or unlock steps to be missed,
resulting in the loss of associated funds.

**Likelihood.** Moderate. The blockchain software stack frequently experiences
interface divergences. Upstream project changes can impact downstream
dependencies silently, leading to hard-to-detect failures in production.

**Exploitation potential.** Low. Primarily a structural failure. While DoS is
possible, triggering specific internal logic bottlenecks remotely is difficult.

**Impact surface.** Moderate to high. Potential principal loss due. Increased
exposure to exchange rate drift during "blind" windows. Erosion of consumer
confidence.

#### [AN-E-VOL] Market Volatility {#AN-E-VOL}

[ID-E-VOL](#ID-E-VOL)

**Track.** Structural.

**Analysis.** Risk of a fall in the ada-btc pair exceeding Adaptor's pricing
buffer. Exposure is modeled by the standard deviation of daily returns
($\sigma$), scaled by the "vulnerability window" ($\Delta t$):

$$M = z \cdot \sigma \cdot \sqrt{\Delta t}$$

For the ada-btc pair, $\sigma$ typically ranges between 4% and 6% (by comparison
eur-usd is 0.4%-0.6%).

The model assumes a standard distribution, but is invalidated during "black
swan" events where the "Square Root of Time" rule underestimates tail risk.
Historical precedents demonstrate this fragility:

- **Systemic Crashes:** In May 2021, the market saw daily drops exceeding 25%.
  Similar "deleveraging" events in early 2025 saw altcoin pairs fall 12–15% in
  <24 hours.
- **Liquidity Thinning:** During periods of high stress, order book depth
  evaporates. This increases the realized $\sigma$ instantly.

If $\Delta t$ is extended by [AN-B-TIME] or [AN-C-SET], the probability of $M$
breaching the Adaptor's margin increases non-linearly.

**Likelihood.** High. Volatility is the baseline state of the asset pair.
High-impact "tail events" ($>3\sigma$) are not anomalous in crypto-markets; they
are recurrent features.

**Exploitation potential.** Low (in isolation). Market volatility is an external
dependency. See [TODO] for how it used along with other risks.

**Impact surface.** Moderate to high. Results in a loss of funds not yet
redeemed times the price drop.

#### [AN-E-ORA] Oracle Data Error {#AN-E-ORA}

[ID-E-ORA](#ID-E-ORA)

**Track.** Structural.

**Analysis.** Risk of Adaptor utilizing stale, manipulated, or otherwise
inaccurate pricing for the ada-btc pair. Since Adaptor must provide a quote
before the trade, any lag in the price feed creates an "arbitrage window".
Vectors include:

- **API Latency:** Stale prices during high-volatility periods.
- **Outlier Data:** A single exchange "flash crash" skewing the aggregate price.
- **Connectivity Loss:** Adaptor's price-fetcher fails, reverting to last-known
  value.

**Likelihood.** Moderate. API outages and "socket hangs" are common in the
cryptocurrency data ecosystem.

**Exploitation potential.** High. An attacker (or automated bot) can monitor the
Adaptor's quoted rate against the global spot price.

**Impact surface.** Moderate to high. Direct financial loss through unfavorable
trade execution. Clear financial benefit from exploitation.

---

#### [AN-E-GATE] Exchange/Gateway Failure {#AN-E-GATE}

[ID-E-GATE](#ID-E-GATE)

**Track.** Structural.

**Analysis.** Risk of the inability to access external venues for rebalancing or
hedging. The Gateway is the mechanism by which the Adaptor maintains a "neutral"
book. Failure occurs via:

- **Technical Outage:** Exchange API downtime or maintenance.
- **Service Termination:** Sudden account suspension or regulatory "de-risking".
- **Liquidity Crunch:** Inability to query market depth, leading to poor
  execution on necessary hedges.

This is fundamentally an "inventory risk". If the Adaptor fulfills a bitcoin
payment but cannot immediately buy back the equivalent bitcoin on an exchange,
they are left with a directional "long" position in ada.

**Likelihood.** Low to moderate. Major exchanges have high uptime, but
individual account restrictions or localized API throttles occur frequently.

**Exploitation potential.** Low. An external attacker cannot easily trigger a
Gateway failure, though they can observe the resulting "imbalance" and market
distortion.

**Impact surface.** High. Leads to [AN-I-SERV](#AN-I-SERV) (service
degradation). The primary impact is the inability to hedge, leaving Adaptor
critically exposed to [AN-E-VOL](#AN-E-VOL) (market volatility). If the market
moves while the Gateway is down, Adaptor's unhedged "long" position can suffer
massive drawdown.

### Compound Analysis

We consider how risks may interplay in a scenario.

TODO

---

## Evaluate {#E}

TODO

---

## Treat {#treat}

TODO

---

## Appendices

### Quantifying Temporal Divergence {#APP-TP}

TODO :: this needs to be tidied.

One of the key motivations of using lightning is (near) instant finality. This
is in contrast with blockchains like Bitcoin and Cardano that have probablistic
finality, and can experience rollbacks. In certain respects this 'contrast' is
really 'at odds'.

In bitcoin, block production is random and is based on the network "hashing
power" and the current "difficulty". Every 2016 blocks, bitcoin undergoes a
difficulty adjustment. This process sets parameters such that for the given
hashing power a block time averages (almost) 10 minutes.

For a fixed hashing power and difficulty (together with assumptions the L1 is
functioning), block times can be assumed to be IID (independently identically
distributed) with exponential distribution. Under these assumptions, and
assuming block propogation is relatively negligible, we can treat block
production as a Poisson point process (see also
[Erlang distribution](https://en.wikipedia.org/wiki/Erlang_distribution)). The
time (in mins) `X_k` to produce `k` consecutive blocks has expectation, and
standard deviation:

$$ \mathbb{E}(X*k) = k * 10, ~~ \sigma(X*k) = sqrt(k) * 10$$

An Erlang distribution is closely related to the chi-squared distribution. By
this, or other means, we compute a waiting time bound `T`.

$$T_{conf}(k) = 5 \chi^{2} _{conf} (2 k)$$

`T` signifies that with a confidence of `conf`, `k` blocks are produced in time
`< T`.

Below is a table of some illustrutive values.

+--------+------------------+-------+------------------+------------------+------------------+
| Blocks | Expected Time | | p=0.10 (90% Conf)| p=0.05 (95% Conf)| p=0.01 (99%
Conf)| | (k) | (Average) | Value | (Upper Bound) | (Upper Bound) | (Upper Bound)
|
+--------+------------------+-------+------------------+------------------+------------------+
| 1 | 10.0 min | chi2 | 4.605 | 5.991 | 9.210 | | | | Time | 23.0 min | 30.0 min
| 46.1 min |
+--------+------------------+-------+------------------+------------------+------------------+
| 10 | 100.0 min | chi2 | 28.412 | 31.410 | 37.566 | | | | Time | 142.1 min |
157.1 min | 187.8 min |
+--------+------------------+-------+------------------+------------------+------------------+
| 40 | 400.0 min | chi2 | 96.578 | 101.879 | 112.329 | | | | Time | 482.9 min |
509.4 min | 561.6 min |
+--------+------------------+-------+------------------+------------------+------------------+
| 144 | 1440.0 min | chi2 | 321.92 | 328.58 | 341.33 | | | (24.0 hrs) | Time |
1609.6 min | 1642.9 min | 1706.7 min | | | | | (26.8 hrs) | (27.4 hrs) | (28.4
hrs) |
+--------+------------------+-------+------------------+------------------+------------------+
| 288 | 2880.0 min | chi2 | 623.51 | 632.71 | 650.21 | | | (48.0 hrs) | Time |
3117.6 min | 3163.6 min | 3251.1 min | | | | | (52.0 hrs) | (52.7 hrs) | (54.2
hrs) |
+--------+------------------+-------+------------------+------------------+------------------+

The above estimates are based on the assumptions stated. A drop of hashing power
will likey lengthen of block times and in particular, these bounds would be
incorrect.

Attempts to provide emperical evidence to support this modelling faces some
challenges. See, for example, [this paper](https://arxiv.org/pdf/1801.07447).
With that considered, the conclusions of the paper do not seem to contradict the
conclusions of the modelling. Signficant drops in hashing power between
adjustments are not unheard of, for example, May 2021, and June 2024 (FIXME ::
citation).

On the question of exploitability, an attacker can easily wait for the last
block to reveal a secret. To cause an actual exploit, they would also need
bitcoin block production to be slow enough that this last block occurred after
the timeout of a coresponding cheque (or at least so close that Adaptor is
unable submit a transaction).

Aforementioned drops in hashing power are generally anticipated. Adaptor must
stay abreast of such developments regarding drop in hashing power. It is
possible that an attacker has non-public information that anticipates hashing
power will be dropping significantly. However, we have a hunch that such
information is far far more valuable in contexts other than Konduit.

#### Quantifying Routing Issues

Hard numbers on the prevelance and causes of route finding failures are hard to
find. This is often the case in decentralized networks. Moreover, the statistics
that are available are generally up front regarding how unrepresentative they
may be, since they are based on their view of the network. We cannot be
confident they are representative of those of Adaptor.

Without a route, Adaptor cannot handle payments. No funds are at risk, only loss
of potential earnings.
[River](https://river.com/learn/files/river-lightning-report-2023.pdf), a BLN
service provider, have shared stats on this issue from their (their nodes')
perspective. According to River, between 1-3% (depending on the month) of route
findings failed. The two main reasons identified:

- insufficient liquidity
- receiver offline

Some [sources](https://youtu.be/zk7hcJDQH-I?t=740) suggest the failure rates of
"random" nodes is likely far higher than nodes of commercially driven payment
providers. According to this [source](https://youtu.be/zk7hcJDQH-I?t=920), 99.7%
were "fast" (< 30 seconds), and of the remaining 0.11% were resolved slowly,
while 0.19% failed outright. The two sources do not seem to agree on the
numbers, but this is surely due to the context of their sample.

We can be confident that reliability is highly sensitive to the following
parameters: payment amount, route length (number of hops).

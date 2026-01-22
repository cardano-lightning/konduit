---
title: Adaptor Risk Assessment
---

Adaptor handles payments in the belief that they can redeem funds owed.
Konduit's design gives this belief foundation, but it does not make Adaptor's
position risk-free. Adaptor must be clear eyed regarding the risks to which they
are exposed.

This document considers:

- sources of risk
- under what conditions the risk is realised
- when and how sources of risk may be exploitable
- expectations and upper bounds of potential losses

This document considers these risks in terms of:

- Infrastructure
- Single payment
- Coupled risks

The considerations here are not comprensive in the sense of that they are not
all possible risks to Adaptor. It is focussed to risks that are particular
operating as an adaptor, in contrast to, say, some other B2C digital service
provider.

# Infrastructre

## Settlement layer

Konduit cannot be more secure than the blockchain(s) over which it is built.
This is holds for any application dependent on blockchain. If there are
fundamental flaws in, say, the implementation of cryptography then regarding the
well working of Konduit channels, all bets are off.

Moreover, Konduit's design assumes that the L1s are functioning within some
tolerable range. Sometimes these assumptions are demonstrably invalid. For
example 2025/11/21 Cardano experienced a fork. This was caused by a transaction
that some nodes deemed valid, while others deemed invalid. The network
eventually recovered, however it showed that numbers around, say, block time are
based on assumptions that do not always hold.

## Internal infrastructure

It goes without further comment that if Adaptor has compromised keys then they
ought to expect to lose all funds owed, and all funds controlled by those keys.

If Adaptor looses receipts they are unable to redeem any owed funds. The L2
protocol requires Adaptor to provide consumer with latest previous squash. The
recommended course of action is for the Adaptor to treat the channel as
`is_served = false`.

Adaptor may continue to serve the channel without incurring additional loss.
They can recover data from chain by finding previously submitted txs. These will
include the squashes and used unlockeds. Anything unrecoverable is previously
owed funds that are written off. Channels with no transactions require a null
squash before being served.

## Price data

Adaptor must have confidence the price feed for which they are computing quotes,
and handling cheques, are realistic. Failure to do so can result in losses.

# Single payment

In a single payment, Adaptor commits to a payment in bitcoin, on the commitment
of receiving ada. Analogous services exist in traditional finance.
Traditionally, such a service provider is exposed to three sources of risk:

- Principal risk: the debtor defaults/ fails to make the promised payment.
- Market/ FX risk: the relative value of the received assets has dropped since
  the agreement.
- Temporal liquidity risk: the service provider can use the funds they would
  otherwise have between the commitment to pay, and awaiting payment.

## Principal risk

Konduit derisks the principal risk subject to certain assumptions. Namely:

- Confirmed retainers are final
- Payment resolution is not severly delayed
- Adaptor txs make it onchain

### Bad retainer

One of the key motivations of using lightning is (near) instant finality. This
is in contrast with blockchains like Bitcoin and Cardano that have probablistic
finality, and can experience rollbacks. In certain respects this 'contrast' is
really 'at odds'.

Adaptor routes a payment having identified a retainer ie with a specific UTXO
that is "backing" the payment. Cardano (Praos) takes 2160 blocks to be
considered final, about 36 hours. The probability of a block being eventually
final is:

```
[TODO]
```

Note, that the formula assumes the L1 is functioning as intended. The 2025/11/21
incident shows these assumptions are not always adhered to.

There is trade off between Consumer UX and Adaptor safety: Consumer will be less
likely to use the service of Adaptor the longer the confirmation time is.
Centralised exchanges and payment platforms have exposure to the same risk. Hard
numbers on acceptable confirmation times are not easy find. Some
[sources](https://earnpark.com/en/posts/how-long-for-cardano-transactions-to-finalize-complete-guide/#how-many-confirmations-does-cardano-need)
suggest 15-30 blocks, about 3 to 5 minutes, and longer for larger transaction
values. This aligns with the authors experience.

Suggested mitigations:

- Consider retainers only if sufficiently confirmed
- Suspend operation if there known network issues

### (Very) slow payment resolution

Adaptor's position is not only in two different units of value, but also in two
different units of time. BLN uses (bitcoin) block height for time, while Cardano
uses posix time.

Pay resolution, the process by which the secret is learned by Adaptor, is often
near instant. It is a lower bound between Adaptor committing bitcoin and
redeeming ada. However, pay resolution can take considerably longer.

BLN nodes are generally configured to not accept pay commitments exceeding 2,016
blocks (~2 weeks) [source](https://github.com/lightning/bolts/pull/1086). This
is the defactor limit for Adaptor's commitments. In a worst case scenario this
limit can be realised.

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

```math
Exp(X_k) = k * 10
Std(X_k) = sqrt(k) * 10
```

An Erlang distribution is closely related to the chi-squared distribution. By
this, or other means, we compute a waiting time bound `T`.

```math
T(k; conf) = 5 * chisquare(2 * k; 1 - conf)
```

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

Mitigations:

- Handle payments with a time delta that aligns with their risk appetite
- Increase minimum timeouts if there is drop in hashing power

### Tx failure

Konduit txs are txs; if txs aren't making it into blocks then Konduit channels
are affected. There are two considerations: congestion and contestation.

Chain congestion has previously been an issue on Cardano. (FIXME :: citation).
There are no "official" work arounds such as a fee market. This may only lead to
a delay in the accessing of owed funds. However, some steps are more urgent than
others: sub with unlocked and unsquashed cheque; a respond.

It is expensive to orchestrate congestion, although not wildly so. (FIXME ::
Numbers). Events causing congestion, like a protocol launch, may be used by an
attacker. either with anticipation or oppotunistically.

The only meaningful contention is caused by Consumer submitting a tx very
shortly before Adaptor. Consumer may exploit this via "add spamming", repeated
doing add steps. The cost of this is simply the tx fees required to do so.

Mitigations:

- Limit total owed before submitting a tx
- Increase requied timeout delta if Cardano is experiencing congestion
- If congestion is servere, suspend service
- Being on very good terms with block producers, in order to interupt add
  spamming

## FX risk

An adaptor that has committed to a payment in BTC with the promise of ada is
exposed to this market volatility. Crashes of more 25% in less than 24 hours
have occurred in the ada-btc price (FIXME :: Citation needed).

See the previous section for a exposition on worst case timebounds. As expounded
above, can not only be realised but can orchestrated by an attacker. More
concerning still, without mitigations Konduit channels can be used effectively
as call options.

Mitigations:

- Monitor FX volatility;
- If volatile then: increase fees; decrease max acceptable timeouts; decrease
  max acceptable value
- Purchase call options for bitcoin matching exposure

## Temporal liquidity risk

Unlike the previous sections, this does not lead to a loss of funds, but a loss
of potentially earnings due to an inefficient allocation of capital. See the
previous section for a exposition on worst case timebounds. Again, this is
exploitable.

Mitigations:

- Place more restrictions on new or questionable channels
- Limit single max cheque size

# Coupled risks

The risks expounded above are focussed on risks at the single payment level. It

Price volatility, rollbacks, and chain congestion can be correllated. Moreover,
payments associated to different chane

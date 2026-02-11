# Log

> Konduit's project log

Core contributors are asked to regularly (ie weekly when things are running a
pace). Entries should be in reverse chronological order ie most recent first.
Below is an entry template.

### yyyy-mm-dd

@\<you\>

- Describe : **What** did you work on this week? (little prose, mostly links to
  PR, docs, source code, ..)
- OKRs : **What** outcome/key result did it support?
- Next : **What** is immediately next?

## Entries

### 2025-05-31

@waalge:
[fix-multi-unlocked](https://github.com/cardano-lightning/konduit/pull/69) is
merged, albeit as incomplete. Majority of efforts were on bringing coherence to
the server.

OKRs:

```
roadmap // v1 // konduit server // second implementation
```

Next:

- Align bln-client with fx-client interms of cli behaviour
- Run manually driven testing of the bln-client
- Re-do the konduit server handlers for pay and quote

### 2025-01-31

@waalge:
[fix-multi-unlocked](https://github.com/cardano-lightning/konduit/pull/69/changes/0be1c46cb47e6e5a77cbd21ee9f5b9610b813ccd)
work continues on konduit-server (currently konduit-adaptor). An accidental,
large refactoring brought about as the amount of incoherence in the existing
code became unmanageable while trying to bring in the new.

OKRs:

```
roadmap // v1 // konduit server // second implementation
```

Next:

- Complete refactor of konduit-server. Get thing running again

### 2025-01-24

@waalge:
[fix-multi-unlocked](https://github.com/cardano-lightning/konduit/pull/69/commits/982fcc1c29e98e100c37b41ef142c7bac91249ae)
Continued work on the risk assessment for Adaptor. ADR on
[rate limit add](https://github.com/cardano-lightning/konduit/blob/2de196b8481a3fb1e06a0b75e08d32383b630863/docs/adrs/01-rate-limit-add.md),
proposing two possible solutions to prevent an "add spamming" attack described
in the risk assessment.

OKRs:

```
roadmap // v1 // maturity // docs // quantified adaptor risk assessment
```

Next:

- Find pragmatic solution(s) to get adaptor server back on track

### 2025-01-17

@waalge:
[fix-multi-unlocked](https://github.com/cardano-lightning/konduit/pull/69/commits/0ca5a8ce00835c79d0803b2562ceaffeb57294f2)
has CLI working. At least open, add, and sub are tested. Time has been sunk into
providing a quantified risk assessment for Adaptor. This includes considering
the many "what happens if..." after Adaptor routes a payment. This has stalled
work on finishing the above PR to fixing the afore named `konduit-adaptor`
service.

OKRs:

```
roadmap // v1 // product // konduit tools // konduit cli // iteration after second draft
roadmap // v1 // maturity // docs // quantified adaptor risk assessment
```

Next:

- First draft of quantified risk assessment
- Find pragmatic solution to get adaptor server back on track

### 2025-01-10

**What** did you work on this week? (little prose, mostly links to PR, docs,
source code, ..)

Main focus has been on the branch `w/fix-multi-unlocked`. Previous work had seen
an update to the kernel, that fixed the inability to handle two unlocked cheques
with different timeouts in an unambiguously safe manner. This week work
continued on propagating these changes into the rust codebase.

The CLI got a re-design to be user centric; yet another manifestation of a
deeper appreciation we now have of the heterogeneous nature of Konduit channel
participants.

**What** outcome/key result did it support?

```
roadmap // v1 // product // konduit tools // konduit cli // iteration after second draft
```

**What** is immediately next?

Complete the work of the CLI to get txs working again. Then revisit the adaptor
server.

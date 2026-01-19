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

### 2025-01-17

@paluh: [Basic TS app organization +
typing]https://github.com/cardano-lightning/konduit-js/pull/2/commits/f8a2702f527084242094d000ca98f79c370626e6
I ported the basic structure of the App. I separated some minimal CSS theming,
reusable components and coused on making the core screen working as stubs
mostly. As the main objective is to port the payment flow now I'm going over the
separate sections of the app (settings, channel setup and invoice scanning) and
impementing those one by one. The above flow enforces me to also port or
re-implement a set of core libraries which I do on "demand" basis. So far I'v
ported or re-implemented the following libraries: `currency-format`, `cardano`
(basic types, operations), `hex`, partially 'konduit-consumer' and improved
error handling in the `bln` lib. [Konsoidate and improve konduit-consumer
lib][https://github.com/cardano-lightning/konduit-js/pull/2/commits/f895297e90c8c324e084323ea8faf6d6fcd733b4]
I decided to actually consolidate and improve the `konduit-consumer` lib
(integrated `cardano`, `hex` into it to minimize the number of sub-libs). I also
established some API standars in the library code - we avoid exceptions and use
explicit error handling thorugh `Result` types instead.

OKRs: TBD

Next: I'm focsuing on the embedded wallet section now in the app. I will try to
improve the key management a bit and make the syncing logic working. All this
should be packaged up as a small self contained lib.

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

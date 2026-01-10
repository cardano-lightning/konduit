# Log

> Konduit's project log

Core contributors are asked to regularly (ie weekly when things are running a
pace). Entries should be in reverse chronological order ie most recent first.
Below is an entry template.

### yyyy-mm-dd

**What** did you work on this week? (little prose, mostly links to PR, docs,
source code, ..)

**What** outcome/key result did it support?

**What** is immediately next?

## Entries

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

Safety.

**What** is immediately next?

Complete the work of the CLI to get txs working again. Then revisit the adaptor
server.

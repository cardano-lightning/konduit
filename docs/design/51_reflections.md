# Reflections

Some thougths on design decisions, and pariticularly on ones that subsequently
seemed suboptimal.

## Reflections on v0.0

### Unsafe for multiple unlocks

PROBLEM

The design leaves a hole. It is not possible for Adaptor to hold two unresolved
cheques. The reason is as follows.

Suppose the following happens:

- Adaptor accepts a cheque, cheque 1. It fails to resolve.
- Adaptor accepts a cheque, cheque 2. It also fails to resolve.
- Cheque 1 resolves. Adaptor subs with receipt using unlocked version of
  cheque 1.
- The timeout of Cheque 1 passes.
- Cheque 2 resolves.

Now if Adaptor subs with receipt using unlocked version of cheque 2, they cannot
include the value of cheque 1. The tx will be invalid if cheque 1 is included
since it has timed out.

PROPOSAL

A solution to this is to record in the Datum any `Used` cheques. This consists
of `(Index, Amount)`.

The inclusion of used cheques or some is necessary.

### Inconsistent types

PROBLEM

The design uses many lists where `constr` would be more traditional. This choice
is motivated by the desire to avoid the pointless wrapping. Mostly by accident,
`Constants` does not follow this convention.

SOLUTION

Flatten `Contstants` so it looks like everything else.

### Overuse of backpassing

PROBLEM

The design tries to enforce only what is required. The handling allows the
timebounds to be not set when they are not required. The result is laborious
case handling. For example, a validity range upper bound is required in certain
steps, and in the case of a sub only when there is an unlocked in the receipt.

SOLUTION

Assume there are always finite time bounds regardless of whether or not they are
required. Moreover, include the bounds in function args and check in the step
function body, rather then backpassing.

Its unclear if this is actually better.

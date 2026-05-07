# BLN Client

> Clients for konduit server needs

## Implementations

- [x] LND
- [ ] core

## Justfile

There is a complementary justfile. This is a quick way to crosscheck information
coming from the server.

## NOTES:

I am using "simple" rest api which seems second class relative to gRPC in terms
accuracy of docs.

I don't know what to do with reveal. LND does not expose `v1/payement/{}`
analogue of `v1/invoice/{}`, ie query by `r_hash`. The naive thing to do is to
hit `v1/payments` but this is _all_ payments. Slightly less naive is to add time
bounds, but this would then leak into the client API. alternatively we make the
stateful...

The key property we want is: robustness. If (when) the thing falls over its very
simple to stand back up again.

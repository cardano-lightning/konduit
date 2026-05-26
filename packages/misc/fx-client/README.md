# FX clients

> FX Clients for konduit server needs

## Implementations

- [x] Fixed - Useful for testing
- [x] Coin gecko - Works with or without token. Beware, rate limiting is very
      sensitive.
- [x] Kraken (also subject to rate limiting but seems less fussy)

## Notes

WARNING : The implementations use curl rather than reqwests. At the time on
development, some APIs seemed to not serve reqwest request, but would serve
curl. Something about fingerprinting the client? Unclear.

Spawning a subprocess and running curl seemed to be more reliable.

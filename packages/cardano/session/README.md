# cardano-session

Session facilitates the construction and submission of transactions
relating to some application.

It provides chain connection together with a wallet to fuel txs.
It can cache tip.

## TODO:

- [x] be connector agnostic
- [x] be wallet agnostic (support CIP-30)
- [ ] better cli support:
  - [ ] track / untrack
  - [ ] prettier utxos
  - [ ] show info sep from chain data.
- [ ] wasm capable.

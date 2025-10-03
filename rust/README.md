# Konduit tools

> A collection to crates to run konduit in the wild

## Workspace

- cardano-tx-builder - Pure cardano transaction builder utils
- cardano-connect - Traits for cardano connector
- cardano-connect-blockfrost - Impl for blockfrost
- konduit-core - Pure konduit transaction builders. Depends only on
  cardano-tx-builder.
  - test round trip data
  - compiles to wasm
- konduit-cli - cli wrapping of core

## TODOs

- [x] serde for relevant data
- [ ] env
  - [ ] wallet keys
  - [ ] cardano connection
- [ ] txs
  - [ ] dev
    - [ ] send
    - [ ] publish
    - [ ] unpublish
  - [ ] open
    - [ ] cli params TODO
    - [ ] fn params:
      - fuel (available utxos)
      - change address
      - konduit address :
        - `konduit_hash`
        - maybe stake key
      - amount (currency is always ada)
      - datum:
        - constants`(tag, add_vkey, sub_vkey, respond_period)`
  - [ ] add
  - [ ] sub
    - [ ] cli params TODO
    - [ ] fn args:
      - generic script args: fuel, change address, script ref ...
      - instance input (resolved)
      - redeemer: receipt = (squash, [cheques])
  - [ ] close
  - [ ] respond
  - [ ] unlock
  - [ ] expire
  - [ ] end
  - [ ] elapse
  - [ ] batch
  - [ ] mutual
- [ ] cardano connection
  - [ ] api
    - [ ] utxos at (with optional stake key)
  - implementations:
  - [ ] blockfrost
  - [ ] kupmios
- [ ] env handling
- [ ] cmd

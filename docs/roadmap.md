# Roadmap

## v1

### Product

- [ ] Kernel
  - [ ] Specification
    - [x] First draft - First complete version
    - [ ] Second draft - Sweep after kernel implementation
    - [ ] Final draft - Sweep after tests and e2e implementation
  - [ ] Implementation
    - [x] First draft - First complete "working" (yet untested) version
    - [ ] Second draft - Test coverage and e2e suggests justifies "working"
    - [ ] Final draft - Tidy up and align with final draft of docs
  - [ ] Tests
    - [ ] Baseline - Some basic tests of sharp edges
    - [ ] Thorough - Testing coverage is thorough (subject to what is
          manageable)
- [ ] General tooling
  - [ ] Cardano tx builder
    - [x] First implementation - Initial complete version
    - [ ] Collecting user feedback (even if that's mainly with dogfooding)
    - [ ] Second draft - Iteration reflecting on user feedback
  - [ ] Cardano connect (design)
    - [x] First draft
    - [ ] Blockfrost implementation
    - [ ] Extra : Ogmios/Kupo implementation
    - [ ] Cloudflare deployment
    - [ ] Other deployment
    - [ ] Collecting user feedback (even if that's mainly with dogfooding)
    - [ ] Second draft - Iteration reflecting on user feedback
- [ ] Konduit tooling
  - [ ] Konduit data - rust lib for encoding/decoding required for L1 and L2
        interations
    - [x] First implementation
    - [ ] Iterations after second drafts of Kernel
    - [ ] Roundtrip and cross compat with kernel tests
  - [ ] Konduit tx - tx builders for Consumer and Adaptor
    - [ ] First implementation: Support for all steps, in variety of scenarios.
          May ignore mutual txs, and may ignore support for simultaneous use of
          Consumer and Adaptor steps.
    - [ ] Iterations after second draft of Kernel
  - [ ] Konduit cli
    - [x] First implementation
    - [ ] Iterations after second drafts of Kernel
    - [ ] Demonstrates full lifecycle management
    - [ ] Bench for adaptor of "sub" in real-ish world conditions
    - [ ] Hand driven tests work after final version of Kernel
- [ ] Konduit adaptor - Adaptor service
  - [ ] Interface specification
    - [x] First draft (roughly compelete upto Auth)
    - [ ] Second draft - Justifiable "complete" given a first version of App
          works.
    - [ ] Final draft - Tidy and aligns with final version
  - [ ] Implementation
    - [x] First implementation
    - [ ] Supports multiple service backends (price feeds, connectors, BLN
          nodes)
    - [ ] Second implementation
    - [ ] Final implementation
  - [ ] Tests
    - [ ] Runs some headless tests against the different services independently
    - [ ] Runs some integration tests
- [ ] Konduit app - Consumer App
  - [ ] Specification / Design
    - [x] Requirements doc
    - [x] Wireframe
    - [x] Mockup
    - [ ] Second iteration
  - [ ] Implementation
    - [ ] First implementation - Basic channel management operations; payments.
    - [ ] Support for bolt11 payments
    - [ ] Extra : Support for other payment request types
    - [ ] Second iteration - Works with adaptor server final implementation
  - [ ] Tests
    - [ ] Distinct service "unit" testing
    - [ ] Integration testing
    - [ ] User driven tests
  - [ ] Deployment
    - [ ] "Easy" instal as PWA

### Maturity metrics

- [ ] Consumer / Payments
  - [ ] > 100 stickers sold (on testnet if not mainnet)
  - [ ] > 10 distinct devices
  - [ ] > 100 separate payments
  - [ ] > 5 contexts (ie different merchants)
  - [ ] Active feedback collection and reflection
- [ ] Adaptor / Infra
  - [ ] > 2 distinct (Externally maintained) deployments
  - [ ] > 1 BLN backend
- [ ] Engagement
  - [ ] > 30 gh stars
  - [ ] > 10 gh users that have in someway engaged (forked/ gh issue/ _etc_)
  - [ ] > 5 Whatever the eqivalent of LoI is for an OS project like Konduit
  - [ ] Live presentation

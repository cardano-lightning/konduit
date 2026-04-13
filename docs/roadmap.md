# **Konduit Roadmap**

## **Phase 1: Foundation & Core Logic**

Focus: Establishing the mathematical and technical primitives for L1/L2
interaction.

### **Protocol Kernel**

- [x] \[P1-KER-SPEC\] First draft specification
- [x] \[P1-KER-IMPL\] Initial "working" implementation
- [x] \[P1-KER-TEST\] Baseline testing of sharp edges
- [ ] \[P1-KER-PROP\] Property-based testing for fund safety invariants
- [x] \[P1-KER-REF\] Refine specification based on initial implementation
      findings

### **Essential SDKs & Data**

- [x] \[P1-CSDK-INIT\] Cardano SDK: Initial version
- [x] \[P1-KCOR-INIT\] Konduit data: Rust lib for L1/L2 encoding/decoding
- [x] \[P1-KCOR-WASM\] JS/Wasm Bindings: Export KCOR logic for use in web-based
      Consumer App
- [x] \[P1-CCON-BLK\] Cardano connect: Initial design and Blockfrost
      implementation
- [x] \[P1-KTX-INIT\] Konduit tx: Support for primary Consumer and Adaptor steps

### **Phase 1 Success Criteria**

- [ ] \[P1-CRIT-CPAT\] Kernel and Rust libs pass cross-compatibility tests
- [ ] \[P1-CRIT-IDOC\] Stable internal documentation for core protocol logic

## **Phase 2: Functional Lifecycle (Alpha)**

Focus: Connecting the components to enable a full end-to-end payment flow using
the current/initial server.

### **Infrastructure & Server**

- [x] \[P2-SRV-INIT\] Adaptor server: First implementation
- [ ] \[P2-SRV-BKND\] Adaptor server: Support for price feeds and BLN nodes
      backends
- [ ] \[P2-SRV-TEST\] Headless testing against independent services
- [ ] \[P2-SRV-SYNC\] State Re-sync: Reconciling state with chain and with app.

### **Client Interfaces**

- [x] \[P2-CLI-INIT\] Konduit CLI: Initial version
- [x] \[P2-APP-DSGN\] Consumer App: Requirements and design mocks
- [ ] \[P2-APP-IDEM\] App-level Idempotency: Ensure duplicate UI actions don't
      trigger double payments
- [ ] \[P2-APP-IMPL\] Consumer App: First implementation of channel management
      and payments
- [ ] \[P2-CLI-LIFE\] CLI: Demonstrates full lifecycle management (Hand-driven
      tests)

### **Phase 2 Success Criteria**

- [ ] \[P2-CRIT-E2E\] Successful end-to-end payment via CLI in real-ish
      conditions
- [ ] \[P2-CRIT-OAPI\] Initial API documentation (OpenAPI) completed
- [ ] \[P2-CRIT-STAR\] \> 10 GitHub stars and initial community engagement

## **Phase 3: Product Maturity (Beta)**

Focus: Reliability, security, and architectural stability.

### **Hardening & Features**

- [ ] \[P3-SRV-CBOR\] CBOR-Native API: Transition server API entirely to CBOR
      with an optional JSON translation layer
- [ ] \[P3-SRV-VER\] Protocol Versioning: Implement version negotiation between
      Consumer and Adaptor for binary schemas
- [ ] \[P3-SRV-REDG\] Server Interface Redesign: Finalize standard/usable API
      based on CBOR-native schema
- [ ] \[P3-SRV-AUTH\] Implementation of Auth layer for Server/Adaptor
- [ ] \[P3-SRV-FIN\] Adaptor Server: Final implementation with multiple backend
      support
- [ ] \[P3-APP-BOLT\] Consumer App: Support for bolt11 and extra payment request
      types
- [ ] \[P3-APP-RECO\] Recovery Logic: Tools for fund recovery if PWA/Server
      state is lost

### **Validation & Documentation**

- [ ] \[P3-KER-TEST\] Kernel: Full test and benchmark suite
- [ ] \[P3-VAL-INTG\] Integration testing across all distinct service units
- [ ] \[P3-DOC-GUID\] High-level explainer and setup/maintenance guides
- [ ] \[P3-RSK-ASSS\] Quantified Adaptor risk assessment

### **Phase 3 Success Criteria**

- [ ] \[P3-CRIT-TRAX\] \> 100 payments / stickers sold (testnet or mainnet)
- [ ] \[P3-CRIT-DEVS\] \> 10 distinct devices interacting with the protocol
- [ ] \[P3-CRIT-DPLY\] \> 2 externally maintained Adaptor deployments

## **Sidequests**

Focus: Alternative implementations and infrastructure flexibility.

- [ ] \[SQ-CCON-OGM\] Support for Ogmios/Kupo and other deployment environments
- [ ] \[SQ-DPLY-CLD\] Cloudflare deployment options
- [ ] \[SQ-KTX-NCHE\] Extra : Support for niche payment request types
- [ ] \[SQ-SRV-BNCH\] Benchmarking adaptor for "sub" (recurring/automated)
      conditions

## **Future Horizons**

- [ ] \[HZ-APP-PAY\] Expansion of supported payment request types
- [ ] \[HZ-KTX-MUTL\] Advanced mutual transaction support in tx builders
- [ ] \[HZ-SRV-BCK\] Additional service backends for connectors

---
title: "Restructuring and Namespacing Off-chain Rust Crates"
author: "@waalge"
date: 2026-04-13
tags:
  - architecture
  - rust
  - refactor
---

## **Context**

The current off-chain tooling is organized in a flat directory structure. This
mixes Konduit-specific logic, Cardano-specific integrations, and generic
infrastructure. As the project scales, we need a hierarchy to distinguish
domain-specific crates from reusable ones. While bln-client and fx-client are
specific to the Konduit ecosystem, others like http-client are generic.

## **Decision**

We will reorganize the directory structure into logical namespaces. To minimize
breaking changes across the ecosystem, **crate names within Cargo.toml will
remain unchanged**. Only the physical paths and directory names will change.

1. **Namespacing**: Organize crates into the following top-level directories:
   - konduit/: Konduit-specific tooling, including bln-client, fx-client, and
     all konduit-\* crates.
   - cardano/: All Cardano-specific connectors and SDKs.
   - shared/: Generic utilities such as http-client.
2. **Directory Renaming & Path Refinement**:
   - Move konduit-server to the directory konduit/adaptor/.
   - Move konduit-client to the directory konduit/consumer/.
   - Move konduit-cli to the directory konduit/cli/.
   - Update the workspace Cargo.toml to rewire member paths to these new
     locations.

## **Decent, counter, and comments**

- **Comment**: Keeping the crate names unchanged prevents breaking downstream
  dependencies that rely on the package registry name, while providing a cleaner
  local development experience.
- **Comment**: This is a step in the direction of `cardano-sdk` and potentially
  other aspects (eg `cardano-connector`) existing in their own repos.

## **Status**

Proposed.

## **Consequences**

- **Positive**: Logical grouping of crates by domain (Konduit vs. Cardano vs.
  Shared).
- **Positive**: Directory names like adaptor and consumer provide better
  architectural context than generic server/client labels.
- **Negative**: Requires a significant update to the workspace Cargo.toml and
  any relative path dependencies within member crates.
- **Neutral**: No impact on external crate consumers since the package names are
  preserved.

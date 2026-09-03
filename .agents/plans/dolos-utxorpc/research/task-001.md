# task-001 research

- scope finding: `task-001` is intentionally limited to workspace wiring and a
  minimal `cardano-connector-utxorpc` crate skeleton; it does not implement
  `CardanoConnector` behavior yet
- verification finding: the accepted verification for this task is
  `cargo check -p cardano-connector-utxorpc`,
  `cargo test -p cardano-connector-utxorpc`, and `cargo check --workspace`; no
  fmt, clippy, or docs build was recorded as run
- workflow finding: `.opencode/workflows/rust.md` required using `rust-router`
  first, adding targeted Rust skills, and reporting only verification that
  actually ran
- doc precedence finding: `rust/Cargo.toml` and current workspace crate paths
  are the canonical source for crate names and workspace wiring;
  `rust/README.md` still contains older historical names such as
  `cardano-connect` and `cardano-connect-blockfrost`
- toolchain finding: for this task, the workspace `rust-version = "1.94.0"` in
  `rust/Cargo.toml` supersedes the generic `rust-router` default project setting
  of `1.85`

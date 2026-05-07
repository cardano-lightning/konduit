# task-102 research

- smoke-test seam finding: the smallest truthful autonomous CLI smoke path for
  the integrated UTxO RPC runtime is the admin `send` command path, not a
  tip-only path, because it composes connector network-derived address
  selection, UTxO lookup, live protocol-parameter retrieval, transaction
  building, signing, and submission in one existing runtime flow
- startup-smoke finding: the smallest truthful autonomous server smoke path is a
  successful `admin::Service::new(...)` boot with protocol parameters plus a
  resolvable reference-script UTxO; this proves the documented startup readiness
  composition without inventing a broader BLN or DB harness or pretending to
  exercise live Dolos
- runtime-send finding: the prior `konduit-server` binary build blocker came
  from the UTxO RPC connector paging helper returning a boxed non-`Send` future
  that propagated into the admin background sync task. Requiring that boxed
  paging future to be `Send` is enough to make the runtime path compile cleanly
  again without refactoring the connector trait or crate layering
- runtime-spawn finding: `konduit-server` runs under `actix_web::main`, so the
  admin background task should use `actix_web::rt::spawn` instead of
  `tokio::spawn` for this runtime boundary. That keeps the fix local to the
  server runtime model and avoids unnecessary broad `Send` pressure in unrelated
  paths
- verification finding: after the bounded connector and runtime-boundary fixes,
  the previously blocked verification gap is cleared:
  `cargo check -p konduit-server`, `cargo test -p konduit-server`,
  `cargo clippy -p konduit-server --all-targets -- -D warnings`,
  `cargo check --workspace`, and `cargo test --workspace` all pass truthfully in
  this repository state
- docs cleanup finding: targeted docs verification surfaced an existing rustdoc
  warning in `konduit-cli/src/cmd/adaptor/tx.rs` caused by angle-bracket
  pseudo-markup in a doc comment; converting that line to backticked text was
  sufficient to keep the touched-surface docs pass clean

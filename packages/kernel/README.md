# konduit

This part is written in Aiken. Find more on the
[Aiken's user manual](https://aiken-lang.org).

## Testing

Run the full suite with production signature verification:

```sh
aiken check
```

The payment tests use real, deterministic Ed25519 signatures. Their source
comments describe the fixture encoding and test keys.

The `no_crypto` environment bypasses signature verification. Use it only for
focused accounting diagnostics, not security checks. Tests that reject forged
signatures are expected to fail in that environment.

```sh
aiken check --env no_crypto -m 'konduit/steps/sub.{..}'
```

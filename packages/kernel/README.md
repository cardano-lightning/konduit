# konduit

This part is written in Aiken. Find more on the
[Aiken's user manual](https://aiken-lang.org).

## Testing

Run the kernel tests with `aiken check`. Signed squash and unlocked vectors used
by the Aiken `sub` tests are generated as a complete Aiken module by:

```sh
cargo run -p konduit-core --example sub-vectors > packages/kernel/lib/konduit/steps/sub_vectors.ak
```

The generated module includes the named `SubVector` records consumed by
`lib/konduit/steps/sub_tests.ak`.

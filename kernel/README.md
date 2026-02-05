# konduit

This part is written in Aiken. Find more on the
[Aiken's user manual](https://aiken-lang.org).

## Testing

Some tests will fail unless they are run with `no_cypto`. No crypto skips
signature, and hash verification steps. This allows test data to be created
without an external source.

For example

```
aiken check --env no_crypto -m lib/konduit/steps/sub.{..}
```

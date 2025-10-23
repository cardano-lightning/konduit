# Konduit CLI

> A command-line to construct and navigate Konduit's stages

## Configuring

See [.env.example](.env.example). Replace environment variables by their corresponding values, and rename as `.env`.

## Using

```
cargo run -- --help
```

### open

```
❯ cargo run -- open --help
Open a channel with an adaptor and deposit some funds into it

Usage: konduit-cli open \
    --amount <U64> \
    --consumer <ED25519_PUB> \
    --adaptor <ED25519_PUB> \
    --channel-tag <HEX32> \
    --close-period <DURATION>

Options:
      --amount <U64>
          Quantity of Lovelace to deposit into the channel

      --consumer <ED25519_PUB>
          Consumer's verification key, allowed to *add* funds.

          We also assume that the consumer is opening that channel and paying for it.

          [env: KONDUIT_CONSUMER=]

      --adaptor <ED25519_PUB>
          Adaptor's verification key, allowed to *sub* funds

          [env: KONDUIT_ADAPTOR=]

      --channel-tag <HEX32>
          An (ideally) unique tag to discriminate channels and allow reuse of keys between them

          [env: KONDUIT_CHANNEL_TAG=]

      --close-period <DURATION>
          Minimum time from `close` to `elapse`. You may specify the duration with a unit; for examples: 5s, 27min, 3h

          [env: KONDUIT_CLOSE_PERIOD=24h]

  -h, --help
          Print help (see a summary with '-h')
```

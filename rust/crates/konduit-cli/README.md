# Konduit CLI

> A command-line to construct and navigate Konduit's stages

## Configuring

See [.env.example](.env.example). Replace environment variables by their
corresponding values, and rename as `.env`.

## Using

The CLI is _user-centric_. In fact, there is a distinct CLI for Consumer and
Adaptor. In addition, there is one for admin.

Outputs are one of:

```
[Def] : Default Space separated values. Binary data is hex
csv : comma seperated values. Binary data is hex
json : pretty JSON
cbor : cbor binary
```

```sh
konduit consumer setup key >> .env.consumer
konduit consumer show config
konduit consumer show keytag <tag>
konduit consumer tx --open <tag>,<adaptor>,<close-period>,<amount> --open <tag>,<adaptor>,<close-period>,<amount> --dry-run
konduit consumer tx --open <tag>,<adaptor>,<close-period>,<amount> --open <tag>,<adaptor>,<close-period>,<amount>
konduit consumer show tip
konduit consumer data null-squash <tag>
konduit consumer make lock <secret> # <lock>
konduit consumer make cheque --tag <tag> --index <index> --amount <amount> --timeout <timeout> --lock <lock> --csv # <hexcbor cheque body>,<hex signature>
konduit consumer make cheque --tag <tag> --index <index> --amount <amount> --timeout <timeout> --secret <secret> --csv # <hexcbor cheque body>,<hex signature>
konduit consumer verify secret --secret <secret> --lock <lock>
konduit consumer verify squash --tag <tag> --body <squash> --signature <signature> --csv
konduit consumer make squash --tag <tag> --amount <amount> --index <index> --exclude <exclude>
konduit consumer tx --open <tag>,<adaptor>,<close-period>,<amount> --add <tag>,<amount> --close <tag>
```

```sh
konduit adaptor setup key >> .env
konduit adaptor show constants --csv
konduit adaptor show config
konduit adaptor show tip
konduit adaptor verify squash --keytag <keytag> --body <body> --signature <signature> # <bool>
konduit adaptor verify squash --keytag <keytag> --squash <body> <signature> # <bool>
konduit adaptor verify locked --keytage <keytag> --body <locked> --signature <signature> # <bool>
konduit adaptor verify secret --secret <secret> --lock <lock> # <bool>
konduit adaptor make receipt <squash>;(<locked>);<locked>,<secret>;<locked>,<secret>;
konduit adaptor tx --receipt <receipt> --receipt <receipt>
```

```sh
konduit admin setup key >> .env
konduit admin show config
konduit admin tx deploy
konduit admin tx send --to <address> <amount> --rest <address>
```

It is easy to compose:

```
konduit consumer tx --open <tag>,$(konduit adaptor show constants --csv),<amount>
konduit adaptor verify squash --keytag $(konduit adaptor show keytag <tag>) --squash $(konduit consumer make squash --tag <tag> --amount <amount> ... --csv)
```

### Dotenv

The tool expect envvars for secrets and constants. These can exist via envvars,
but using `.env` is more convenient when driving by hand. The expected envvars
is not identical for the different users. For this reason, the tool allows for
multiple `.env.<user>`, with `.env` used as fallback.

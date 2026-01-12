# Konduit CLI

> A command-line to construct and navigate Konduit's stages

## Using

The CLI is _user-centric_ : Admin, Adaptor, and Consumer.

### Env

The CLI anticipates, but does not require, the usage of dotenv files.

It uses [`dotenvy`](https://github.com/allan2/dotenvy?tab=readme-ov-file) to
load the dotenv files, if they exist. It tries to load `.env.<user>` and then
`.env`. `dotenvy` will only load a variable into env variables if does not
already exist, so env variables declared in the terminal takes precendence over
dotenv files, as too declarations in `.env.<user>` take precendence over those
in `.env`.

With this setup, it is ergonomic to execute commands "as" different users. For
example:

```bash
alias adaptor="konduit consumer"
alias consumer="konduit consumer"
consumer tx --open "$(adaptor show constants --csv),100"
```

There is overlap in variables expected by each user. For example, each user
needs a connector, and the host address of the konduit script. By declaring the
associated values in the `.env` file, these are shared by the users.

If you are running the CLI as a single user, you can simply use `.env`.

If you are running as more than one adaptor, say, you can effectively invoke
another `.env`, for example `.env.other`, by:

```bash
set -a; eval $(sed 's/ = /=/' .env.other); set +a ; konduit ...
```

Note that the generated pretty toml from `adaptor setup >> .env.other` is not
legal INI. The `sed` noise in the above command handles this quirk.

### Commands

Outputs are one of:

```
csv : comma seperated values. Binary data is hex
json : pretty JSON
cbor : cbor binary
```

```sh
konduit admin setup key >> .env
konduit admin show config
konduit admin tx deploy --spend-all
konduit admin tx send --to <address>,<amount> --rest <address> --spend-all
```

```sh
konduit adaptor setup >> .env.adaptor
konduit adaptor show constants
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

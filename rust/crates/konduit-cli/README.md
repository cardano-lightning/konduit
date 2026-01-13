# Konduit CLI

> A command-line to create and manage Konduit channels and payments

## Using

Konduit CLI is initially intended for rudimental testing. However, it should
also be flexible and good enough to permit "real world" usage.

The CLI is _user-centric_ , providing explicit interfaces for Admin, Adaptor,
and Consumer.

### Env

Konduit CLI anticipates, but does not require, the usage of dotenv files.

Variables can be declared directly in the env or in files `.env.<user>` and
`.env`. Presendence of variables is in that order. Under the hood,
[`dotenvy`](https://github.com/allan2/dotenvy?tab=readme-ov-file) is used to
load the dotenv files, if the file exists. Env variables declared in the
terminal takes precendence over dotenv files, and declarations in `.env.<user>`
take precendence over those in `.env`.

With this setup, it is ergonomic to execute commands "as" different users
simultaneously. For example:

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
legal INI. The `sed` noise in the above command handles this quirk. A less noisy
approach is to simply make the file INI complient.

### Commands

Adaptor verify consumer locked cheque:

```bash
adaptor verify locked \
    --keytag $(consumer show keytag deadbeef) \
    --locked \
        $(consumer make locked \
            --tag deadbeef \
            --index 1 \
            --amount 123 \
            --duration 2000s \
            --lock 0000000000000000000000000000000000000000000000000000000000000000 \
        )
```

Outputs are one of:

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
konduit consumer make null-squash <tag>
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

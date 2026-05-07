# Konduit CLI

> A command-line to create and manage Konduit channels and payments

## Overview

Konduit CLI is initially intended for rudimental testing. However, it should
also be flexible and good enough to permit "real world" usage.

The CLI is _user-centric_ , providing explicit interfaces for:

- [consumer](../../../docs/design/11_roles.md#consumer): principle target of the
  application and akin as to the user of a typical traditional application.
  Consumers typically don't use the command-line, but commands exist for the
  sake of playing that role in a local/test setup.

- [adaptor](../../../docs/design/11_roles.md#adaptor): infrastructure operator
  who run (some of) the "back-end services" of Konduit, along side a BLN node.

- [admin](../../../docs/design/11_roles.md#adaptor): administrator of a Konduit
  protocol instance; deploying and administering smart contracts.

### Configuration

Konduit CLI supports config from command-line options, exported env vars,
`.env.<role>`, and `.env`. Each role has shared options defined at the root of
its subcommand group, and there is overlap in the options expected by each user.

Role-specific dotenv loading is a local-dev convenience implemented by the CLI
itself. It is useful for local testing, but production secrets and long-lived
operator config should live outside the repository checkout.

In any case, environment variables exist for each of those options and can be
declared in `.env[.<user>]` files. For example:

<table>
<strong><code>.env.consumer</code></strong>

```.env
KONDUIT_WALLET=329d3e30535349258fa24d8a58f4c376b14cc5504b1a100fbc266019b994ecb6
```

</table>

Environment follows the following precedence rules (variables found in the first
areas takes precedence):

1. command-line options
1. exported env var
1. `.env.<user>`
1. `.env`

Backend-specific config truth:

- parsed `utxorpc` CLI config requires `KONDUIT_NETWORK`.
- live `utxorpc` connector use for commands such as `show tip` and tx flows also
  requires `KONDUIT_UTXORPC_URI`.
- `KONDUIT_CARDANO_BACKEND=blockfrost` requires `KONDUIT_BLOCKFROST_PROJECT_ID`;
  the network can still be inferred from the project id or default to `mainnet`
  in some CLI config paths.
- live reachability and network validation during connector construction are
  currently eager only for the UTxO RPC backend.

`setup` commands print filled configuration to stdout, including sensitive
values such as generated wallet material. Treat that output as secret material.
Redirect it carefully for local development, and do not treat repo-local `.env*`
files as the recommended production secret-management model.

> [!TIP]
>
> It is ergonomic to execute commands "as" different users simultaneously. For
> example:
>
> ```bash
> alias adaptor="konduit adaptor"
> alias consumer="konduit consumer"
>
> consumer tx --open "$(adaptor show constants --csv),100"
> ```

### Scenarios

Here we go through some example scenarios that illustrate how the CLI commands
can be invoked.

Set some aliases:

```bash
alias admin="cargo run -- admin"
alias adaptor="cargo run -- adaptor"
alias conusmer="cargo run -- consumer"
```

#### Admin deploy:

Create a local-dev admin dotenv file. For the current UTxO RPC path, set the
backend explicitly and keep the generated output out of version control.

```sh
konduit admin --backend utxorpc --network preview --utxorpc http://127.0.0.1:1337 setup >> .env.admin
```

For Blockfrost-based local testing, use
`konduit admin --backend blockfrost --blockfrost ... setup` instead. In both
cases, `setup` output is sensitive and should be treated as local-dev bootstrap
material, not production deployment guidance.

Show wallet details

```sh
admin show config
```

Out of band: fund the wallet from external funds

"Deploy" script, ie submit tx with script in reference script of output.

```sh
admin tx deploy
```

See the result

```sh
admin show tip
```

#### Setup Consumer and adaptor

Create dotenv files for participants. Note that `.env` will be read and be
loaded if not overridden by CLI args, or other envvars.

```sh
consumer setup >> .env.consumer
adaptor setup >> .env.adaptor
```

Open the files in an editor and remove the connector and host address entries.
This way, the CLI will fallback to the `.env` file for these values.

Also edit the adaptor file to set env variables.

Send funds from admin:

```sh
admin tx send --to "$(consumer show address),100" --to "$(adaptor show address),10"
```

WARNING :: This is not supposed to spend the reference script UTXO. Double check
that it hasn't!

Current backend notes:

- `admin show config` and `show address` use parsed config and do not require a
  live connector.
- `show tip` and tx commands do construct live connectors.
- with `utxorpc`, those live commands perform eager reachability and network
  validation.
- with the current direct Blockfrost path, validation is limited to project-id
  presence and network-prefix consistency before later API use.

Consumer opens a channels with Adaptor with tag `deadbeef` and `10` Ada (+ min
ada buffer).

```sh
consumer tx --open "deadbeef,$(adaptor show constants),10"
```

Both Adaptor and Consumer can see this:

```sh
consumer show tip
adaptor show tip
```

Adaptor verify consumer squash:

```sh
adaptor verify squash \
    --keytag $(consumer show keytag deadbeef) \
    --squash $(consumer make squash --tag deadbeef  --amount 123 --index 1)
```

#### Add and sub

Adaptor verify consumer locked cheque:

```sh
adaptor verify locked \
    --keytag $(consumer show keytag deadbeef) \
    --locked \
        $(consumer make locked \
            --tag deadbeef \
            --index 1 \
            --amount 123 \
            --duration 2000s \
            --secret 0000000000000000000000000000000000000000000000000000000000000000 \
        )
```

Consumer adds 2 ada to channel

```sh
consumer tx --add deadbeef,2
```

Adaptor subs 3 ada from channel

```sh
export SECRET="0000000000000000000000000000000000000000000000000000000000000000"
adaptor tx --receipt "$(consumer show keytag deadbeef);$(consumer make squash --tag deadbeef --amount 4560000 --index 5);$(consumer make locked --tag deadbeef --index 7 --amount 1000000 --duration 8h --secret $SECRET),$SECRET"
```

## TODO

- [ ] When is responded safe?! It's safe if you sync against the same utxo set
      used in the tx. In this case, it is not possible to respond to the
      retainer (can respond only to closed whereas retainer must be opened).
      This is a downstream problem, that is, it must be correctly handled in the
      konduit-adaptor server.

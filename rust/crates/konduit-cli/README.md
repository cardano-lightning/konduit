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

Create admin .env file. Here we're inserting the project id.

```sh
admin setup --blockfrost "preview..."  >> .env
```

Alternatively open the file and edit manually. It is optional to move the key to
`.env.admin`.

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

Create admin .env file. Here we're inserting the project id.

```sh
consumer setup >> .env.consumer
adaptor setup >> .env.adaptot
```

Open the files in an editor and remove the connector and host address entries.
This way, the CLI will fallback to the `.env` file for these values.

Also edit the adaptor file to set env variables.

Send funds from admin:

```sh
admin send --to "$(consumer show address),100" --to "$(adaptor show address),10"
```

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

Adaptor verify consumer locked cheque:

```sh
adaptor verify squash \
    --keytag $(consumer show keytag deadbeef) \
    --locked \
        $(consumer make squash \
            --amount 123 \
            --index 1 \
        )
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

```
consumer tx --add deadbeef,2
```

Adaptor subs 3 ada from channel

```
adaptor tx --receipt "$(consumer show keytag deadbeef);$(consumer make squash --amount 3000000 --index 3)"
```

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

### Channel asset catalog

Consumer commands accept `--asset-config PATH` or `KONDUIT_ASSET_CONFIG=PATH`.
The file is a JSON array appended to the built-in `ada`, `usdm`, `usdcx`, and
`usda` definitions:

```json
[
  {
    "alias": "snek",
    "asset": {
      "kind": "native",
      "policy_id": "00000000000000000000000000000000000000000000000000000000",
      "asset_name": "534e454b"
    },
    "decimals": 0,
    "pricing": { "kind": "coin_gecko", "coin_id": "snek" }
  }
]
```

| Alias   | Policy ID                                                  | Asset name         | Decimals | Pricing               |
| ------- | ---------------------------------------------------------- | ------------------ | -------: | --------------------- |
| `ada`   | —                                                          | —                  |        6 | Ada/USD provider rate |
| `usdm`  | `c48cbb3d5e57ed56e276bc45f99ab39abe94e6cd7ac39fb402da47ad` | `0014df105553444d` |        6 | USD 1                 |
| `usdcx` | `1f3aec8bfe7ea4fe14c5f121e2a92e301afe414147860d557cac7e34` | `5553444378`       |        6 | USD 1                 |
| `usda`  | `fe7c786ab321f41c654ef6c1af7b3250a613c24e4213e0425a7ae456` | `55534441`         |        6 | USD 1                 |

Native policy IDs are 56 lowercase hex characters; asset names are 0–64
lowercase hex characters. Aliases use 1–32 lowercase letters, digits, `_`, or
`-`; decimals are `0..=19`. Pricing is either `{"kind":"usd_peg"}` or
`{"kind":"coin_gecko","coin_id":"..."}`. Custom variable assets require the
CoinGecko FX provider. Built-in USDM, USDCx, and USDA are fixed at USD 1 and do
not request external rates.

`--open TAG,ADAPTOR_KEY,CLOSE_PERIOD,AMOUNT` opens Ada. Append an alias to select
another asset:

```sh
consumer tx --open "deadbeef,$(adaptor show constants),10,usdm"
consumer tx --asset-config assets.json \
  --open "cafe,$(adaptor show constants),25,snek"
```

CLI open/add amounts are whole displayed units and are scaled by the catalog
decimals. Quote, cheque, WASM, and protocol amounts are raw asset units. The
consumer wallet must already contain the selected native asset; fund custom
assets with an external Cardano wallet.

This release is a clean validator/database cutover:

1. Close and settle every legacy Ada channel with the old binary.
2. Stop the old service.
3. Deploy the generated generic reference script with
   `konduit-cli admin tx deploy`; update the host address if it changed.
4. Start the server with a fresh `KONDUIT_DB_PATH`,
   `FX_BASE_CURRENCY=usd`, and the same asset catalog used by consumer CLI
   processes.
5. Open new Ada, USDM, USDCx, USDA, or configured custom channels.

Old and new validators must not run concurrently.

`KONDUIT_HOST_ADDRESS` is both the deployment destination and the address that
the server queries at startup. It does not need to be controlled by the admin
wallet. Deploying to a new host does not spend or remove reference scripts at
previous host addresses.

Current mainnet deployment (2026-09-02):

- host: `addr1vy9z4llh8hxdwc54c0xlfgeza39vqm3zua4zva4elp0quqcxa7mjc`
- output: `e40cd245f1cc4b8f8be7f7e676df94f8f70ae5cd6e205ad5820a6f7d1eed66c3#0`
- Plutus version: V3
- script hash: `8cc6bbaeed22c253b9d703d39f63b7e215f7af08bda930ac6b85ebaf`

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

Confirm the parsed network and host before submitting:

```sh
admin show config
admin show tip --verbose
```

Fund the admin wallet out of band, then deploy the reference script:

```sh
admin tx deploy
```

Do not use `--spend-all` to clean up an unrelated previous host. Deployment
inputs come from the admin wallet payment credential; outputs at another
credential require that credential's signing key.

After confirmation, require `admin show tip --verbose` to report the deployment
transaction output, `script ver: v3`, and the script hash embedded by
`konduit-tx`:

```sh
admin show tip --verbose
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

Consumer opens an Ada channel with Adaptor using tag `deadbeef` and `10` Ada.
The validator also requires its minimum-Ada reserve:

```sh
consumer tx --open "deadbeef,$(adaptor show constants),10"
```

To open a built-in stablecoin channel, append its alias:

```sh
consumer tx --open "cafe,$(adaptor show constants),10,usdm"
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

Consumer adds 2 Ada to the Ada channel. Adds always name the expected asset so
an unrelated same-tag output cannot select a different currency:

```sh
consumer tx --add deadbeef,2,ada
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

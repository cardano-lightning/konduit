# API Reference

_Auto-generated from source using `syn`._

All wire types support JSON (`serde`) and CBOR (`minicbor`) serialization.
Errors use the Problem Details convention.

---

## Endpoints

| Path                            | Module                            | Method | Auth |
| ------------------------------- | --------------------------------- | ------ | ---- |
| `/auth/pay/bln_template/commit` | `auth::pay::bln_template::commit` | `POST` | Yes  |
| `/auth/pay/bln_template/quote`  | `auth::pay::bln_template::quote`  | `POST` | Yes  |
| `/auth/pay/bolt11/commit`       | `auth::pay::bolt11::commit`       | `POST` | Yes  |
| `/auth/pay/bolt11/quote`        | `auth::pay::bolt11::quote`        | `POST` | Yes  |
| `/auth/pay/cln_template/commit` | `auth::pay::cln_template::commit` | `POST` | Yes  |
| `/auth/pay/cln_template/quote`  | `auth::pay::cln_template::quote`  | `POST` | Yes  |
| `/auth/squash`                  | `auth::squash`                    | `POST` | Yes  |
| `/auth/state`                   | `auth::state`                     | `GET`  | Yes  |
| `/info`                         | `info`                            | `GET`  | No   |
| `/reg`                          | `reg`                             | `POST` | No   |

---

## `auth`

Channel partners, ie users, require auth.
User must have registered `./reg`.
They then use the (auth) scheme credential.

For HTTP, auth uses standard conveciton of `Authorization` Header,
albeit with a non-standarda scheme.

Header must be of the of the form `Authorization: <scheme> <credential>`.
Credentials are base64 encoded. See `./reg` for details.

### `AuthError`

| Variant   | Slug                 | Status | Description                                                       |
| --------- | -------------------- | ------ | ----------------------------------------------------------------- |
| `None`    | `no-credential`      | 400    | Credential required, none found or could not be sensibly coerced. |
| `Invalid` | `invalid-credential` | 400    | Credential invalid. Try registration                              |

### `Error`

| Variant  | Slug                        | Status | Description |
| -------- | --------------------------- | ------ | ----------- |
| `Auth`   | _delegates to `AuthError`_  | —      |             |
| `Limit`  | _delegates to `LimitError`_ | —      |             |
| `Domain` | _delegates to `T`_          | —      |             |

### Submodules

- [`auth::pay`](#authpay)
- [`auth::squash`](#authsquash)
- [`auth::state`](#authstate)

---

## `auth::pay`

The pay process is a sequence of steps.

Generally the flow is as follows:

- `quote`: the user requests a `quote` to determine whether a payment is possible,
  and if so, the conditions of servicing the payment such as the amount, and timeout.
  The quote is valid for a short amount of time, and is non-binding.
  It may be rejected due to change in circumstances.

- `commit`: the user commits to the payment. On the happy path, a commitment
  is swiftly resolved, and the response is the corresponding secret.
  Otherwise the associated funds cannot be otherwise spent (or committed).

The pay flow follows a "scheme", which identifies with a particular spec.
For exampl, and most notably, `bolt11` aka Bitcoin Lightning invoice.
However, schemes are completely uncoupled. It is forseeable that some
servers offer only a subset of schemes, while some clients only support
a subset of schemes.

### Submodules

- [`auth::pay::bln_template`](#authpaybln_template)
- [`auth::pay::bolt11`](#authpaybolt11)
- [`auth::pay::cln_template`](#authpaycln_template)
- [`auth::pay::common`](#authpaycommon)

---

## `auth::pay::bln_template`

### Submodules

- [`auth::pay::bln_template::commit`](#authpaybln_templatecommit)
- [`auth::pay::bln_template::quote`](#authpaybln_templatequote)

---

## `auth::pay::bln_template::commit`

**`POST /auth/pay/bln_template/commit`**

### Request Body

| Field            | Type                | CBOR | JSON encoding                | Description |
| ---------------- | ------------------- | ---- | ---------------------------- | ----------- |
| `cheque`         | `Locked`            | 0    | —                            |             |
| `payment_secret` | `Option<[u8 ; 32]>` | 1    | Option<serde_with::hex::Hex> |             |

### Response

Alias for `crate :: auth :: pay :: common :: quote :: ChequeProposal`.

**Error:** `crate :: auth :: Error<DomainError>` where domain = `crate :: auth :: pay :: common :: commit :: Error`

---

## `auth::pay::bln_template::quote`

BLN template: quote without a BOLT-11 invoice.

The naming convention follows BOLT-11 spec.

The client specifies the payment parameters directly.
The lock (`r_hash`) will be taken from the cheque.
Using the template method allows a new class of payment failure:
user error (lock mismatch).

If `final_cltv` is None, a server default is used.
We follow the naming and structure of the spec.

**`POST /auth/pay/bln_template/quote`**

### Request Body

| Field            | Type                | CBOR | JSON encoding                | Description                                                                                                                     |
| ---------------- | ------------------- | ---- | ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `payee`          | `[u8 ; 33]`         | 0    | serde_with::hex::Hex         |                                                                                                                                 |
| `amount_msat`    | `u64`               | 1    | —                            |                                                                                                                                 |
| `r_hash`         | `[u8 ; 32]`         | 2    | serde_with::hex::Hex         |                                                                                                                                 |
| `final_cltv`     | `Option<u64>`       | 3    | —                            | If not included, assume default                                                                                                 |
| `route_hints`    | `Vec<RouteHint>`    | 4    | —                            |                                                                                                                                 |
| `payment_secret` | `Option<[u8 ; 32]>` | 5    | Option<serde_with::hex::Hex> | The payment secret is an anti-probing defense. Not to be confused with the r_preimage which in Konduit we refer to as `Secret`. |

### Response

Alias for `crate :: auth :: pay :: common :: quote :: ChequeProposal`.

### RouteHint

We restate RouteHint with all derivations.
A route hint included in a BOLT11 invoice, describing a path of hops
the payer can use to reach the recipient.

An invoice may contain multiple [`RouteHint`]s, giving the payer
several candidate paths. Each hint is a sequence of [`RouteHintHop`]s
to traverse in order.

### RouteHintHop

A single hop within a [`RouteHint`], describing a channel that can
be used to reach the recipient of a BOLT11 invoice.

| Field               | Type          | CBOR | JSON encoding        | Description                                                          |
| ------------------- | ------------- | ---- | -------------------- | -------------------------------------------------------------------- |
| `src_node_id`       | `[u8 ; 33]`   | 0    | serde_with::hex::Hex | The node through which this hop routes.                              |
| `short_channel_id`  | `u64`         | 1    | —                    | The channel to use for this hop, identified by its short channel ID. |
| `fees`              | `RoutingFees` | 2    | —                    | Fees charged by this hop for forwarding an HTLC.                     |
| `cltv_expiry_delta` | `u16`         | 3    | —                    | The CLTV delta added by this hop, in blocks.                         |
| `htlc_minimum_msat` | `Option<u64>` | 4    | —                    | The minimum HTLC size this hop will forward, in millisatoshis.       |
| `htlc_maximum_msat` | `Option<u64>` | 5    | —                    | The maximum HTLC size this hop will forward, in millisatoshis.       |

### RoutingFees

Fees charged by a hop when routing a payment.

Used within [`RouteHintHop`] to describe the cost of forwarding
an HTLC through a particular channel.

```math
fee = base_msat + (amount_msat * proportional_millionths / 1_000_000)
```

| Field                     | Type  | CBOR | Description                                                                                                 |
| ------------------------- | ----- | ---- | ----------------------------------------------------------------------------------------------------------- |
| `base_msat`               | `u32` | 0    | Flat fee, in millisatoshis, charged for forwarding any HTLC.                                                |
| `proportional_millionths` | `u32` | 1    | Proportional fee, in millionths of the HTLC amount, charged for forwarding. For example, `1000` means 0.1%. |

**Error:** `crate :: auth :: Error<DomainError>` where domain = `crate :: auth :: pay :: common :: quote :: Error`

---

## `auth::pay::bolt11`

### Submodules

- [`auth::pay::bolt11::commit`](#authpaybolt11commit)
- [`auth::pay::bolt11::quote`](#authpaybolt11quote)

---

## `auth::pay::bolt11::commit`

**`POST /auth/pay/bolt11/commit`**

### Request Body

Alias for `Locked`.

### Response

Alias for `crate :: auth :: pay :: common :: quote :: ChequeProposal`.

**Error:** `crate :: auth :: Error<DomainError>` where domain = `crate :: auth :: pay :: common :: commit :: Error`

---

## `auth::pay::bolt11::quote`

BOLT-11 aka invoice.

**`POST /auth/pay/bolt11/quote`**

### Request Body

Body is just invoice: the bech32 encoding string as it appears to the user. No funny business.
Alias for `String`.

### Response

Alias for `crate :: auth :: pay :: common :: quote :: ChequeProposal`.

**Error:** `crate :: auth :: Error<DomainError>` where domain = `crate :: auth :: pay :: common :: quote :: Error`

---

## `auth::pay::cln_template`

### Submodules

- [`auth::pay::cln_template::commit`](#authpaycln_templatecommit)
- [`auth::pay::cln_template::quote`](#authpaycln_templatequote)

---

## `auth::pay::cln_template::commit`

**`POST /auth/pay/cln_template/commit`**

### Request Body

Alias for `Locked`.

### Response

Alias for `crate :: auth :: pay :: common :: quote :: ChequeProposal`.

**Error:** `crate :: auth :: Error<DomainError>` where domain = `crate :: auth :: pay :: common :: commit :: Error`

---

## `auth::pay::cln_template::quote`

CLN template: a bare bones pay request for Cardano Lightning.

**`POST /auth/pay/cln_template/quote`**

### Request Body

| Field    | Type        | CBOR | JSON encoding        | Description                                                                      |
| -------- | ----------- | ---- | -------------------- | -------------------------------------------------------------------------------- |
| `payee`  | `[u8 ; 32]` | 0    | serde_with::hex::Hex | Payee: their ed25519 verifying key                                               |
| `amount` | `u64`       | 1    | —                    | Amount is in the unit currency. For example: lovelace for an Ada backed channel. |
| `lock`   | `Lock`      | 2    | —                    | FIXME :: Move to `order`. The lock                                               |

### Response

Alias for `crate :: auth :: pay :: common :: quote :: ChequeProposal`.

**Error:** `crate :: auth :: Error<DomainError>` where domain = `crate :: auth :: pay :: common :: quote :: Error`

---

## `auth::pay::common`

### Submodules

- [`auth::pay::common::commit`](#authpaycommoncommit)
- [`auth::pay::common::quote`](#authpaycommonquote)

---

## `auth::pay::common::commit`

### `CommittedError`

| Variant    | Slug               | Status | Description                     |
| ---------- | ------------------ | ------ | ------------------------------- |
| `BadRoute` | `bad-route`        | 400    | A failure occured on the route. |
| `Other`    | `postcommit-other` | 400    | Something went wrong            |

### `Error`

| Variant       | Slug                              | Status | Description                                                                                                                         |
| ------------- | --------------------------------- | ------ | ----------------------------------------------------------------------------------------------------------------------------------- |
| `Uncommitted` | _delegates to `UncommittedError`_ | —      | An error occurred either before server commitment or commitment is unwound. Server, _should_ accept reuse of index.                 |
| `Committed`   | _delegates to `CommittedError`_   | —      | An error occurred after server commitment, and without graceful resolution ie unwinding. Associated funds are locked until timeout. |

### `UncommittedError`

| Variant    | Slug              | Status | Description                                     |
| ---------- | ----------------- | ------ | ----------------------------------------------- |
| `Stale`    | `commit-stale`    | 400    | The quote on which the commit is based is stale |
| `Mismatch` | `commit-mismatch` | 400    | The quote does not match the commit             |
| `NoRoute`  | `no-route`        | 400    | A route no longer exists.                       |
| `Other`    | `precommit-other` | 400    | Something went wrong                            |

---

## `auth::pay::common::quote`

### ChequeProposal

The common response is a ChequeProposal

| Field              | Type  | CBOR | Description                                                                                                                      |
| ------------------ | ----- | ---- | -------------------------------------------------------------------------------------------------------------------------------- |
| `index`            | `u64` | 0    | Cheque index                                                                                                                     |
| `amount`           | `u64` | 1    | Cheque amount                                                                                                                    |
| `relative_timeout` | `u64` | 2    | Cheque timeout. Note that this is **relative** and in ms                                                                         |
| `fee`              | `u64` | 3    | Routing fee. This is informational - the import value is the `amount`. Clients should independently calculate the effective fee. |

### `Error`

| Variant    | Slug                 | Status | Description                                                                                                                                                                                                 |
| ---------- | -------------------- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Backing`  | `no-backing`         | 400    | There no backing. This may be because none exists on-chain or server is not serving this channel                                                                                                            |
| `Squash`   | `no-squash`          | 400    | The channel has no squash.                                                                                                                                                                                  |
| `Capacity` | `no-capacity`        | 400    | Channel has no room for more cheques: too many unresolved payments or in-flight cheques. Client must submit a squash to free capacity before retrying, or may have to wait until in-flight cheques timeout. |
| `Funds`    | `insufficient-funds` | 400    | The backing has insufficient funds                                                                                                                                                                          |
| `Route`    | `no-route`           | 400    | No route found                                                                                                                                                                                              |
| `Size`     | `max-size`           | 400    | Request payload exceeds the maximum size.                                                                                                                                                                   |

---

## `auth::squash`

**`POST /auth/squash`**

### Request Body

_(unit type — no fields)_

### SquashProposal

| Field       | Type            | CBOR | Description |
| ----------- | --------------- | ---- | ----------- |
| `current`   | `Squash`        | 0    |             |
| `unlockeds` | `Vec<Unlocked>` | 1    |             |
| `proposal`  | `SquashBody`    | 2    |             |

### `DomainError`

| Variant         | Slug             | Status | Description                                                      |
| --------------- | ---------------- | ------ | ---------------------------------------------------------------- |
| `InvalidSquash` | `invalid-squash` | 400    | Something is wrong with the data, for example invalid signature. |

**Error:** `super :: Error<DomainError>`

---

## `auth::state`

**`GET /auth/state`**

### Response

| Field     | Type      | CBOR | Description                                                                                                       |
| --------- | --------- | ---- | ----------------------------------------------------------------------------------------------------------------- |
| `backing` | `Backing` | 0    | This may be the case for numerous reasons. For example, the channel was closed or server no longer recognizes it. |
| `receipt` | `Receipt` | 1    |                                                                                                                   |

### Amounts

TODO:: More explicit term desired, but this is literally "amounts".

| Field     | Type  | CBOR | Description                           |
| --------- | ----- | ---- | ------------------------------------- |
| `subbed`  | `u64` | 0    | (Total) subbed according to the datum |
| `balance` | `u64` | 1    | Amount of asset in utxo               |

### Backing

Backing is treated as purely informational.
This value may be cached; it may be stale at the
point at which the user requests a quote

| Field     | Type              | CBOR | Description                                                                                                       |
| --------- | ----------------- | ---- | ----------------------------------------------------------------------------------------------------------------- |
| `settled` | `Option<Amounts>` | 0    | Amount that server deems confirmed on-chain, and is backing the channel. None indicates channel is not available. |
| `pending` | `Option<Amounts>` | 1    | Amount that is seen on-chain but not yet settled. This can alleviate some UX issues                               |

### Params

TODO:: Do we want query params?

### Receipt

| Field     | Type          | CBOR | Description |
| --------- | ------------- | ---- | ----------- |
| `squash`  | `Squash`      | 0    |             |
| `cheques` | `Vec<Cheque>` | 1    |             |

### `DomainError`

| Variant | Slug          | Status | Description                    |
| ------- | ------------- | ------ | ------------------------------ |
| `Other` | `state-other` | 400    | FIXME :: Something went wrong. |

**Error:** `super :: Error<DomainError>`

---

## `info`

**`GET /info`**

### Response

| Field                | Type                | CBOR | Description                            |
| -------------------- | ------------------- | ---- | -------------------------------------- |
| `tos`                | `TosInfo`           | 0    | Terms of service. Purely informational |
| `channel_parameters` | `ChannelParameters` | 1    | Channel parameters                     |
| `tx_help`            | `TxHelp`            | 2    | Tx building                            |

### ChannelParameters

| Field          | Type           | CBOR | JSON encoding        | Description |
| -------------- | -------------- | ---- | -------------------- | ----------- |
| `adaptor_key`  | `VerifyingKey` | 0    | serde_with::hex::Hex |             |
| `close_period` | `Duration`     | —    | —                    |             |
| `tag_length`   | `usize`        | 2    | —                    |             |

### TosInfo

| Field      | Type  | CBOR | Description |
| ---------- | ----- | ---- | ----------- |
| `flat_fee` | `u64` | 0    |             |

### TxHelp

| Field          | Type               | CBOR | JSON encoding              | Description |
| -------------- | ------------------ | ---- | -------------------------- | ----------- |
| `host_address` | `Address<Shelley>` | —    | serde_with::DisplayFromStr |             |
| `validator`    | `Hash<28>`         | 1    | serde_with::hex::Hex       |             |

---

## `limit`

Resources are limited.

### `LimitError`

| Variant   | Slug            | Status | Description                                                                |
| --------- | --------------- | ------ | -------------------------------------------------------------------------- |
| `Reached` | `limit-reached` | 400    | Resource limit reached. Cannot carryout additional action. User must wait. |

---

## `reg`

**Registration**

There may be different _schemes_ for registration.
Regardless of the scheme, registration is required to send the initial squash.
Re-registration maybe required on, for example, an expiration of a auth token.

An instance should support only one scheme.

A scheme must specify:

- request body token
- an error type
- scheme name as appears in header (follow the existing conventions of RFC 7235)
- credential as appears in header. Should be base64 encoded.

**`POST /reg`**

### Request Body<T>

Request body

| Field    | Type             | CBOR | Description                                    |
| -------- | ---------------- | ---- | ---------------------------------------------- |
| `token`  | `T`              | 0    | Token depends on the authorization scheme used |
| `squash` | `Option<Squash>` | 1    | The initial handshake must include a squash    |

### `CommonError`

| Variant     | Slug                        | Status | Description                                                                                                                                                             |
| ----------- | --------------------------- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Limit`     | _delegates to `LimitError`_ | —      |                                                                                                                                                                         |
| `NoChannel` | `no-channel`                | 400    | This can occur if the server has not seen the channel on-chain, or has removed the channel from the db. In either case, the server is not treating the channel as live. |
| `Input`     | `bad-input`                 | 400    | Bad input, for example invalid signature                                                                                                                                |
| `Squash`    | `no-channel`                | 400    | Squash required. Particularly on initial registration                                                                                                                   |

### Submodules

- [`reg::cobbl3`](#regcobbl3)
- [`reg::no_auth`](#regno_auth)

---

## `reg::cobbl3`

Defines the concrete [`Body`] implementation for the cobbl3 HMAC-BLAKE3
auth protocol.

The server crate implements [`cobbl3::Verify`] for [`Body`] using cryptoxide.

Beware! There is mild divergence between what is a "token" between here and Cobbl3.

### Request Body

Request
Alias for `super :: Body<cobbl3 :: Request<TokenBody>>`.

### Response

Response

### TokenBody

The payload the client signs and the server MACs.

`key` and `tag` together identify a channel.
`ttl` is an absolute POSIX timestamp in milliseconds bounding the
validity window of the auth request.

Conversion to/from `konduit-data` types (`VerificationKey`, channel tag)
happens at the server layer — the wire crate treats both as plain bytes.

| Field | Type        | CBOR | Description                            |
| ----- | ----------- | ---- | -------------------------------------- |
| `key` | `[u8 ; 32]` | 0    | Consumer key                           |
| `tag` | `Vec<u8>`   | 1    | Channel tag                            |
| `ttl` | `u64`       | 2    | Expiry as absolute POSIX milliseconds. |

### `Error`

| Variant  | Slug                                  | Status | Description |
| -------- | ------------------------------------- | ------ | ----------- |
| `Common` | _delegates to `super :: CommonError`_ | —      |             |
| `Cobbl3` | _delegates to `cobbl3 :: Error`_      | —      |             |

---

## `reg::no_auth`

Register with no auth.

The server responds without verification.
No funds at risk, but potential leaking of `/state`,
and risk of spamming.

Keytage

### Request Body

Request
Alias for `super :: Body<Keytag>`.

### Response

Response

### Keytag

Keytag bytes carried in [`HEADER`].

Encodes `key || tag` — the server splits and interprets them.
`Display` encodes to base64url (no padding); `FromStr` decodes.

### `Error`

| Variant  | Slug                                  | Status | Description |
| -------- | ------------------------------------- | ------ | ----------- |
| `Common` | _delegates to `super :: CommonError`_ | —      |             |

---

## `version`

### Response

Version should work for _all_ clients with _all_ servers.
Thus `Response` datatype should _never_ change.
All other endpoints may be entirely different, but
at least a client will be able to establish incompatibility from the version.

| Field           | Type                            | CBOR | Description                                                                                                                                                                                                                                                                                                                                                                      |
| --------------- | ------------------------------- | ---- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `flavor`        | `String`                        | 0    | Support diversity in versioning. If you want to make a new version, make the flavour distinct and identifiable. The default flavour is `default`                                                                                                                                                                                                                                 |
| `release`       | `SemVer`                        | 1    | Bumped on any breaking change. At clients discression how to proceed. This provides a friendly way to navigate to the correct docs. It is much more flexible than `protocol_hash`                                                                                                                                                                                                |
| `protocol_hash` | `String`                        | 2    | Content hash of protocol-relevant crates. Stable across monorepo changes: changes only when wire behavior can change. More refined than git hash, but also much more complicated.                                                                                                                                                                                                |
| `vcs_hash`      | `VcsHash`                       | 3    | Git hash for cross-referencing with VCS. Simpler than `protocol_hash` and more useful when needing to checkout the source code. However it is also noisier: irrelevant changes will be picked up.                                                                                                                                                                                |
| `features`      | `BTreeMap<String, FeatureInfo>` | 4    | What features are supported.                                                                                                                                                                                                                                                                                                                                                     |
| `docs_base_url` | `Option<String>`                | 5    | Base URL for hosted documentation for this exact release. Constructed as `<host>/docs/<vcs_hash>` at build time. Append `doc_path` (with "docs/" stripped and ".md" removed) to get the web URL. Example: "https://myorg.github.io/myrepo/abc1234f" Self-hosters set this to their own docs server. In Unknown/Dirty builds this may be empty — use git or local access instead. |

### FeatureInfo

| Field         | Type       | CBOR | JSON encoding        | Description                                                                                                                                                                                                                                                                                                                                                                                                                 |
| ------------- | ---------- | ---- | -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `version`     | `SemVer`   | 0    | —                    | Bumped on any breaking change. At clients discression how to proceed. Versioning at the feature level allows for extensions to be worked on independently                                                                                                                                                                                                                                                                   |
| `schema_hash` | `[u8 ; 8]` | 1    | serde_with::hex::Hex | Truncated SHA-256 (8 bytes) over the canonical schema definition. Fast mismatch detection across independently-versioned features. Not collision-resistant — use `version` for authoritative compatibility checks.                                                                                                                                                                                                          |
| `doc_path`    | `String`   | 2    | —                    | Path to the human-readable spec, relative to VCS root. Always of the form "docs/spec/<feature>.md". Three access methods, all mechanically derived from this path: git: `git show <vcs_hash>:doc_path` local: `docs/book/spec/<feature>.html` (after `mdbook build`) web: `<Response::docs_base_url>/spec/<feature>` The convention "docs/spec/_.md" → "/spec/_" must be preserved for all three to stay in correspondence. |
| `source_path` | `String`   | 3    | —                    | Path to the canonical type definition in the _protocol_ crate, relative to VCS root. This is the wire contract — not the downstream implementation. `git show <vcs_hash>:source_path` Example: "crates/konduit-api/src/endpoints/channel/sync.rs"                                                                                                                                                                           |

### SemVer

| Field   | Type  | CBOR | Description                   |
| ------- | ----- | ---- | ----------------------------- |
| `major` | `u16` | 0    | Bumped on any breaking change |
| `minor` | `u16` | 1    | Bumped on additive change     |
| `patch` | `u16` | 2    | Bumped on bug fix/ patch      |

---

## Shared Error Types

Channel partners, ie users, require auth.
User must have registered `./reg`.
They then use the (auth) scheme credential.

For HTTP, auth uses standard conveciton of `Authorization` Header,
albeit with a non-standarda scheme.

Header must be of the of the form `Authorization: <scheme> <credential>`.
Credentials are base64 encoded. See `./reg` for details.

### `auth::AuthError`

| Variant   | Slug                 | Status | Description                                                       |
| --------- | -------------------- | ------ | ----------------------------------------------------------------- |
| `None`    | `no-credential`      | 400    | Credential required, none found or could not be sensibly coerced. |
| `Invalid` | `invalid-credential` | 400    | Credential invalid. Try registration                              |

### `auth::Error`

| Variant  | Slug                        | Status | Description |
| -------- | --------------------------- | ------ | ----------- |
| `Auth`   | _delegates to `AuthError`_  | —      |             |
| `Limit`  | _delegates to `LimitError`_ | —      |             |
| `Domain` | _delegates to `T`_          | —      |             |

### `auth::pay::common::commit::CommittedError`

| Variant    | Slug               | Status | Description                     |
| ---------- | ------------------ | ------ | ------------------------------- |
| `BadRoute` | `bad-route`        | 400    | A failure occured on the route. |
| `Other`    | `postcommit-other` | 400    | Something went wrong            |

### `auth::pay::common::commit::Error`

| Variant       | Slug                              | Status | Description                                                                                                                         |
| ------------- | --------------------------------- | ------ | ----------------------------------------------------------------------------------------------------------------------------------- |
| `Uncommitted` | _delegates to `UncommittedError`_ | —      | An error occurred either before server commitment or commitment is unwound. Server, _should_ accept reuse of index.                 |
| `Committed`   | _delegates to `CommittedError`_   | —      | An error occurred after server commitment, and without graceful resolution ie unwinding. Associated funds are locked until timeout. |

### `auth::pay::common::commit::UncommittedError`

| Variant    | Slug              | Status | Description                                     |
| ---------- | ----------------- | ------ | ----------------------------------------------- |
| `Stale`    | `commit-stale`    | 400    | The quote on which the commit is based is stale |
| `Mismatch` | `commit-mismatch` | 400    | The quote does not match the commit             |
| `NoRoute`  | `no-route`        | 400    | A route no longer exists.                       |
| `Other`    | `precommit-other` | 400    | Something went wrong                            |

### `auth::pay::common::quote::Error`

| Variant    | Slug                 | Status | Description                                                                                                                                                                                                 |
| ---------- | -------------------- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Backing`  | `no-backing`         | 400    | There no backing. This may be because none exists on-chain or server is not serving this channel                                                                                                            |
| `Squash`   | `no-squash`          | 400    | The channel has no squash.                                                                                                                                                                                  |
| `Capacity` | `no-capacity`        | 400    | Channel has no room for more cheques: too many unresolved payments or in-flight cheques. Client must submit a squash to free capacity before retrying, or may have to wait until in-flight cheques timeout. |
| `Funds`    | `insufficient-funds` | 400    | The backing has insufficient funds                                                                                                                                                                          |
| `Route`    | `no-route`           | 400    | No route found                                                                                                                                                                                              |
| `Size`     | `max-size`           | 400    | Request payload exceeds the maximum size.                                                                                                                                                                   |

Resources are limited.

### `limit::LimitError`

| Variant   | Slug            | Status | Description                                                                |
| --------- | --------------- | ------ | -------------------------------------------------------------------------- |
| `Reached` | `limit-reached` | 400    | Resource limit reached. Cannot carryout additional action. User must wait. |

Defines the concrete [`Body`] implementation for the cobbl3 HMAC-BLAKE3
auth protocol.

The server crate implements [`cobbl3::Verify`] for [`Body`] using cryptoxide.

Beware! There is mild divergence between what is a "token" between here and Cobbl3.

### `reg::cobbl3::Error`

| Variant  | Slug                                  | Status | Description |
| -------- | ------------------------------------- | ------ | ----------- |
| `Common` | _delegates to `super :: CommonError`_ | —      |             |
| `Cobbl3` | _delegates to `cobbl3 :: Error`_      | —      |             |

Register with no auth.

The server responds without verification.
No funds at risk, but potential leaking of `/state`,
and risk of spamming.

Keytage

### `reg::no_auth::Error`

| Variant  | Slug                                  | Status | Description |
| -------- | ------------------------------------- | ------ | ----------- |
| `Common` | _delegates to `super :: CommonError`_ | —      |             |

---
title: Scenarios
---

The following scenarios provide in plain language a brief outline of the context
of users and their intentions in relation to their interactions Konduit.

# Consumer Payment

This scenario is the PPP in context. Consumer wishes to acquire goods, or
services, using Ada. Merchant uses BLN, but need not know about Cardano or
Konduit in any way.

- Consumer walks into a shop, selects item, and hands them to Merchant.
- Merchant creates Bitcoin lightning QR code invoice from items and presents
  this to Consumer.
- Consumer scans invoice, and "consent screen" opens on Konduit App.
- Consent screen displays info including:
  - the amount of ada Consumer pays for transaction
  - the recipient address
  - the recipient amount
  - the effective fee
  - the apparent exchange rate
- Consumer gives or withholds consent.
- On consent, Merchant receives payment.

The analogous scenario holds for a digital store, with a QR code presented at
checkout. Payments need to be fast. Konduit should introduce an insignificant
overhead to the BLN payment in terms of time and cost.

# Consumer Wallet Setup

The App supports a single key, used for both an embedded wallet, and for the
channel credentials. It seems easier/ necessary to have an embedded wallet,
rather than try to interface with an existing one.

- Consumer "installs" Konduit App, and opens App.
- App without a key loaded invites a Consumer to load key.
- Either:
  - "Create" new key
  - "Import" existing key
- Consumer can export key in a safe way, to be imported later.
- Consumer needs to an Cardano Connector, that ought to be distinct from the
  channel partner. However, the friction introduced by requiring this is
  possibly too high for some Consumers. Consumer is able, and encouraged, to
  provide alternative L1 sources.
- Once loaded, App syncs via Cardano Connector. There is some progress
  indication is given while this takes place, particularly for "Import".
- Consumer can then see current balance in wallet.

# Consumer Settings

Consumer settings include data sources, and wallet management

- Consumer can set Cardano Connector.
- Consumer can set a stake credential. App informs Consumer stake credentials
  are used for new Channels.
- Change theme (default to device theme).

# Consumer Wallet Management

- Consumer can view embedded wallet address, current funds, and transactions
  involving wallet (available from the Cardano Connector).
- Status of information is shown (_eg_ "last updated 30s ago").
- Consumer can withdraw any amount of funds.
- Consumer can Export key.
- Consumer can "Forget" wallet, this removes all data from the App.

Any moving of funds, changing of credentials, and forgetting requires Consumer
confirmation.

# Consumer Channel Open

Initial versions of the App support only a single Channel. For the sake of
prudence, proceed assuming that later versions will support multiple channels.
Once a wallet is initialized, but prior to an existing Channel.

- Consumer adds Adaptor details via either:
  - Consumer scans QR code from, say, Adaptor landing page (discovery is OOB).
  - Consumer inputs details manually.
- If channel to operator exists, display channel.
- Else, displays details to open new channel.
- Consumer can name a Channel, defaults to Adaptor name.
- On case of new channel, either close (no new channel), or proceed to open
  channel.
- On new channel creation, show L1 status.
- Await confirmation from partner. The current status is displayed.

# Consumer Channel Show

- Consumer wishes to inspect the Channels state and history.
- Consumer can see current Channel state.
- Consumer can scroll complete history. Each paid invoice expands to all
  relevant and available data.
- Consumer can export data (JSON or CSV).

# Consumer Channel Add

- Consumer adds funds to the channel from the embedded wallet.

# Consumer Channel Close

- Consumer wishes to disengage a channel and recover funds.
- Funds can be recovered to the embedded wallet (default), or an external
  address.
- Either:
  - Consumer sends Adaptor invitation to leave by mutual consent.
  - Consumer closes unilaterally. In this case, Consumer await the Adaptor's
    response on the L1. The resolution of the close is handled automagically by
    the App.
- In either case, the current status of proceedings is displayed.
- The Channel data is "Achieved"

# Cardano Connector Service

This is a separate service to meet Consumer needs of L1 state and tx submission.
Someone needs to run this. We assume Adaptor maintains an instance for their
consumers. However, this introduce trusts of Consumer on Adaptor. Adaptor may
lie about the on-chain state, and put Consumer funds at risk. Consumer is
encouraged to use a separate entity for their Cardano Connector. A keen Consumer
running their own Cardano node, and with a little technical know-how can also
run this service.

There is graceful handling of failure, although TBC exactly what that handling
is.

- App requests a credential is tracked (indexed) as payment address from a point
  in time (default to now).
- The App requests an update.
- The Cardano Connector responds if there are any relevant events since last
  update. If last update is in history, then only chain events since update are
  included. Else there has been a rollback since last update, and the L1 updates
  accordingly. If there are no relevant events, the response is empty.
- Events include all transactions from payment address, and all channels with
  credentials as partner.
- Cardano Connector provides data to Consumer to build txs (open, add, close).
- Adaptor (or keen Consumer) can configure access to server via allow deny
  lists.

# Adaptor Setup

Adaptor has a BLN node running on a machine. Adaptor wishes to add Konduit
Consumers.

- Adaptor can deploy Konduit Adaptor Node by either:
  - Configurable Helm chart (TBC: or equivalent).
  - RYO from docker image, with example compose config.
  - From binary (or from source) with an example systemd process file.
- Adaptor configures Channel settings, such as Currency, Close period, Minimum
  Channel flux _etc_. These values are shared with potential channel partners.
- Adaptor configures BLN and Cardano node access.
- Once the machine is running, there is a health check. The liveness check
  reports status of connection to different networks, as well as current usage
  metrics.

There are quite a lot of properties to configure in the Konduit node, with
relatively involved understanding. For example, when to close a Channel and
whether to do so in batches. These properties need to be well presented to the
Adaptor.

# Adaptor Show

Adaptor has left the machine running and wants to know how its been going.

- Adaptor can read logs of all events
- Adaptor can query health check to see connection to networks and current usage
  metrics
- Adaptor can show all Channels, and specific Channel's state and history
- Adaptor can show "problematic" Channels, such as "with Pending HTLC"

# Adaptor Manual Action

Adaptor wishes to stop serving particular Channel(s) or Close Channel.

- Adaptor suspends a Channel's service (or set of Channels). Cheques are no
  longer handled from these Channels. An optional text field can be set which is
  forwarded to Consumer as the error message in response to a cheque.
- Adaptor closes Channels (in batch).

# Adaptor Restart

An Adaptors Machine stopped, gracefully or otherwise. The Adaptor wishes to
restart the Machine.

- There is a straight forward way to restart the Konduit node, such that syncing
  with the chains is safe yet fast.

# Adaptor Edit Config

Adaptor wishes to use change values in their configuration.

- Adaptor creates new instance with the new config.
- The operation of existing Channels is not affected, at least with respect to
  the properties of the Channel.
- New Channels will be accepted only if they meet the new criteria.
- Care must be take when changing, say, Currency (TBC whether this is
  permissible).
- Configuration permits suspending all new Channel connections

# Dev Audit

Dev wishes to Konduit to see what it can and cannot do.

- Dev can deploy locally with connections to either real or mock chain nodes.
- Dev can run App to connect to local Konduit node.
- Dev can review previous ADRs.
- Dev can see roadmap of development of features currently supported, or soon to
  be supported.
- Project is setup in a way that invites contributions and suggestions.

# Marketer Audit

Marketer wishes to understand the value of the project. They do not have
technical expertise, and do not personally wish to use Konduit. They want to
promote the project, but only once they have confidence it can meet a genuine
need, and is well founded from an engineering and product perspective.

- Marketer can access material explaining at a high level how Konduit works
- Marketer can access and understand the usage metrics provided, including total
  funds locked, and total funds used.
- A marketer can easily provide feedback, ask questions, and make suggestions.

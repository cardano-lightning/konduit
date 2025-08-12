---
title: "Konduit Requirements :: App"
---

App is a mobile first application available via web and natively on popular OSs.
It is the user interface of Konduit for User.

We adopt the following conventions for specification of App:

- Unless stated otherwise, each subsection is a page of App. Otherwise it states
  "widget", "card", "function" _etc_.
- The phrase "Pressing Back" applies to a browser or Android OS context. For iOS
  a back button needs to appear. Standard design is to have back arrow in the
  top left.
- Subsections are linked. Depending on the markdown renderer this may or may not
  work in your chosen viewer. We refer to each subsection by its full title
  verbatim.

#### Launch

When there is no key in App state, such as first load, App opens to App Launch
page. This is the first thing User sees.

Display:

- Logo.
- App name.
- Tag line (TBC).
- Buttons:
  - "Import": Opens [Import Function](#import-function)
  - "Create": Opens [Create Function](#create-function)
- Help (?) button. On click explains why App setup is necessary and the
  expectations of the file imported.
- App version.
- Link to site (placeholder: "cardano-lightning.org/konduit"). Opens new page or
  external browser.

##### Import Function

User imports existing key and settings. Importing uses OS or browser's file
browser function to find file. The file format aligns with that which is
exported.

TBC the precise format. Working assumption that this is a JSON, contains its
format version, keys are Bech32 encoded, and it includes other app settings that
are created on [Setup](#setup), such as `createdAt` and `stakeKey`. The
additional fields are for chain indexing purposes, and staking.

Importing an incorrectly formatted file results in failure with appropriate
error. Importing a file in which required fields are not present or not
parse-able results in running the App setup with the remaining "good" fields
pre-set. Optional fields can be omitted.

TBC key is passcode protected.

##### Create Function

App creates new e25519 key. This is used throughout for embedded wallet, and
channel credential.

On create the user is lead through the [Setup](#setup)

#### Setup

A sequence of pages to initialise configuration of App (aka a setup wiazrd).

Triggered by [Create](#create), and [Import](#import) in the case that there are
missing fields.

Setup consists of a sequence of input forms. There is a progress to indicate how
far through the setup User is. Each form is also accessible via
[Settings](#settings).

For each input page there is:

- Input label
- Short description
- Help icon (?) with more detailed description
- Input field possibly with sensible default
- In some cases, there might be multiple inputs on the same page.
- A `Skip` button if the setting is optional.
- A `Next` button, "disabled" until the Input is valid.
- An attempt to submit invalid data results in an unobtrusive error, reporting
  any known reason for invalidity, with examples of valid input.

The input pages include:

- App Settings L1 Liaison
- App Settings Currencies - Only Ada is supported at this time.
- App Settings Stake
- App Settings Locale - Date time, number format, fiat.
- App Settings XXX

Note that the behaviour is a little different to the displayed when accessed via
[Settings](#settings). Pressing "Back" on these pages goes back one page:
pressing "Back" when accessed via app settings returns to app settings. The
Button says `Next` rather than `Ok`.

After App Setup is complete, Open [Embedded Wallet](#embedded-wallet).

#### Settings

Page contains:

- List of settings:
  - Each setting has
    - Label
    - Help icon which brings up floating help OR expands a collapsible dialogue
      box (TBC which).
    - Current value
    - Edit button. If clicked, triggers the corresponding App Settings for
      property
- Export Button: Triggers [Export](#export)
- Forget Button: Triggers [Forget](#forget)
- [Nav Bar](#nav-bar)

On any App Settings X (ie subpage) page:

- Maybe "Back" Arrow. Pressing "Back" returns to App Settings page, without
  saving any changes.
- Settings title
- Short description
- Help icon
- Input field with previous value as default

List of settings (order):

- [Settings VKey](#settings-vkey)
- [Settings Sync From](#settings-sync-from)
- [Settings L1 Liaison](#settings-l1-liaison)
- [Settings Currencies](#settings-currencies)
- [Settings Stake](#settings-stake)
- [Settings Locale](#settings-locale)
- [Settings L1 Explorer](#settings-l1-explorer)
- [Settings Price Feeds](#settings-price-feeds)

#### Settings VKey

Title: VKey

Info: This key is used for all channels and embedded wallet. The VKey is not
editable. You can only [Settings Forget](#settings-forget).

Display: Default format is Bech32, with copy Button.

Edit: Not available

#### Settings Sync From

Title: Sync From

Info: This datetime is used to ask the L1 Liaison from when to track the
credential. If changed to an earlier time, App will re-sync L1 state from
scratch. It defaults to the time the VKey is created.

Display: Datetime in condensed form with timezone, depending on
[Settings Locale](#settings-locale).

Edit: Datetime selector.

#### Settings L1 Liaisons

Title: L1 Liaisons

Info: The L1 Liaison syncs state with and submits txs to the Cardano L1. The
endpoint must provide an API compatible with this app. Having a diverse set of
L1 Liaisons increases confidence that the local state is accurately reflecting
the state on the L1. For example: - `cardano-lightning.org/konduit/l1l/v0` -
`konduit.cardanofoundation.org/l1l/v0`

Display: The URLs

Edit: Edit or remove an existing entry, add a new URL.

#### Settings Stake

Title: Stake

Info: (Optional) If set, all new channels will use stake address, as will the
embedded wallet. Note that channels stake credentials cannot be changed.

Display: As Bech32

Edit: A text field. Accept Bech32, or hex, or address with stake credentials.
"OK" button is "disabled" if input cannot be parsed, but provide info to
supported input formats with examples, if they attempt to put in unrecognized
data.

#### Settings Currencies (TBC)

Title: Currencies

Info: This version supports only Ada channels. There is no abitility to set the
properties of Ada. In future, other currencies can be supported. A Currency can
be set with the following properties:

- Name. UTF-8. User's choice
- PolicyId. Hexidecimal 56 characters. Aka script hash, this is the blake2b256
  hash of the script of the currency.
- Name. Hexidecimal, >= 64 characters. The token name.
- Symbol. Dropdown, or paste. Single character (Emoji support?). The symbol
  indicating character.
- DP. Non negative integer. The number of decimal places of the currency. For
  example, Ada has 6 as 1 Ada is 1000000 Lovelace.
- High Fee Flag. Two non negative numeric fields, an absolute and a percentage.
  If set, then a fee is greater than the indicated amount will be flagged on a
  consent form.

Display: Ada fields, non editable. Add button, disabled.

Edit: Disabled

#### Settings Locale

Title: Locale

Info: Set language, date and time format, fiat currency, hi/lo colors (TBC). To
be useful, the fiat currency must be available from the price feeds.

Display: Current settings.

Edit: TBC - Copy another apps locale settings. For MVP only `en_US` language is
supported.

#### Settings L1 Explorer

Title: L1 Explorer

Info: (Optional). If set, external links for transactions and addresses will use
the chosen explorer.

Display: "(None)" if None set (Also default), else root URL of selection.

Edit: Drop down list of "cexplorer.io", "cardanoscan.io", and any others easily
configured to work.

#### Settings Price Feeds

Title: Price Feeds

Info: (Optional). Price Feeds source provides the current exchange rates between
currencies: Ada, Bitcoin, Fiat _etc_. If set, prices and costs are converted. It
is required in order to determine high fees and display costs in fiat.

Display: If not set, then "(None)", else URL

Edit: URL selector or custom text field. Note that the latter will expect to
have one of the supported formats.

#### Settings Export

Title: Export

Info: Export keys and settings. The exported file can be used to setup Konduit
on another device, or after "Forgetting" details on the this device. DANGER -
The export contains the signing key, so keep the exported file safe.

Display: Button "Export"

On-click: Screen with danger notice. Button with Export icon. Launches devices
file browser to find location to save file.

#### Settings Forget

Title: Forget

Info: Forget keys and settings. This resets the App. Use the "Export" to save
the current keys and settings. This can be imported into the app in future.
Danger: Proceeding with "Forget" will reset the app.

Display: Button "Forget"

Edit: Not available

On-click: Screen with danger notice. Button with Forget Icon (Maybe dustbin).
On-click, remove all data, and show launch page.

#### Embedded Wallet

The embedded wallet covers collateral and funds channels. It is the default
output address when closing channels. The help dialogue conveys the purpose of
the embedded wallet, and that insufficient funds will impact correct functioning
(ie less than X amount of ada and txs will fail).

Widgets:

- Total Ada (in embedded wallet)
- Total Other (TBC)
- Wallet Address. Buttons: Copy, create QR, View address in L1 explorer (if
  set).
- App Embedded Wallet Activity Latest Widget, with "See all" Button. Opens
  [Embedded Wallet Activity](#embedded-wallet-activity).
- Withdraw. Launches [Embedded Wallet Withdraw](#embedded-wallet-withdraw)
- Nav bar

#### Embedded Wallet Activity

Embedded Wallet Activity, is all txs submitted from App, and or reported by L1
Liaisons. It is a list of Tx Previews. On click of preview, Open
[Embedded Wallet Tx](#embedded-wallet-tx). The order defaults to most recent
first. There is a filter for "Only confirmed".

A Tx Preview consists of the following info:

- TxId. Possibly squashed to fit inline. With Copy button and External link to
  explorer.
- Created at. This appears only if the transaction came from App, and so is
  known.
- Status. One of:
  - Confirmed. Number of confirmations. Time of block in which it is confirmed.
  - Pending.
  - Failed.
- Net Change to embedded wallet of asset of greatest abundant. Color coded by
  App Settings Locale Color.

#### Embedded Wallet Tx

Displaying tx details.

Showing:

- TxId. Copy and L1 Explorer link buttons
- Created at. This appears only if the transaction came from App, and so is
  known.
- Status. One of:
  - Confirmed. Number of confirmations out of total L1 Liaisons. Time of block
    in which it is confirmed. If confirmations is not all L1 Liaisons, then on
    hover indicate which have, and which have not confirmed.
  - Pending.
  - Failed.
- Tx details. Something to similar to what you see in wallets.

#### Embedded Wallet Withdraw

The embedded wallet comes tx fees and funds channels. It is the default output
address when closing channels. The Help dialogue conveys the purpose of the
embedded wallet, and that insufficient funds will impact correct functioning (ie
less than X amount of ada and txs will fail).

- Total Ada (in embedded wallet)
- Total Other (TBC)
- Input slider for Withdraw amount (or amounts TBC)
- Output Address
- Cancel Button. On click returns to App Embedded Wallet
- Submit Button. On click returns to App Embedded Wallet

#### Home

Opening App when fully configured, opens to Home page. All other pages have
"Back" button and "Title" in the same location, as Home "Logo" and "App Name"

Home comprises of the following widgets.

- Logo and App name.
- Connection Status Widget. This indicates to the user whether there are any
  problems accessing services, such as price feed, l1 liaisons, or channels. If
  there are no issues, display "Connected", if there are issues with connection
  display "Issues with connections >". On-click, display full screen modal of
  the connection issues: which service, request made (default collapsed),
  response given if any (default collapsed), error help if any.
- [Channel List Widget](#channel-list-widget).
- [App Activity] Preview Widget. Displays activity previews from embedded
  wallet, and all channels.
- [Action Bar Widget](#action-bar-widget)

#### Channel List Widget

Display the current Channels.

Horizontal scroll carousel of active channel cards, ordered by most funded.
Default focus is most funded channel. Each Channel Card displays:

- Channel name
- Amount available (ie not yet spent or committed)
- Amount locked
- amount pending, if any
- Currency symbol

On click: go to corresponding [Channel](#channel) page.

Below channel carousel are button:

- Add icon: On-click [Channel Add](#channel-add)
- Archive icon: On-click [Channel Archived](#channel-archived).

#### Channel

Shows status of channel, activity, and actions. User can also enter [Pay](#pay)
from here, where only this channel will provide quote.

Widgets:

- Total/ Uncommitted / Pending, Currency Symbol. If currency is not ada, display
  Policy and name each with "Copy" button, and "External Link" button to L1
  explorer of asset.
- Channel status: L1 stage (eg "Opened", "Closed"). Last updated, and number of
  confirmations. Link to UTXO on L1 explorer,
- [Channel Activity](#channel-activity) latest. On click open full list.
- If channel stage is "Opened", then Pay button: Open [Pay](#pay). Only current
  channel will provide quote.
- Tx options, if channel stage is "Opened", then
  - "Add" Button : [Add Tx](#add-tx). Only if Channel
  - "Close" Button : [Close Tx](#close-tx)
- Tx options, if channel stage is "Closed", then
  - "Settle" Button : [Settle Tx](#settle-tx). Only if this is relevant

#### Add Tx

TODO

#### Close Tx

TODO

#### Settle Tx

TODO

#### Channel Activity

List of Channel Event Preview Cards. Ordered by time of event. Paginate if long.
Export Button, on-click downloads JSONL file.

Each preview card.

- HTLC:
  - Amount
  - Destination address (truncated or shorthand).
  - Time
  - Resolution (and time) or otherwise status eg "Failed"
  - Subsumed by Snapshot
- Snapshot:
  - Time
  - Details
  - TBC whether this is necessary
- Tx:
  - Tx type: Open, Add, Close, ...
  - Time (if locally created, then time submitted. If only seen on-chain, then
    time of block)
  - Status: Pending, Confirmed, Failed.

On click: The corresponding [Activity Details](#activity-details) is opened.

#### HTLC Details

Activity details displays long form what is available in preview.

TODO

#### Snapshot Details

Activity details displays long form what is available in preview.

TODO

#### Tx Details

Activity details displays long form what is available in preview.

TODO

#### Channel Add

Top button "Manual" to skip QR code. Preview of Camera view. On scan: - If
parse-able channel partner details, then render manual form, with fields filled
in. - Else display error "Cannot make sense of QR code". On "Back" return to QR
scanner. On click of manual or successful scan open form view. User must set
amount to fund channel. There are two numbers to provide context:

- Partner's "min volume" or "min flux" (TBC)
- Embedded Wallet amount of the relevant currency

Bottom of form has "Add" button. On click submits tx via L1 Liaison. Returns to
[Home](#home). New channel appears in
[Channel List Widget](#channel-list-widget), and with new channel in focus (if
this is not too much work). Channel Status indicates confirmation pending,
successful, or failed.

#### Action Bar Widget

Action bar has three icons:

- App Pay: On click open [Pay](#pay). Button is primary.
- App Embedded Wallet: On click [Embedded Wallet](#embedded-wallet). Button is
  secondary, but display as "Warning" if funds are low, or data is stale or
  there is a problem with L1 connectivity.
- App Settings: On click [Settings](#settings). Button is secondary.

#### Pay

The key feature of App, meeting the PPP.

Page is context aware: If entered from [Channel](#channel), then "Back" returns
to that channel. Otherwise return to [Home](#home).

QR code scanner. Button for "Manual". Manual form includes details of BLN
invoice.

On scan:

- If unparse-able, handle and display error. "Back" returns to QR code scanner.
- If parse-able, go to "Manual" form with values filled.

Manual form submit button is "Get Quote" (single channel case) or "Get Quotes"
(multi channel case) if available. Help icon makes this point clear: Will not
get a quote from channels for which there are insufficient funds. If App Pay is
launched from a [Channel](#channel), the quote will only be for that channel. In
this case User can request quotes from other channels via [Quotes](#quotes).

On submit, open [Quotes](#quotes).

#### Quotes

Triggered by [Pay](#pay).

"Back" returns to App Pay. If App Pay was opened via [Channel](#channel), then
only this channel will be quoted.

Page displays a list of Quotes vertically. The update of the quotes will load
asynchronously. While awaiting quote, indicate awaiting. If quote request fails,
display failure on Quote. Quote ordered from cheapest at top. If a cheaper quote
arrives later, an animation makes clear to User a reordering has taken place.
Channels available, but no quote requested are listed below quotes. On-click a
quote is requested. Channels unavailable (insufficient funds or otherwise), are
greyed out.

On failed quote, On click display error message. On successful quote, on click
launches [Pay Confirm](#pay-confirm).

#### Pay Confirm

Full details of payment and channel are displayed. If fees are high according to
[Settings High Fee Flag](#settings-high-fee-flag) then fee is flagged. User can
confirm or cancel. Confirm goes to [Channel](#channel). Cancel returns to
[Quote](#quote) with previous state.

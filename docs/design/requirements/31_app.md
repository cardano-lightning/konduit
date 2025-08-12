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
- Help (?) button. On click explains why App setup is necessary.

- App version.
- Link to site: Opens new page or external browser.

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
- App Settings Stake
- App Settings Currencies (TBC) - Crypto
- App Settings High Fee Flag
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
    - Short description
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

#### Settings L1 Liaisons

TODO

#### Settings L1 Explorer

Title: L1 Explorer

Info: (Optional). If set, external links for transactions and addresses will use
the chosen explorer.

Display: "(None)" if None set (Also default), else root URL of selection.

Edit: Drop down list of "cexplorer.io", "cardanoscan.io", and any others easily
configured to work.

#### Settings VKey

Title: VKey

Info: This key is used for all channels and embedded wallet.

Display: Default format is Bech32, with copy Button.

Edit: The VKey is not editable. User can only
[Settings Forget](#settings-forget).

#### Settings Created At

Title: Created at

Info: This datetime is used to ask the L1 Liaison from when to track the
credential. If changed to an earlier time, App will re-sync L1 state from
scratch.

Display: Datetime in condensed form with timezone, depending on
[Settings Locale](#settings-locale).

Edit: The VKey is not editable. User can only
[Settings Forget](#settings-forget).

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

Info: This version supports only Ada channels.

Display: Ada -> Ada symbol.

Edit: Disabled

#### Settings High Fee Flag

Title: Fee flags

Info: If set > 0, then fees greater than this settings are deemed high and are
visually indicated.

Display: Absolute number, percentage (TBC).

Edit: Two number selectors for absolute and percentage.

#### Settings Locale

Title: Locale

Info: Language, date and time, currency.

Display: Absolute number, percentage (TBC).

Edit: Two number selectors for absolute and percentage.

TODO

#### Settings Price Feeds

Title: Price Feeds

Info: Price Feeds source provides the current exchange rates between currencies:
Ada, Bitcoin, Fiat _etc_.

Display: If not set, then "(None)", else URL

Edit: URL selector or custom text field. Note that the latter will expect to
have one of the supported formats.

#### Settings Export

TODO

#### Settings Forget

TODO

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
  [Embedded Wallet Activity](#.....).
- Withdraw. Launches [Embedded Wallet Withdraw](#......)
- Nav bar

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

#### Home

Opening App when fully configured, opens to Home. Returning from settings or
other pages defaults to returning to Home, and all such pages have "Back"
buttons and "Title" in the same location.

Home is a collection of widgets.

- Logo and App name.
- Connection status: On-line: if off-line display alert. L1 Liaisons: if not
  reached display alert.
- [Channel List Widget](#channel-list-widget).
- Latest App Activity Widget. Displays activity previews from embedded wallet,
  and all channels.
- [Nav bar](#nav-bars)

#### Nav Bar

- App Pay: On click open [Pay](#pay). Button is primary.
- App Embedded Wallet: On click [Embedded Wallet](#embedded-wallet). Button is
  secondary, but display as "Warning" if funds are low, or data is stale or
  there is a problem with L1 connectivity.
- App Settings: On click [Settings](#settings). Button is secondary.

#### Channel List Widget

Display the current Channels.

Horizontal scroll carousel of active channel cards, ordered by most funded.
Default focus is most funded channel or buttons if there are none. Scroll left
for buttons for [Channel Add](#channel-add), and
[Channel Archived](#channel-archived). Scroll right for other channels. Channel
Card displays: - Channel name - amount locked - amount not yet committed -
amount pending if any - currency

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

#### Channel

Shows status of channel, activity, and actions. User can also enter [Pay](#pay)
from here, where only this channel will provide quote.

Widgets:

- Total/ Uncommitted / Pending, Currency
- Last updated, number of confirmations, Link to UTXO on L1 explorer,
- [Channel Activity](#channel-activity) latest. On click open full list.
-

TODO

#### Channel Activity

Paginate if long. Export Button, on-click downloads JSONL file.

TODO

#### Channel Event

TODO

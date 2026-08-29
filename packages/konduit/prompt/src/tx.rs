use std::collections::BTreeMap;

use anyhow::Result;
use cardano_sdk::Input;
use inquire::Select;
use konduit_data::{Constants, Duration, Tag, Unlocked, VerifyingKey};
use konduit_tx2::{
    StagedTx,
    channel::Channel,
    step::{Can, Want},
};

use crate::{
    known_keys::KnownKeys,
    prompt::{Candidate, prompt_with_candidates, variant_prompt},
    receipt::{Keytag, Receipt, Receipts},
};

const DROP_A_PROPOSAL: &str = "(drop a proposal)";
const ADD_OPEN: &str = "(open a new channel)";
const DROP_AN_OPEN: &str = "(drop an open)";
const REVIEW: &str = "(review staged so far)";
const FINISH: &str = "(finish)";
const USE_RECEIPT: &str = "(use matching receipt)";
const PICK_MANUALLY: &str = "(pick manually)";

/// Everything a `Tx` session needs. `Open` is really just another
/// kind of `Want` (both end up staged against a `StagedTx`) but too
/// distinct in its prompting to share `build_want`'s variant menu — not
/// distinct enough to need a separate context, though: both need the
/// same pasted-or-picked signed-value pools and known keys.
///
/// There's no `squashes`/`cheques` pool any more — `Sub`/`Respond` draw
/// theirs entirely from `receipts` now, which already models exactly
/// that pair.
#[derive(Default)]
pub struct Ctx {
    pub unlockeds: Vec<Candidate<Unlocked>>,
    pub known_keys: Vec<Candidate<VerifyingKey>>,
    pub receipts: Receipts,
}

pub fn build_staged_tx_interactively(
    staged: &mut StagedTx,
    known_keys: &KnownKeys,
    ctx: &Ctx,
) -> Result<()> {
    let mut proposed: BTreeMap<Input, &'static str> = BTreeMap::new();

    loop {
        let inputs: Vec<Input> = staged.channels().keys().cloned().collect();

        let mut menu: Vec<String> = inputs
            .iter()
            .map(|input| {
                let channel = &staged.channels()[input];
                let status = proposed
                    .get(input)
                    .map(|w| format!("  [{w} proposed]"))
                    .unwrap_or_default();
                format!("{}{status}", describe_channel(known_keys, input, channel))
            })
            .collect();

        menu.push(ADD_OPEN.to_string());
        if !staged.opens().is_empty() {
            menu.push(DROP_AN_OPEN.to_string());
        }
        if !proposed.is_empty() {
            menu.push(DROP_A_PROPOSAL.to_string());
        }
        if !proposed.is_empty() || !staged.opens().is_empty() {
            menu.push(REVIEW.to_string());
        }
        menu.push(FINISH.to_string());

        let chosen = Select::new("Act on which channel?", menu.clone()).prompt()?;

        if chosen == FINISH {
            return Ok(());
        }
        if chosen == ADD_OPEN {
            let channel = build_open(ctx)?;
            if staged.add_open(channel) {
                println!("proposed open");
            } else {
                println!("already proposed (duplicate open)");
            }
            continue;
        }
        if chosen == DROP_AN_OPEN {
            let target = pick_open(staged, known_keys, "drop which open?")?;
            staged.retain(|c| c != &target);
            continue;
        }
        if chosen == DROP_A_PROPOSAL {
            let droppable: Vec<Input> = proposed.keys().cloned().collect();
            let input = pick_input(
                staged,
                known_keys,
                &droppable,
                "drop the proposal for which channel?",
            )?;
            staged.drop_intent(&input);
            proposed.remove(&input);
            continue;
        }
        if chosen == REVIEW {
            print_review(staged, &proposed, known_keys);
            continue;
        }

        let idx = menu
            .iter()
            .position(|m| *m == chosen)
            .expect("selected label must be present");
        let input = inputs[idx].clone();
        let channel = staged.channels()[&input].clone();

        let want = build_want(&channel, ctx)?;
        let want_label = want.label();
        let who = describe_channel(known_keys, &input, &channel);
        match staged.want(input.clone(), want) {
            Ok(()) => {
                proposed.insert(input, want_label);
                println!("proposed {want_label} for {who}");
            }
            Err(e) => eprintln!("rejected: {e}"),
        }
    }
}

fn pick_input(
    staged: &StagedTx,
    known_keys: &KnownKeys,
    inputs: &[Input],
    prompt: &str,
) -> Result<Input> {
    let menu: Vec<String> = inputs
        .iter()
        .map(|i| describe_channel(known_keys, i, &staged.channels()[i]))
        .collect();
    let chosen = Select::new(prompt, menu.clone()).prompt()?;
    let idx = menu
        .iter()
        .position(|m| *m == chosen)
        .expect("selected label must be present");
    Ok(inputs[idx].clone())
}

fn pick_open(staged: &StagedTx, known_keys: &KnownKeys, prompt: &str) -> Result<Channel> {
    let opens: Vec<Channel> = staged.opens().iter().cloned().collect();
    let menu: Vec<String> = opens.iter().map(|c| describe_open(known_keys, c)).collect();
    let chosen = Select::new(prompt, menu.clone()).prompt()?;
    let idx = menu
        .iter()
        .position(|m| *m == chosen)
        .expect("selected label must be present");
    Ok(opens[idx].clone())
}

fn print_review(
    staged: &StagedTx,
    proposed: &BTreeMap<Input, &'static str>,
    known_keys: &KnownKeys,
) {
    println!("\n-- staged so far --");
    for (input, label) in proposed {
        let channel = &staged.channels()[input];
        println!(
            "  {}: {label}",
            describe_channel(known_keys, input, channel)
        );
    }
    for channel in staged.opens() {
        println!("  {}", describe_open(known_keys, channel));
    }
    println!("  net gain: {} lovelace", staged.gain());
    println!("  signers: {}", staged.signers().len());
    println!();
}

fn describe_channel(known_keys: &KnownKeys, input: &Input, channel: &Channel) -> String {
    let who = known_keys
        .channel_label(channel.constants())
        .unwrap_or_else(|| short_input(input));
    format!(
        "{who} — {} · {} lovelace",
        channel.stage().label(),
        channel.amount()
    )
}

/// Same as `describe_channel` but for a not-yet-staged open, which has no
/// `Input` to fall back on.
fn describe_open(known_keys: &KnownKeys, channel: &Channel) -> String {
    let who = known_keys
        .channel_label(channel.constants())
        .unwrap_or_else(|| "unknown counterparty".to_string());
    format!("{who} — open · {} lovelace", channel.amount())
}

/// Never-raw-hex fallback for a channel whose keys aren't in
/// `KnownKeys` — a short, stable stand-in built from the input itself.
fn short_input(input: &Input) -> String {
    let debug = format!("{input:?}");
    let short: String = debug.chars().take(24).collect();
    if short.len() < debug.len() {
        format!("{short}…")
    } else {
        short
    }
}

// --- Want ---

/// Offers only variants `channel.can()` allows — same source `resolve()`
/// checks, so the menu can't offer something that'd then be rejected —
/// plus, now, `Sub`/`Respond` further require a matching receipt (see
/// `can_variant`), since there's no other way left to supply one.
fn build_want<'a>(channel: &'a Channel, ctx: &'a Ctx) -> Result<Want> {
    let variants = channel
        .can()
        .iter()
        .filter_map(|c| can_variant(c, channel, ctx))
        .collect();
    variant_prompt("Want:", variants)
}

/// `None` for `Sub`/`Respond` when no receipt matches this channel —
/// `Ctx` no longer holds a squash/cheque pool to fall back to, so without
/// a receipt there's nothing to build them from.
fn can_variant<'a>(
    can: &Can,
    channel: &'a Channel,
    ctx: &'a Ctx,
) -> Option<(&'static str, Box<dyn Fn() -> Result<Want> + 'a>)> {
    match can {
        Can::Add => Some((
            "Add",
            Box::new(|| {
                Ok(Want::Add {
                    amount: prompt_u64("amount")?,
                })
            }),
        )),
        Can::Sub { .. } => {
            matching_receipt(ctx, channel)?;
            Some(("Sub", Box::new(move || prompt_sub(channel, ctx))))
        }
        Can::Close => Some(("Close", Box::new(|| Ok(Want::Close)))),
        Can::Respond { .. } => {
            matching_receipt(ctx, channel)?;
            Some(("Respond", Box::new(move || prompt_respond(channel, ctx))))
        }
        Can::End => Some(("End", Box::new(|| Ok(Want::End)))),
        Can::Elapse { .. } => Some(("Elapse", Box::new(|| Ok(Want::Elapse)))),
        Can::Unlock => Some(("Unlock", Box::new(move || prompt_unlock(channel, ctx)))),
        Can::Expire => Some(("Expire", Box::new(|| Ok(Want::Expire)))),
    }
}

fn prompt_u64(field: &str) -> Result<u64> {
    Ok(inquire::CustomType::<u64>::new(&format!("{field}:")).prompt()?)
}

/// A channel's `Constants` (its `add_vkey` + `tag`) are the receipt
/// context's defacto id — `Receipts` is keyed by exactly that pair.
fn matching_receipt<'r>(ctx: &'r Ctx, channel: &Channel) -> Option<&'r Receipt> {
    let constants = channel.constants();
    let key_tag = Keytag::new(&constants.add_vkey, &constants.tag);
    ctx.receipts.get(&key_tag)
}

/// `Sub` and `Respond` are only ever offered (see `can_variant`) once a
/// receipt matches, so this can't miss in practice — the `ok_or_else` is
/// just a clear error instead of a panic if that guarantee ever slips.
fn prompt_sub(channel: &Channel, ctx: &Ctx) -> Result<Want> {
    let receipt = matching_receipt(ctx, channel)
        .ok_or_else(|| anyhow::anyhow!("Sub: no matching receipt"))?;
    Ok(Want::Sub {
        squash: receipt.squash.clone(),
        cheques: receipt
            .cheques
            .iter()
            .filter_map(|u| u.as_unlocked())
            .collect(),
    })
}

fn prompt_respond(channel: &Channel, ctx: &Ctx) -> Result<Want> {
    let receipt = matching_receipt(ctx, channel)
        .ok_or_else(|| anyhow::anyhow!("Respond: no matching receipt"))?;
    Ok(Want::Respond {
        squash: receipt.squash.clone(),
        cheques: receipt.cheques.clone(),
    })
}

fn prompt_unlock(channel: &Channel, ctx: &Ctx) -> Result<Want> {
    let receipt = matching_receipt(ctx, channel)
        .ok_or_else(|| anyhow::anyhow!("Sub: no matching receipt"))?;
    Ok(Want::Unlock {
        secrets: receipt
            .cheques
            .iter()
            .filter_map(|u| u.as_unlocked().map(|u| *u.secret()))
            .collect(),
    })
}

// --- Open ---

/// Builds a fresh open `Channel`: vkeys (picked from `ctx.known_keys` or
/// pasted) + tag + close_period + amount. Delegation is a placeholder.
fn build_open(ctx: &Ctx) -> Result<Channel> {
    let add_vkey = prompt_vkey("add_vkey", &ctx.known_keys)?;
    let sub_vkey = prompt_vkey("sub_vkey", &ctx.known_keys)?;
    let tag = prompt_tag()?;
    let close_period = prompt_duration("close_period")?;
    let amount = prompt_u64("amount")?;
    let constants = Constants {
        tag,
        add_vkey,
        sub_vkey,
        close_period,
    };
    Ok(Channel::new_open(
        None, /* TODO: delegation */
        constants, amount,
    ))
}

fn prompt_vkey(field: &str, candidates: &[Candidate<VerifyingKey>]) -> Result<VerifyingKey> {
    prompt_with_candidates(field, candidates, || paste_vkey(field))
}

fn paste_vkey(field: &str) -> Result<VerifyingKey> {
    let raw = inquire::Text::new(&format!("{field} (hex, 32 bytes):")).prompt()?;
    let bytes: [u8; 32] = hex::decode(raw.trim())?
        .try_into()
        .map_err(|_| anyhow::anyhow!("{field}: expected 32 bytes"))?;
    Ok(VerifyingKey::from(bytes))
}

fn prompt_tag() -> Result<Tag> {
    let raw = inquire::Text::new("tag (hex, upto 32 bytes):").prompt()?;
    let bytes = hex::decode(raw)?;
    Ok(Tag::from(bytes))
}

fn prompt_duration(field: &str) -> Result<Duration> {
    let raw = inquire::Text::new(&format!("{field}:")).prompt()?;
    match raw.parse::<Duration>() {
        Ok(d) => Ok(d),
        Err(e) => {
            println!("Err {}. Try again", e);
            prompt_duration(field)
        }
    }
}

use anyhow::Result;
use konduit_data::{Cheque, Constants, Duration, Squash, Tag, Unlocked, VerifyingKey};
use konduit_tx2::{
    channel::Channel,
    step::{Can, Want},
};

use crate::{
    prompt::{Candidate, prompt_many, prompt_signed, prompt_with_candidates, variant_prompt},
    receipt::Receipts,
};

/// Single context for the whole `Propose` flow. `Open` is really just
/// another kind of `Want` (both end up staged against a `StagedTx`) but
/// too distinct in its prompting to share `build_want`'s variant menu —
/// not distinct enough to warrant a separate context, though: both need
/// the same pasted-or-picked signed-value pools and known keys.
#[derive(Default)]
pub struct ProposeContext {
    pub squashes: Vec<Candidate<Squash>>,
    pub cheques: Vec<Candidate<Cheque>>,
    pub unlockeds: Vec<Candidate<Unlocked>>,
    pub known_keys: Vec<Candidate<VerifyingKey>>,
    pub receipts: Receipts,
}

/// Offers only variants `channel.can()` allows — same source `resolve()`
/// checks, so the menu can't offer something that'd then be rejected.
pub fn build_want(channel: &Channel, ctx: &ProposeContext) -> Result<Want> {
    let variants = channel.can().iter().map(|c| can_variant(c, ctx)).collect();
    variant_prompt("Want:", variants)
}

fn can_variant<'a>(
    can: &Can,
    ctx: &'a ProposeContext,
) -> (&'static str, Box<dyn Fn() -> Result<Want> + 'a>) {
    match can {
        Can::Add => (
            "Add",
            Box::new(|| {
                Ok(Want::Add {
                    amount: prompt_u64("amount")?,
                })
            }),
        ),
        Can::Sub { .. } => ("Sub", Box::new(move || prompt_sub(ctx))),
        Can::Close => ("Close", Box::new(|| Ok(Want::Close))),
        Can::Respond { .. } => ("Respond", Box::new(move || prompt_respond(ctx))),
        Can::End => ("End", Box::new(|| Ok(Want::End))),
        Can::Elapse { .. } => ("Elapse", Box::new(|| Ok(Want::Elapse))),
        Can::Unlock => ("Unlock", Box::new(move || prompt_unlock(ctx))),
        Can::Expire => ("Expire", Box::new(|| Ok(Want::Expire))),
    }
}

fn prompt_u64(field: &str) -> Result<u64> {
    Ok(inquire::CustomType::<u64>::new(&format!("{field}:")).prompt()?)
}

fn prompt_sub(ctx: &ProposeContext) -> Result<Want> {
    let squash = prompt_signed("squash", &ctx.squashes)?;
    let cheques = prompt_many("cheque", || prompt_signed("cheque", &ctx.cheques))?;
    Ok(Want::Sub { squash, cheques })
}

fn prompt_respond(ctx: &ProposeContext) -> Result<Want> {
    let squash = prompt_signed("squash", &ctx.squashes)?;
    let cheques = prompt_many("cheque", || prompt_signed("cheque", &ctx.cheques))?;
    Ok(Want::Respond { squash, cheques })
}

fn prompt_unlock(ctx: &ProposeContext) -> Result<Want> {
    let unlockeds = prompt_many("unlocked cheque", || {
        prompt_signed("unlocked cheque", &ctx.unlockeds)
    })?;
    Ok(Want::Unlock { unlockeds })
}

// --- Opens ---

/// Builds a fresh open `Channel`: vkeys (picked from `ctx.known_keys` or
/// pasted) + tag + close_period + amount. Delegation is a placeholder.
pub fn build_open(ctx: &ProposeContext) -> Result<Channel> {
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

// TODO: confirm Tag's actual constructor.
fn prompt_tag() -> Result<Tag> {
    let raw = inquire::Text::new("tag (hex, upto 32 bytes):").prompt()?;
    let bytes = hex::decode(raw)?;
    Ok(Tag::from(bytes))
}

fn prompt_duration(field: &str) -> Result<Duration> {
    let raw = inquire::Text::new(&format!("{field}:")).prompt()?;
    Ok(raw.parse::<Duration>()?)
}

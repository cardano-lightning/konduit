use anyhow::Result;
use inquire::{Confirm, Select};

/// A selectable option paired with the real value it resolves to — e.g.
/// a human-readable keyring label paired with the actual key.
pub struct Candidate<T> {
    pub label: String,
    pub value: T,
}

impl<T> Candidate<T> {
    pub fn new(label: impl Into<String>, value: T) -> Self {
        Self {
            label: label.into(),
            value,
        }
    }
}

/// Injectable context for prompt-builders — anything beyond raw typed
/// user input that a specific CLI might want to offer as a shortcut.
/// Add fields as more context is needed; an empty field just means every
/// candidate list built from it is empty, so every prompt using it falls
/// straight through to manual entry. A CLI with nothing to inject uses
/// `PromptContext::default()`.
#[derive(Default)]
pub struct PromptContext<K> {
    /// e.g. keyring entries: label -> key. Generic over `K` so this
    /// doesn't force a dependency on any particular key type here.
    pub keys: Vec<Candidate<K>>,
}

/// Offers `candidates` as a menu; falls back to `manual` if there are
/// none, or if the user declines to pick from the list. `field` is used
/// in the prompt text.
pub fn prompt_with_candidates<T: Clone>(
    field: &str,
    candidates: &[Candidate<T>],
    manual: impl FnOnce() -> Result<T>,
) -> Result<T> {
    if candidates.is_empty() {
        return manual();
    }
    if !Confirm::new(&format!("select {field} from the list?"))
        .with_default(true)
        .prompt()?
    {
        return manual();
    }
    let labels: Vec<&str> = candidates.iter().map(|c| c.label.as_str()).collect();
    let chosen = Select::new(&format!("{field}:"), labels).prompt()?;
    let candidate = candidates
        .iter()
        .find(|c| c.label == chosen)
        .expect("selected label must be present");
    Ok(candidate.value.clone())
}

/// Prompts which of `variants` to build, then runs its builder. Owned
/// `Box<dyn Fn>` (not `fn` pointers) so builders can capture context, and
/// so callers can assemble `variants` at runtime, not just as a literal.
pub fn variant_prompt<T>(
    label: &str,
    variants: Vec<(&str, Box<dyn Fn() -> Result<T> + '_>)>,
) -> Result<T> {
    let names: Vec<&str> = variants.iter().map(|(name, _)| *name).collect();
    let chosen = Select::new(label, names).prompt()?;
    let (_, build) = variants
        .into_iter()
        .find(|(name, _)| *name == chosen)
        .expect("selected variant must be present");
    build()
}

/// Anything carrying a signature is never typed field-by-field — paste
/// hex-encoded cbor, or pick one already available via `candidates`.
pub fn prompt_signed<T>(field: &str, candidates: &[Candidate<T>]) -> Result<T>
where
    T: Clone + for<'b> minicbor::Decode<'b, ()>,
{
    prompt_with_candidates(field, candidates, || paste_cbor(field))
}

fn paste_cbor<T: for<'b> minicbor::Decode<'b, ()>>(field: &str) -> Result<T> {
    let raw = inquire::Text::new(&format!("{field} (hex-encoded cbor):")).prompt()?;
    let bytes = hex::decode(raw.trim())?;
    minicbor::decode(&bytes).map_err(|e| anyhow::anyhow!("decoding {field}: {e}"))
}

/// Repeats `one` until the user stops adding. Defaults to "yes" on the
/// first ask (so an empty list needs an explicit no), "no" after that.
pub fn prompt_many<T>(field: &str, one: impl Fn() -> Result<T>) -> Result<Vec<T>> {
    let mut items = Vec::new();
    while Confirm::new(&format!("add a {field}?"))
        .with_default(items.is_empty())
        .prompt()?
    {
        items.push(one()?);
    }
    Ok(items)
}

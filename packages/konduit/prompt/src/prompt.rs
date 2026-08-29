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
#[allow(clippy::type_complexity)] // FIXME
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

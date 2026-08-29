use std::path::Path;

use anyhow::Context;
use cardano_connector::CardanoConnector;
use cardano_wallet::Wallet;

use crate::{Addressbook, Config, Session, Tip, tip::TipVec};

// ---- config -------------------------------------------------------------

pub fn init(path: &Path, force: bool) -> anyhow::Result<()> {
    anyhow::ensure!(
        force || !path.exists(),
        "{} already exists - pass --force to overwrite",
        path.display()
    );
    let contents =
        toml::to_string_pretty(&Config::default()).context("serializing default config")?;
    std::fs::write(path, contents).with_context(|| format!("writing {}", path.display()))?;
    print_json(&serde_json::json!({ "wrote": path.display().to_string() }))
}

pub fn load_config(path: &Path) -> anyhow::Result<Config> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("reading config at {} - run `init` first?", path.display()))?;
    toml::from_str(&contents).with_context(|| format!("parsing config at {}", path.display()))
}

// ---- tip cache ------------------------------------------------------------

/// `Tip::default()` if never cached - same "missing means default"
/// contract as `load_addressbook`, so callers never need to branch on
/// whether a cache file exists yet.
pub fn load_tip(path: &Path) -> anyhow::Result<Tip> {
    match std::fs::read_to_string(path) {
        Ok(contents) => Ok(serde_json::from_str::<TipVec>(&contents)
            .with_context(|| format!("parsing {}", path.display()))?
            .into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Tip::default()),
        Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
    }
}

pub fn save_tip(tip: &Tip, path: &Path) -> anyhow::Result<()> {
    let contents =
        serde_json::to_string_pretty(&TipVec::from(tip.clone())).context("serializing tip")?;
    std::fs::write(path, contents).with_context(|| format!("writing {}", path.display()))
}

// ---- addressbook ----------------------------------------------------------

pub fn load_addressbook(path: &Path) -> anyhow::Result<Addressbook> {
    Addressbook::load(path).map_err(Into::into)
}

pub fn save_addressbook(book: &Addressbook, path: &Path) -> anyhow::Result<()> {
    book.save(path).map_err(Into::into)
}

// ---- hydrating/persisting a session - shared by every `cli` binary, not
// just this crate's own -----------------------------------------------------

/// Hydrates `session`'s tip/addressbook from the cache files, then
/// refreshes from the chain if `force_refresh`. Call once, right after
/// `Session::init`/`new` and before dispatching to any subcommand.
pub async fn hydrate<C: CardanoConnector, W: Wallet>(
    session: &mut Session<C, W>,
    tip_cache_path: &Path,
    addressbook_path: &Path,
    force_refresh: bool,
) -> anyhow::Result<()> {
    session.load_tip(load_tip(tip_cache_path)?);
    session.load_addressbook(load_addressbook(addressbook_path)?)?;
    if force_refresh {
        session.refresh_all().await?;
    }
    Ok(())
}

/// Best-effort persists `tip`/`addressbook` back to disk - a caching
/// failure is only ever a warning, never masks a command's own result.
/// Takes the two pieces directly (rather than a whole `Session`) so it
/// works the same for this crate's own `Session` and for anything that
/// wraps one and exposes `.tip()`/`.addressbook()` through `Deref`.
pub fn persist(
    tip: &Tip,
    addressbook: &Addressbook,
    tip_cache_path: &Path,
    addressbook_path: &Path,
) {
    if let Err(e) = save_tip(tip, tip_cache_path) {
        eprintln!("warning: failed to cache tip: {e}");
    }
    if let Err(e) = save_addressbook(addressbook, addressbook_path) {
        eprintln!("warning: failed to save addressbook: {e}");
    }
}

// ---- shared presentation ----------------------------------------------------

// Shared by the `wallet`, `addressbook`, `tip`, and `refresh` adapters too.
pub(super) fn print_json(value: &impl serde::Serialize) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

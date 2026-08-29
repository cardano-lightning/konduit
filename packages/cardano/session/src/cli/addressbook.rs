//! Addressbook management subcommands. Operates on a bare `Addressbook`,
//! not a `Session` - pure local file editing, no connector or wallet
//! needed.

use anyhow::Context;
use cardano_sdk::{Address, address::kind::Shelley};

use crate::Addressbook;

use super::cmd::print_json;

#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    Insert {
        label: String,
        address: Address<Shelley>,
    },
    /// By label or address.
    Remove {
        label_or_address: String,
    },
    List,
}

impl Cmd {
    pub fn run(&self, book: &mut Addressbook) -> anyhow::Result<()> {
        match self {
            Cmd::Insert { label, address } => {
                book.insert(label.clone(), address.to_owned())?;
                print_json(&serde_json::json!({ "inserted": label }))
            }
            Cmd::Remove { label_or_address } => {
                let address = book.resolve(label_or_address)?;
                let label = book
                    .get_label(&address)
                    .with_context(|| format!("{label_or_address:?} has no addressbook entry"))?;
                let address = book.remove(&label)?;
                print_json(&serde_json::json!({ "removed": label, "address": address.to_string() }))
            }
            Cmd::List => {
                let entries: std::collections::BTreeMap<_, _> = book
                    .iter()
                    .map(|(label, address)| (label.to_string(), address.to_string()))
                    .collect();
                print_json(&entries)
            }
        }
    }
}

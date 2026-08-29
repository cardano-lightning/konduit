//! Tracking and reading tip data - everything under `tip`. Syncing
//! (`refresh`) is a sibling top-level command, not part of this enum.

use cardano_connector::CardanoConnector;
use cardano_wallet::Wallet as WalletTrait;

use crate::Session;

use super::cmd::print_json;

#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    /// Labels an address and starts tracking it, fetching its UTXOs
    /// immediately. The label is mandatory - there's no unlabelled
    /// tracking any more, so name it whatever you like.
    Track {
        /// Name to track the address under.
        label: String,
        /// Address to track - a literal address, or an existing label
        /// to resolve one from.
        address: String,
    },
    /// By label or address.
    Untrack {
        label_or_address: String,
    },
    WaitId {
        id: String,
    },
    /// Cached UTXOs at an address (label or literal) - no network call.
    /// Errors if not yet synced; see `refresh at`.
    At {
        label_or_address: String,
    },
    /// Tracked-address and cached-UTXO-count summary.
    Info,
}

impl Cmd {
    pub async fn run<C: CardanoConnector, W: WalletTrait>(
        &self,
        session: &mut Session<C, W>,
    ) -> anyhow::Result<()> {
        match self {
            Cmd::Track { label, address } => {
                let resolved = session.resolve(address)?;
                session.track(label.clone(), resolved.clone())?;
                let address = resolved.to_string();
                session.refresh_at(resolved).await?;
                print_json(&serde_json::json!({ "tracked": label, "address": address }))
            }

            Cmd::Untrack { label_or_address } => {
                session.untrack(label_or_address)?;
                print_json(&serde_json::json!({ "untracked": label_or_address }))
            }

            Cmd::WaitId { id } => {
                let id = cardano_sdk::Hash::<32>::try_from(id.as_str())?;
                session.wait_wallet(&id).await?;
                print_json(&serde_json::json!({ "confirmed": true }))
            }

            // Errors rather than printing an empty list if `label_or_address`
            // was never refreshed, since `[]` would otherwise be ambiguous
            // between "empty" and "not synced".
            Cmd::At { label_or_address } => {
                let utxos = session.utxos_of(label_or_address)?.ok_or_else(|| {
                    anyhow::anyhow!(
                        "no cached utxos for {label_or_address:?} - run `refresh at {label_or_address}` first"
                    )
                })?;
                print_json(&utxos.iter().collect::<Vec<_>>())
            }

            Cmd::Info => {
                let counts: Vec<(String, Option<String>, usize)> = session
                    .tracked()
                    .map(|address| {
                        let utxos = session.utxos_at(address).map_or(0, |m| m.len());
                        (
                            address.to_string(),
                            session.addressbook().get_label(address),
                            utxos,
                        )
                    })
                    .collect();
                let total_utxos: usize = counts.iter().map(|(_, _, n)| n).sum();
                print_json(&serde_json::json!({
                    "tracked_addresses": counts.len(),
                    "total_utxos": total_utxos,
                    "addresses": counts.iter().map(|(address, label, utxos)| {
                        serde_json::json!({ "address": address, "label": label, "utxos": utxos })
                    }).collect::<Vec<_>>(),
                }))
            }
        }
    }
}

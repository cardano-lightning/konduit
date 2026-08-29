#![cfg(feature = "cli")]

use cardano_sdk::{
    Address, Hash, PlutusScript, PlutusVersion, ProtocolParameters, Transaction, Value,
    address::kind, transaction::state,
};

use crate::{Wallet, cbor, txs};

#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    /// Write a default config template to --config.
    Init,
    /// Own address
    Address,
    /// All utxos
    Utxos,
    /// Non-script utxos - the ones spendable as plain inputs.
    Fuel,
    /// Script-bearing utxos, as (input, script hash) - not the full script.
    ReferenceScripts,
    Sign {
        tx_hex: String,
    },
    Submit {
        tx_hex: String,
    },
    /// Send lovelace to a single address.
    Send {
        to: String,
        lovelace: u64,
    },
    /// Sweep every wallet UTXO, including script-bearing ones, to `to`.
    Empty {
        to: String,
    },
    /// Upload a script as a reference script at the wallet's own address.
    Upload {
        /// Script bytes, as hex.
        script: ScriptBytes,
        /// Plutus version (1, 2, or 3). Defaults to V3.
        #[arg(default_value = "3", value_parser = parse_plutus_version)]
        version: PlutusVersion,
    },
    /// Spend a reference script back into the wallet. Inverse of `upload`.
    Teardown {
        hash: String,
    },
}

/// Wraps `Vec<u8>` purely so clap treats `script` as one opaque value -
#[derive(Debug, Clone)]
pub struct ScriptBytes(pub Vec<u8>);

impl std::str::FromStr for ScriptBytes {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        hex::decode(s).map(ScriptBytes).map_err(|e| e.to_string())
    }
}

/// Numeric Plutus version ("1"/"2"/"3") -> `PlutusVersion`. clap always
/// hands parsers the raw `&str`, never a pre-parsed numeric type, so this
/// does the `u8` parse itself before the `TryFrom`.
fn parse_plutus_version(s: &str) -> Result<PlutusVersion, String> {
    let v: u8 = s
        .parse()
        .map_err(|_| format!("{s:?} is not a valid Plutus version number"))?;
    PlutusVersion::try_from(v)
        .map_err(|_| format!("{v} is not a known Plutus version (expected 1, 2, or 3)"))
}

impl Cmd {
    pub async fn run<W: Wallet>(
        &self,
        wallet: &W,
        protocol_parameters: &ProtocolParameters,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        Ok(match self {
            Cmd::Init => unreachable!("handled before wallet construction"),

            Cmd::Address => serde_json::to_value(wallet.change_address().await?)?,

            Cmd::Utxos => {
                let utxos = wallet
                    .utxos(None)
                    .await?
                    .map(|m| m.into_iter().collect::<Vec<_>>());
                serde_json::to_value(utxos)?
            }

            Cmd::Fuel => {
                let utxos = wallet.utxos(None).await?.unwrap_or_default();
                let fuel: Vec<_> = utxos
                    .into_iter()
                    .filter(|(_, o)| o.script().is_none())
                    .collect();
                serde_json::to_value(fuel)?
            }

            Cmd::ReferenceScripts => {
                let utxos = wallet.utxos(None).await?.unwrap_or_default();
                let scripts: Vec<_> = utxos
                    .into_iter()
                    .filter_map(|(input, output)| {
                        output
                            .script()
                            .map(|script| (input, Hash::<28>::from(script)))
                    })
                    .collect();
                serde_json::to_value(scripts)?
            }

            Cmd::Sign { tx_hex } => {
                let tx: Transaction<state::ReadyForSigning> = cbor::from_cbor_hex(tx_hex)?;
                let (vkey, signature) = wallet.sign_tx(&tx).await?;
                serde_json::json!({ "vkey": vkey, "signature": signature })
            }

            Cmd::Submit { tx_hex } => {
                let tx: Transaction<state::ReadyForSigning> = cbor::from_cbor_hex(tx_hex)?;
                serde_json::json!({ "tx_id": wallet.submit(&tx).await? })
            }

            Cmd::Send { to, lovelace } => {
                let to: Address<kind::Any> = to.parse()?;
                let change = wallet.change_address().await?;
                let utxos = wallet
                    .utxos(None)
                    .await?
                    .ok_or("wallet has no utxos - nothing to spend from")?;
                let tx = txs::send(
                    protocol_parameters,
                    &utxos,
                    vec![(to, Value::new(*lovelace))],
                    change,
                )?;
                let id = sign_and_submit(wallet, tx).await?;
                serde_json::json!({ "submitted": id })
            }

            Cmd::Empty { to } => {
                let to: Address<kind::Any> = to.parse()?;
                let utxos = wallet
                    .utxos(None)
                    .await?
                    .ok_or("wallet has no utxos - nothing to sweep")?;
                let tx = txs::empty(protocol_parameters, &utxos, to)?;
                let id = sign_and_submit(wallet, tx).await?;
                serde_json::json!({ "submitted": id })
            }

            Cmd::Upload { script, version } => {
                // ASSUMPTION: `PlutusScript::new(version, bytes)` - adjust
                // if the real constructor differs.
                let plutus_script = PlutusScript::new(*version, script.0.clone());
                let change = wallet.change_address().await?;
                let utxos = wallet
                    .utxos(None)
                    .await?
                    .ok_or("wallet has no utxos - nothing to spend from")?;
                let tx = txs::upload(protocol_parameters, &utxos, plutus_script, change)?;
                let id = sign_and_submit(wallet, tx).await?;
                serde_json::json!({ "submitted": id })
            }

            Cmd::Teardown { hash } => {
                let hash: Hash<28> = hash.parse()?;
                let change = wallet.change_address().await?;
                let utxos = wallet
                    .utxos(None)
                    .await?
                    .ok_or("wallet has no utxos - nothing to spend from")?;
                let tx = txs::teardown(protocol_parameters, &utxos, hash, change)?;
                let id = sign_and_submit(wallet, tx).await?;
                serde_json::json!({ "submitted": id })
            }
        })
    }
}

async fn sign_and_submit<W: Wallet>(
    wallet: &W,
    mut tx: Transaction<state::ReadyForSigning>,
) -> Result<Hash<32>, Box<dyn std::error::Error>> {
    let (vkey, sig) = wallet.sign_tx(&tx).await?;
    tx.add_witness(vkey, sig);
    Ok(wallet.submit(&tx).await?)
}

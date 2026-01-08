use std::fmt;

use cardano_connect::CardanoConnect;
use cardano_tx_builder::{Address, Hash, Value, address::kind};
use konduit_tx::{KONDUIT_VALIDATOR, Utxo, Utxos};

use crate::config::{self};

pub struct Admin {
    wallet: Utxos,
    reference_script: Option<Utxo>,
}

impl Admin {
    pub const LABEL: &str = "ADMIN";

    pub async fn new(
        connector: &impl CardanoConnect,
        config: &config::admin::Config,
    ) -> anyhow::Result<Self> {
        let wallet = connector
            .utxos_at(&config.host_address.payment(), None)
            .await?;
        let reference_script = get_script(connector, &config.host_address).await?;
        Ok(Self {
            wallet,
            reference_script,
        })
    }
}

async fn get_script(
    connector: &impl CardanoConnect,
    host_address: &Address<kind::Shelley>,
) -> anyhow::Result<Option<Utxo>> {
    let payment = host_address.payment();
    let delegation = host_address.delegation();
    let utxos = connector.utxos_at(&payment, delegation.as_ref()).await?;
    Ok(utxos.into_iter().find(|(_, o)| {
        o.script()
            .is_some_and(|script| script == &KONDUIT_VALIDATOR.script)
    }))
}

impl fmt::Display for Admin {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "== Tip :: {} ==\n", Self::LABEL)?;
        write!(f, "Wallet ")?;
        display_utxos(f, &self.wallet)?;
        write!(f, "Reference script ")?;
        display_reference_script(f, &self.reference_script)?;
        Ok(())
    }
}

fn display_reference_script(f: &mut fmt::Formatter, u: &Option<Utxo>) -> fmt::Result {
    match u {
        Some(u) => {
            if let Some(script) = u.1.script() {
                write!(f, "\n{}#{}\n", u.0.transaction_id(), u.0.output_index())?;
                if f.alternate() {
                    write!(f, " - script ver: {:#}\n", script.version())?;
                    write!(f, " - script hash: {:#}\n", Hash::<28>::from(script))?;
                } else {
                    write!(f, " - script ver: {:#}\n", script.version())?;
                    write!(f, " - script hash: {:#}\n", Hash::<28>::from(script))?;
                }
            } else {
                write!(f, "Utxo {} has no script!!", u.0)?;
            }
        }
        None => write!(f, " None found")?,
    };
    Ok(())
}

// Assume the address is deduced from context
fn display_utxos(f: &mut fmt::Formatter, us: &Utxos) -> fmt::Result {
    if f.alternate() {
        // Verbose
        write!(f, "utxos:\n")?;
        for (i, o) in us.iter() {
            write!(f, " => {}#{}\n", i.transaction_id(), i.output_index())?;
            write!(f, " - value : {:#}\n", o.value())?;
            if let Some(datum) = o.datum() {
                match datum {
                    cardano_tx_builder::Datum::Hash(hash) => write!(f, " - datum hash : {}", hash)?,
                    cardano_tx_builder::Datum::Inline(data) => {
                        write!(f, " - datum inline: {}\n", &data.to_string()[0..100])?
                    }
                }
            }
            if let Some(script) = o.script() {
                write!(f, " - script ver: {:#}\n", script.version())?;
                write!(f, " - script hash: {:#}\n", Hash::<28>::from(script))?;
            }
        }
    } else {
        write!(f, "summary:\n")?;
        let count = us.len();
        let value = us.values().fold(Value::new(0), |acc, curr| {
            let mut acc = acc;
            acc.add(curr.value());
            acc
        });
        let datum = if us
            .values()
            .fold(false, |acc, curr| acc || curr.datum().is_some())
        {
            "some"
        } else {
            "no"
        };
        let script = if us
            .values()
            .fold(false, |acc, curr| acc || curr.script().is_some())
        {
            "some"
        } else {
            "no"
        };
        write!(
            f,
            "{} Utxos with {} datum(s) and {} script(s)\n",
            count, datum, script
        )?;
        write!(f, "Total : {}\n", value)?;
    };
    Ok(())
}

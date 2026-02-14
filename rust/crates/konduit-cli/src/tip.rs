use std::{collections::BTreeMap, fmt};

use cardano_connect::CardanoConnect;
use cardano_tx_builder::{Address, Credential, Hash, Input, Value, address::kind};
use konduit_data::{Keytag, Pending, Used};
use konduit_tx::{ChannelOutput, KONDUIT_VALIDATOR, Utxo, Utxos, filter_channels};

use crate::config::{self};

pub struct Consumer {
    wallet: Utxos,
    reference_script: Option<Utxo>,
    channels: BTreeMap<Input, ChannelOutput>,
}

impl Consumer {
    pub const LABEL: &str = "Consumer";

    pub async fn new(
        connector: &impl CardanoConnect,
        config: &config::consumer::Config,
    ) -> anyhow::Result<Self> {
        let add_vkey = config.wallet.to_verification_key();
        let own_address = add_vkey.to_address(connector.network().into());
        let wallet = connector
            .utxos_at(&own_address.payment(), own_address.delegation().as_ref())
            .await?;
        let reference_script = get_script(connector, &config.host_address).await?;
        // FIXME :: NO STAKING
        let konduit_utxos = connector
            .utxos_at(&Credential::from_script(KONDUIT_VALIDATOR.hash), None)
            .await?;
        let channels = filter_channels(&konduit_utxos, |co| co.constants.add_vkey == add_vkey)
            .into_iter()
            .collect();
        Ok(Self {
            wallet,
            reference_script,
            channels,
        })
    }
}

impl fmt::Display for Consumer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "== Tip :: {} ==\n", Self::LABEL)?;
        write!(f, "Wallet ")?;
        display_utxos(f, &self.wallet)?;
        write!(f, "Reference script ")?;
        display_reference_script(f, &self.reference_script)?;
        write!(f, "Channels : {}\n", self.channels.len())?;
        for (input, channel) in self.channels.iter() {
            write!(f, "  Input : {}\n", input)?;
            write!(f, "  Tag : {}\n", channel.constants.tag)?;
            write!(
                f,
                "  Sub : {} || Close Period : {} \n",
                channel.constants.sub_vkey, channel.constants.close_period
            )?;
            display_stage(f, &channel.stage)?;
            write!(f, "  Amt : {} \n", channel.amount)?;
        }
        Ok(())
    }
}
pub struct Adaptor {
    wallet: Utxos,
    reference_script: Option<Utxo>,
    channels: BTreeMap<Input, ChannelOutput>,
}

impl Adaptor {
    pub const LABEL: &str = "Adaptor";

    pub async fn new(
        connector: &impl CardanoConnect,
        config: &config::adaptor::Config,
    ) -> anyhow::Result<Self> {
        let sub_vkey = config.wallet.to_verification_key();
        let own_address = sub_vkey.to_address(connector.network().into());
        let wallet = connector
            .utxos_at(&own_address.payment(), own_address.delegation().as_ref())
            .await?;
        let reference_script = get_script(connector, &config.host_address).await?;
        let konduit_utxos = connector
            .utxos_at(&Credential::from_script(KONDUIT_VALIDATOR.hash), None)
            .await?;
        let channels = filter_channels(&konduit_utxos, |co| co.constants.sub_vkey == sub_vkey)
            .into_iter()
            .collect();
        Ok(Self {
            wallet,
            reference_script,
            channels,
        })
    }
}

impl fmt::Display for Adaptor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "== Tip :: {} ==\n", Self::LABEL)?;
        write!(f, "Wallet ")?;
        display_utxos(f, &self.wallet)?;
        write!(f, "Reference script ")?;
        display_reference_script(f, &self.reference_script)?;
        write!(f, "Channels : {}\n", self.channels.len())?;
        for (input, channel) in self.channels.iter() {
            write!(f, "  Input : {}\n", input)?;
            write!(
                f,
                "  Keytag : {}\n",
                Keytag::new(channel.constants.add_vkey, channel.constants.tag.clone())
            )?;
            display_stage(f, &channel.stage)?;
            write!(f, "  Amt : {} \n", channel.amount)?;
        }
        Ok(())
    }
}

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
        let own_address = config
            .wallet
            .to_verification_key()
            .to_address(connector.network().into());
        let wallet = connector
            .utxos_at(&own_address.payment(), own_address.delegation().as_ref())
            .await?;
        let reference_script = get_script(connector, &config.host_address).await?;
        Ok(Self {
            wallet,
            reference_script,
        })
    }
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

// Assume the address is deduced from context
// FIXME :: Is this worth keeping??
#[allow(dead_code)]
fn display_channels(f: &mut fmt::Formatter, us: &Utxos) -> fmt::Result {
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

fn useds_to_string(useds: &Vec<Used>) -> String {
    if useds.len() == 0 {
        "[NONE]".to_string()
    } else {
        useds
            .iter()
            .map(|x| format!("[{},{}]", x.index, x.amount))
            .collect::<Vec<_>>()
            .join(",")
    }
}

fn pendings_to_string(pendings: &Vec<Pending>) -> String {
    if pendings.len() == 0 {
        "[NONE]".to_string()
    } else {
        pendings
            .iter()
            .map(|x| format!("[{},{},{}]", x.amount, x.timeout, hex::encode(x.lock.0)))
            .collect::<Vec<_>>()
            .join(",")
    }
}

fn display_stage(f: &mut fmt::Formatter<'_>, stage: &konduit_data::Stage) -> fmt::Result {
    match stage {
        konduit_data::Stage::Opened(subbed, useds) => {
            write!(f, "  Opened : {} : {} \n", subbed, useds_to_string(useds))
        }
        konduit_data::Stage::Closed(subbed, useds, elapse_at) => write!(
            f,
            "  Closed : {} : {} : {} \n",
            subbed,
            useds_to_string(useds),
            elapse_at
        ),
        konduit_data::Stage::Responded(pendings_amount, pendings) => write!(
            f,
            "  Responded : {} : {} \n",
            pendings_amount,
            pendings_to_string(pendings)
        ),
    }
}

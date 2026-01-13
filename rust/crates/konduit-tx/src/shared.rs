use std::{
    cmp,
    collections::BTreeMap,
    sync::LazyLock,
    time::{SystemTime, UNIX_EPOCH},
};

use cardano_tx_builder::{
    Address, Credential, Hash, Input, NetworkId, Output, PlutusData, ProtocolParameters, Value,
    VerificationKey,
};
use konduit_data::{Constants, Datum, Duration, L1Channel, Stage};

use crate::KONDUIT_VALIDATOR;

pub type Lovelace = u64;

pub type Utxo = (Input, Output);

pub type Utxos = BTreeMap<Input, Output>;

/// The default Time-To-Live for transactions. This impacts the upper bound of transactions and the
/// overall 'speed' at which transitions between stages can happen.
///
/// While this can _in theory_ be as low as 1s... setting it too low will increase the likelihood of
/// a transaction to fail to submit (due to blocks following a random distribution).
pub static DEFAULT_TTL: LazyLock<Duration> = LazyLock::new(|| Duration::from_secs(120));

pub const MIN_ADA_BUFFER: Lovelace = 2_000_000;

pub const FEE_BUFFER: Lovelace = 3_000_000;

pub struct NetworkParameters {
    pub network_id: NetworkId,
    pub protocol_parameters: ProtocolParameters,
}

pub struct Bounds {
    pub lower: Duration,
    pub upper: Duration,
}

impl Bounds {
    pub fn twenty_mins() -> Self {
        // TODO :: Either use std time, or upstream methods
        let lower = Duration::from_secs(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                // Hack to handle blockfrost slots not aligning with current time.
                .saturating_sub(60),
        );
        let upper = Duration::from_secs(lower.as_secs() + 19 * 60);
        Bounds { lower, upper }
    }
}

pub struct OptionalBounds {
    lower: Option<Duration>,
    upper: Option<Duration>,
}

impl OptionalBounds {
    pub fn low(&mut self, bound: Duration) {
        self.lower = Some(self.lower.map_or(bound, |curr| cmp::max(curr, bound)));
    }
    pub fn up(&mut self, bound: Duration) {
        self.upper = Some(self.upper.map_or(bound, |curr| cmp::min(curr, bound)));
    }
    pub fn union(&self, other: &Self) -> Self {
        let lower = self
            .lower
            .map(|l| other.lower.map_or(l, |r| r.min(l)))
            .or(other.lower);
        let upper = self
            .upper
            .map(|l| other.upper.map_or(l, |r| r.min(l)))
            .or(other.upper);
        Self { lower, upper }
    }
}

#[derive(Debug, Clone)]
pub struct ChannelOutput {
    pub amount: u64,
    pub constants: Constants,
    pub stage: Stage,
}

impl ChannelOutput {
    pub fn to_l1_channel(&self) -> L1Channel {
        L1Channel {
            amount: self.amount.clone(),
            stage: self.stage.clone(),
        }
    }

    pub fn from_l1_channel(l1: L1Channel, constants: Constants) -> Self {
        Self {
            amount: l1.amount,
            constants,
            stage: l1.stage,
        }
    }

    pub fn to_datum(&self) -> konduit_data::Datum {
        konduit_data::Datum {
            own_hash: KONDUIT_VALIDATOR.hash,
            constants: self.constants.clone(),
            stage: self.stage.clone(),
        }
    }

    pub fn from_output(o: &Output) -> Option<Self> {
        if o.address().as_shelley()?.payment().as_script()? != KONDUIT_VALIDATOR.hash {
            return None;
        }
        let cardano_tx_builder::Datum::Inline(data) = o.datum()? else {
            return None;
        };
        let Datum {
            own_hash,
            constants,
            stage,
        } = Datum::try_from(data).ok()?;
        if own_hash != KONDUIT_VALIDATOR.hash {
            return None;
        }
        Some(Self {
            amount: extract_amount(o.value()),
            constants,
            stage,
        })
    }

    pub fn to_output(&self, network_id: &NetworkId, credential: &Option<Credential>) -> Output {
        let mut address = Address::new(
            network_id.clone(),
            Credential::from_script(KONDUIT_VALIDATOR.hash),
        );
        if let Some(credential) = credential {
            address = address.with_delegation(credential.clone());
        };
        Output::new(address.into(), Value::new(MIN_ADA_BUFFER + self.amount))
            .with_datum(self.to_datum().into())
    }
}

pub fn extract_amount(value: &cardano_tx_builder::Value<u64>) -> u64 {
    value.lovelace().saturating_sub(MIN_ADA_BUFFER)
}

pub fn wallet_inputs(wallet: &VerificationKey, utxos: &Utxos) -> Vec<Input> {
    let wallet_credential = Credential::from_key(Hash::<28>::new(wallet));
    utxos
        .iter()
        .filter(|(_i, o)| {
            o.address()
                .as_shelley()
                .is_some_and(|a| a.payment() == wallet_credential)
        })
        .map(|(i, _o)| i.clone())
        .collect::<Vec<_>>()
}

pub fn filter_channels(
    utxos: &Utxos,
    filter: impl Fn(&ChannelOutput) -> bool,
) -> Vec<(Input, ChannelOutput)> {
    utxos
        .iter()
        .filter_map(|(i, o)| {
            ChannelOutput::from_output(o)
                .filter(|c| filter(&c))
                .map(|c| (i.clone(), c))
        })
        .collect::<Vec<_>>()
}

pub fn select_utxos(
    utxos: &Utxos,
    amount: Lovelace,
) -> anyhow::Result<Vec<(Input, Option<PlutusData<'static>>)>> {
    if amount == 0 {
        return Ok(vec![]);
    }
    let mut sorted_utxos: Vec<(&Input, &Output)> = utxos
        .iter()
        // Filter out utxos which hold reference scripts
        .filter(|(_, output)| output.script().is_none())
        // Filter out those locked by a script
        .filter(|(_, output)| {
            output
                .address()
                .as_shelley()
                .is_some_and(|addr| addr.payment().as_key().is_some())
        })
        .collect();
    sorted_utxos.sort_by_key(|(_, output)| std::cmp::Reverse(output.value().lovelace()));

    let mut selected_inputs = Vec::new();
    let mut total_lovelace: u64 = 0;

    for (input, output) in sorted_utxos {
        selected_inputs.push((input.clone(), None));
        total_lovelace = total_lovelace.saturating_add(output.value().lovelace());

        if total_lovelace >= amount {
            break;
        }
    }

    if total_lovelace < amount {
        return Err(anyhow::anyhow!(
            "insufficient funds in wallet to cover the amount"
        ));
    }

    Ok(selected_inputs)
}

pub fn konduit_reference(utxos: &Utxos) -> Option<Input> {
    utxos
        .iter()
        .find(|(_i, o)| {
            o.script()
                .is_some_and(|s| Hash::<28>::from(s) == KONDUIT_VALIDATOR.hash)
        })
        .map(|(i, _o)| i.clone())
}

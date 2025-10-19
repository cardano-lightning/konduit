use anyhow::{Result, anyhow};
use cardano_tx_builder::{
    Address, Credential, Hash, NetworkId, Output, PlutusData, Value, address,
};
use konduit_data::{
    base::{Amount, ScriptHash},
    constants::Constants,
    datum::Datum,
    stage::Stage,
};

#[derive(Debug, Clone)]
pub struct Channel {
    pub delegation: Option<Credential>,
    pub amount: Amount,
    pub constants: Constants,
    pub stage: Stage,
}

impl Channel {
    pub fn new(
        delegation: Option<Credential>,
        amount: Amount,
        constants: Constants,
        stage: Stage,
    ) -> Self {
        Self {
            delegation,
            amount,
            constants,
            stage,
        }
    }
    pub fn try_from_output(script_hash: Hash<28>, output: Output) -> Result<Self> {
        let delegation = as_channel_delegation(script_hash.clone(), output.address())?;
        let amount = as_channel_amount(output.value())?;
        let (constants, stage) = as_channel_constants_stage(script_hash, output.datum())?;
        Ok(Self {
            delegation,
            amount,
            constants,
            stage,
        })
    }

    pub fn to_output(&self, network_id: NetworkId, script_hash: Hash<28>) -> Output {
        let mut address = Address::new(network_id, Credential::from_script(script_hash.clone()));
        if let Some(delegation) = &self.delegation {
            address = address.with_delegation(delegation.clone())
        };
        let value = Value::new(self.amount.0);
        let datum = Datum::new(
            ScriptHash(script_hash.into()),
            self.constants.clone(),
            self.stage.clone(),
        );
        Output::new(address.into(), value).with_datum(PlutusData::from(datum))
    }
}

pub fn as_channel_delegation(
    script_hash: Hash<28>,
    address: &Address<address::kind::Any>,
) -> anyhow::Result<Option<Credential>> {
    if let Some(address) = address.as_shelley() {
        if address.payment() == Credential::from_script(script_hash) {
            Ok(address.delegation())
        } else {
            Err(anyhow!("Expect channel address"))
        }
    } else {
        Err(anyhow!("Expect Shelley address"))
    }
}

pub fn as_channel_amount(value: &Value<u64>) -> anyhow::Result<Amount> {
    // Support only ada for now
    if value.assets().is_empty() {
        Ok(Amount(value.lovelace()))
    } else {
        Err(anyhow!("Bad value"))
    }
}

pub fn as_channel_datum(datum: Option<&cardano_tx_builder::Datum>) -> anyhow::Result<Datum> {
    if let Some(cardano_tx_builder::Datum::Inline(plutus_data)) = datum {
        Datum::try_from(plutus_data.clone())
    } else {
        Err(anyhow!("Expect inline datum"))
    }
}

pub fn as_channel_constants_stage(
    script_hash: Hash<28>,
    datum: Option<&cardano_tx_builder::Datum>,
) -> anyhow::Result<(Constants, Stage)> {
    let Datum {
        own_hash,
        constants,
        stage,
    } = as_channel_datum(datum)?;
    if <Hash<28>>::from(own_hash.0) != script_hash {
        Err(anyhow!("Bad datum own hash"))?;
    };
    Ok((constants, stage))
}

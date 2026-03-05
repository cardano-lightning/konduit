use cardano_sdk::{Output, Value};
use konduit_data::{Constants, Datum, Stage};

use crate::{KONDUIT_VALIDATOR, MIN_ADA_BUFFER, channel_variables::Variables};

/// Data obtained from parsing a channel
#[derive(Debug, Clone)]
pub struct ChannelData {
    amount: u64,
    constants: Constants,
    stage: Stage,
}

impl ChannelData {
    pub fn new(amount: u64, constants: Constants, stage: Stage) -> Self {
        Self {
            constants,
            stage,
            amount,
        }
    }

    pub fn constants(&self) -> &Constants {
        &self.constants
    }

    pub fn stage(&self) -> &Stage {
        &self.stage
    }

    pub fn amount(&self) -> u64 {
        self.amount
    }

    /// Ada channels require min ada buffer
    pub fn buffered_amount(&self) -> u64 {
        self.amount + MIN_ADA_BUFFER
    }

    /// Ada channels require min ada buffer
    pub fn buffered_value(&self) -> Value<u64> {
        Value::new(self.buffered_amount())
    }

    /// As datum
    pub fn datum(&self) -> Datum {
        Datum {
            own_hash: KONDUIT_VALIDATOR.hash,
            constants: self.constants.clone(),
            stage: self.stage.clone(),
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("Expect Shelley Address")]
    ShelleyAddress,
    #[error("Expect Script Payment Credential")]
    ScriptCredential,
    #[error("Expect Konduit Payment Credential")]
    KonduitCredential,
    #[error("Expect datum")]
    Datum,
    #[error("Expect Inline datum")]
    Inline,
    #[error("Failed to parse datum")]
    ParseDatum,
    #[error("Own hash is wrong")]
    OwnHash,
}

impl TryFrom<&Output> for ChannelData {
    type Error = Error;

    fn try_from(output: &Output) -> Result<Self, Self::Error> {
        let Some(address) = output.address().as_shelley() else {
            return Err(Error::ShelleyAddress);
        };
        let Some(hash) = address.payment().as_script() else {
            return Err(Error::ScriptCredential);
        };
        if hash != KONDUIT_VALIDATOR.hash {
            return Err(Error::KonduitCredential);
        }
        let Some(datum) = output.datum() else {
            return Err(Error::Datum);
        };
        let cardano_sdk::Datum::Inline(data) = datum else {
            return Err(Error::Inline);
        };
        let Datum {
            own_hash,
            constants,
            stage,
        } = Datum::try_from(data).map_err(|_| Error::ParseDatum)?;
        if own_hash != KONDUIT_VALIDATOR.hash {
            return Err(Error::OwnHash);
        }
        let amount = debuffer_amount(output.value());
        Ok(Self {
            amount,
            constants,
            stage,
        })
    }
}

pub fn debuffer_amount(value: &cardano_sdk::Value<u64>) -> u64 {
    value.lovelace().saturating_sub(MIN_ADA_BUFFER)
}

impl From<&ChannelData> for Variables {
    fn from(value: &ChannelData) -> Self {
        Self::new(value.amount(), value.stage().clone())
    }
}

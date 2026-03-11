use std::{collections::BTreeMap, iter};

use cardano_sdk::{Input, Output, PlutusData, Value, VerificationKey};
use konduit_data::{Constants, Cont, Datum, Eol, Redeemer, Stage, Step};

use crate::{KONDUIT_VALIDATOR, MIN_ADA_BUFFER, Utxo, Utxos};

/// Data obtained from parsing a channel
#[derive(Debug, Clone)]
pub struct ChannelData {
    constants: Constants,
    stage: Stage,
    amount: u64,
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

impl TryFrom<Output> for Channel {
    type Error = Error;

    fn try_from(output: Output) -> Result<Self, Self::Error> {
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
        let amount = extract_amount(output.value());
        Ok(Self {
            amount,
            constants,
            stage,
        })
    }
}

pub fn extract_amount(value: &cardano_sdk::Value<u64>) -> u64 {
    value.lovelace().saturating_sub(MIN_ADA_BUFFER)
}

#[derive(Debug, Clone)]
pub struct ChannelUtxo {
    input: Input,
    output: Output,
    channel: Channel,
}

#[derive(Debug, Clone)]
pub enum Stepping {
    Cont(Cont, u64, Stage),
    Eol(Eol),
}

impl Stepping {
    pub fn step(&self) -> Step {
        match &self {
            Self::Cont(cont, _, _) => Step::Cont(cont.clone()),
            Self::Eol(eol) => Step::Eol(eol.clone()),
        }
    }

    pub fn cont(&self) -> Option<(u64, Stage)> {
        match &self {
            Self::Cont(_, amount, stage) => Some((amount.clone(), stage.clone())),
            Self::Eol(_) => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChannelStepping {
    utxo: ChannelUtxo,
    stepping: Stepping,
}

fn konduit_datum(constants: Constants, stage: Stage) -> Datum {
    Datum {
        own_hash: KONDUIT_VALIDATOR.hash,
        constants,
        stage,
    }
}

impl ChannelStepping {
    pub fn new(utxo: ChannelUtxo, stepping: Stepping) -> Self {
        Self { utxo, stepping }
    }

    pub fn input(&self) -> &Input {
        &self.utxo.input
    }

    pub fn output(&self) -> &Output {
        &self.utxo.output
    }

    pub fn constants(&self) -> &Constants {
        &self.utxo.channel.constants
    }

    pub fn utxo(&self) -> Utxo {
        (self.utxo.input.clone(), self.utxo.output.clone())
    }

    pub fn step(&self) -> Step {
        self.stepping.step()
    }

    pub fn cont_output(&self) -> Option<Output> {
        self.stepping.cont().map(|(amount, stage)| {
            Output::new(self.utxo.output.address().clone(), Value::new(amount)).with_datum(
                PlutusData::from(konduit_datum(self.constants().clone(), stage)),
            )
        })
    }

    pub fn signer(&self) -> &VerificationKey {
        if self.step().is_adaptor() {
            &self.utxo.channel.constants.sub_vkey
        } else {
            &self.utxo.channel.constants.add_vkey
        }
    }
}

pub struct ChannelSteppings(Vec<ChannelStepping>);

impl ChannelSteppings {
    pub fn new(v: Vec<ChannelStepping>) -> Self {
        let mut u = v;
        u.sort_by_key(|x| x.utxo.input.clone());
        Self(u)
    }

    pub fn steps(&self) -> Vec<Step> {
        self.0.iter().map(|x| x.step()).collect::<Vec<_>>()
    }

    pub fn inputs(&self) -> Vec<(Input, Redeemer)> {
        match &self.0[..] {
            [] => vec![],
            [main, rest @ ..] => {
                iter::once((main.utxo.input.clone(), Redeemer::Main(self.steps())))
                    .chain(rest.iter().map(|x| (x.utxo.input.clone(), Redeemer::Defer)))
                    .collect::<Vec<_>>()
            }
        }
    }

    pub fn utxos(&self) -> Utxos {
        self.0
            .iter()
            .map(|x| (x.utxo.input.clone(), x.utxo.output.clone()))
            .collect::<BTreeMap<_, _>>()
    }

    pub fn signers(&self) -> Vec<VerificationKey> {
        self.0
            .iter()
            .map(|x| x.signer().clone())
            .collect::<Vec<_>>()
    }

    pub fn conts(&self) -> Vec<Output> {
        self.0
            .iter()
            .filter_map(|x| x.cont_output())
            .collect::<Vec<_>>()
    }
}

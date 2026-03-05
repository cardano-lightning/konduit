use cardano_sdk::{Address, Output, address::kind::Shelley};
use konduit_data::{Constants, Keytag, L1Channel, Stage};

use crate::KONDUIT_VALIDATOR;

#[derive(Debug, Clone)]
pub struct ChannelOutput {
    pub constants: Constants,
    pub stage: Stage,
    pub amount: u64,
}

impl ChannelOutput {
    pub fn keytag(&self) -> Keytag {
        Keytag::new(self.constants.add_vkey, self.constants.tag.clone())
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
        let cardano_sdk::Datum::Inline(data) = o.datum()? else {
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
        let mut address =
            Address::new(*network_id, Credential::from_script(KONDUIT_VALIDATOR.hash));
        if let Some(credential) = credential {
            address = address.with_delegation(credential.clone());
        };
        Output::new(address.into(), Value::new(MIN_ADA_BUFFER + self.amount))
            .with_datum(self.to_datum().into())
    }

    pub fn to_output_from_address(&self, address: Address<kind::Any>) -> Output {
        Output::new(address.into(), Value::new(MIN_ADA_BUFFER + self.amount))
            .with_datum(self.to_datum().into())
    }
}

use cardano_sdk::{Address, Credential, NetworkId, Output, PlutusData, address::kind};
use konduit_data::{Constants, Stage};

use crate::{Channel, Variables, konduit_address};

#[derive(Debug, Clone)]
pub struct Open {
    channel: Channel,
    delegation: Option<Credential>,
}

impl Open {
    pub fn new(amount: u64, constants: Constants, delegation: Option<Credential>) -> Self {
        let variables = Variables::new(amount, Stage::Opened(0, vec![]));
        Self {
            channel: Channel::new(constants, variables),
            delegation,
        }
    }

    /// Specify any kind of output. Can start a channel "mid-lifecycle".
    pub fn new_raw(channel: Channel, delegation: Option<Credential>) -> Self {
        Self {
            channel,
            delegation,
        }
    }

    pub fn data(&self) -> &Channel {
        &self.channel
    }

    pub fn delegation(&self) -> Option<&Credential> {
        self.delegation.as_ref()
    }

    pub fn buffered_amount(&self) -> u64 {
        self.data().buffered_amount()
    }

    pub fn address(&self, network_id: NetworkId) -> Address<kind::Shelley> {
        konduit_address(network_id, self.delegation())
    }

    pub fn output(&self, network_id: NetworkId) -> Output {
        Output::new(
            self.address(network_id).into(),
            self.data().buffered_value(),
        )
        .with_datum(PlutusData::from(self.data().datum()))
    }
}

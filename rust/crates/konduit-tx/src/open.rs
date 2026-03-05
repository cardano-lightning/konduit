use cardano_sdk::{Address, Credential, NetworkId, Output, PlutusData, address::kind};
use konduit_data::{Constants, Datum, Stage};

use crate::{ChannelData, konduit_address};

#[derive()]
pub struct Open(ChannelData, Option<Credential>);

impl Open {
    pub fn new(amount: u64, constants: Constants, delegation: Option<Credential>) -> Self {
        Self(
            ChannelData::new(amount, constants, Stage::Opened(0, vec![])),
            delegation,
        )
    }

    /// Specify any kind of output. Can start a channel "mid-lifecycle".
    pub fn new_raw(data: ChannelData, delegation: Option<Credential>) -> Self {
        Self(data, delegation)
    }

    pub fn data(&self) -> &ChannelData {
        &self.0
    }

    pub fn delegation(&self) -> Option<&Credential> {
        self.1.as_ref()
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

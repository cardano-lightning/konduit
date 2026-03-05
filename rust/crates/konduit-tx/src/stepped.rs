use crate::{ChannelData, Stepping};

/// Encapsulate the current channel data, together with a valid stepping.
#[derive(Debug, Clone)]
pub struct Stepped {
    data: ChannelData,
    stepping: Stepping,
}

impl Stepped {
    pub fn new(data: ChannelData, stepping: Stepping) -> Self {
        Self { data, stepping }
    }

    pub fn data(&self) -> &ChannelData {
        &self.data
    }

    pub fn stepping(&self) -> &Stepping {
        &self.stepping
    }

    pub fn cont_data(&self) -> Option<ChannelData> {
        self.stepping.variables().map(|v| {
            ChannelData::new(
                v.amount().clone(),
                self.data.constants().clone(),
                v.stage().clone(),
            )
        })
    }
}

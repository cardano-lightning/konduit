use crate::{Bounds, Channel, StepTo};

/// Encapsulate the current channel data, together with a valid stepping.
#[derive(Debug, Clone)]
pub struct Stepped {
    channel: Channel,
    step_to: StepTo,
    bounds: Bounds,
}

impl Stepped {
    pub fn new(channel: Channel, step_to: StepTo, bounds: Bounds) -> Self {
        Self {
            channel,
            step_to,
            bounds,
        }
    }

    pub fn channel(&self) -> &Channel {
        &self.channel
    }

    pub fn step_to(&self) -> &StepTo {
        &self.step_to
    }

    pub fn bounds(&self) -> &Bounds {
        &self.bounds
    }

    pub fn cont_data(&self) -> Option<Channel> {
        self.step_to
            .variables()
            .map(|v| Channel::new(self.channel.constants().clone(), v.clone()))
    }

    pub fn gain(&self) -> i64 {
        let cont_amount = self.step_to.variables().map_or(0, |v| v.amount());
        self.channel().amount() as i64 - cont_amount as i64
    }
}

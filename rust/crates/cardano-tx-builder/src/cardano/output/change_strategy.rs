//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{Address, Output, Value, address};
use anyhow::anyhow;
use std::collections::VecDeque;

pub struct ChangeStrategy(
    #[allow(clippy::type_complexity)]
    Box<dyn FnOnce(Value<u64>, &mut VecDeque<Output>) -> anyhow::Result<()>>,
);

// --------------------------------------------------------------------- Running

impl ChangeStrategy {
    pub fn with(self, change: Value<u64>, outputs: &mut VecDeque<Output>) -> anyhow::Result<()> {
        self.0(change, outputs)
    }
}

// -------------------------------------------------------------------- Building

impl Default for ChangeStrategy {
    fn default() -> Self {
        Self::new(|_change, _outputs| {
            Err(anyhow!(
                "no explicit change strategy defined; use 'with_change_strategy' to define one."
            ))
        })
    }
}

impl ChangeStrategy {
    pub fn new(
        strategy: impl FnOnce(Value<u64>, &mut VecDeque<Output>) -> anyhow::Result<()> + 'static,
    ) -> Self {
        Self(Box::new(strategy))
    }

    pub fn as_last_output(change_address: Address<address::Any>) -> Self {
        Self::new(move |change, outputs| {
            outputs.push_back(Output::new(change_address, change));
            Ok(())
        })
    }

    pub fn as_first_output(change_address: Address<address::Any>) -> Self {
        Self::new(move |change, outputs| {
            outputs.push_front(Output::new(change_address, change));
            Ok(())
        })
    }
}

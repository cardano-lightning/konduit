//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{Address, Output, Value, address::kind::*};
use anyhow::anyhow;
use std::collections::VecDeque;

/// Defines the behaviour of the transaction builder towards change outputs.
///
/// A _strategy_ is nothing more than a (faillible) function of the change and a mutable reference
/// to the outputs. It can be defined explicitly using [`Self::new`]. Alternatively, we provide a
/// handful of pre-defined common strategies:
///
/// - [`Self::as_first_output`]
/// - [`Self::as_last_output`]
pub struct ChangeStrategy(
    #[allow(clippy::type_complexity)]
    Box<dyn FnOnce(Value<u64>, &mut VecDeque<Output>) -> anyhow::Result<()>>,
);

// --------------------------------------------------------------------- Running

impl ChangeStrategy {
    /// Run the encapsulated (faillible) strategy on a mutable set of outputs. This is called
    /// internally by the [`Transaction::build`](crate::Transaction::build) when necessary to
    /// distribute change to the outputs according to the given strategy.
    pub fn apply(self, change: Value<u64>, outputs: &mut VecDeque<Output>) -> anyhow::Result<()> {
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
    /// Define a custom strategy manually. For example, we have:
    ///
    /// ```ignore
    /// pub fn as_last_output(change_address: Address<Any>) -> Self {
    ///     Self::new(move |change, outputs| {
    ///         outputs.push_back(Output::new(change_address, change));
    ///         Ok(())
    ///     })
    /// }
    /// ```
    ///
    /// ```ignore
    /// pub fn as_first_output(change_address: Address<Any>) -> Self {
    ///     Self::new(move |change, outputs| {
    ///         outputs.push_front(Output::new(change_address, change));
    ///         Ok(())
    ///     })
    /// }
    /// ```
    pub fn new(
        strategy: impl FnOnce(Value<u64>, &mut VecDeque<Output>) -> anyhow::Result<()> + 'static,
    ) -> Self {
        Self(Box::new(strategy))
    }

    /// A change strategy that creates a new output with all change sent at the given address, and
    /// append it to the existing list of outputs.
    pub fn as_last_output(change_address: Address<Any>) -> Self {
        Self::new(move |change, outputs| {
            outputs.push_back(Output::new(change_address, change));
            Ok(())
        })
    }

    /// A change strategy that creates a new output with all change sent at the given address, and
    /// prepend it to the existing list of outputs.
    pub fn as_first_output(change_address: Address<Any>) -> Self {
        Self::new(move |change, outputs| {
            outputs.push_front(Output::new(change_address, change));
            Ok(())
        })
    }
}

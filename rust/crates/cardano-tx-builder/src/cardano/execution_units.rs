//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{cbor, pallas};
use std::{cmp::Ordering, fmt};

/// Abstract execution units used to measure execution of [`PlutusScript`](crate::PlutusScript).
#[derive(Debug, Clone, Copy, PartialEq, Eq, cbor::Encode, cbor::Decode)]
#[repr(transparent)]
#[cbor(transparent)]
pub struct ExecutionUnits(#[n(0)] pallas::ExUnits);

impl fmt::Display for ExecutionUnits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExecutionUnits")
            .field("mem", &self.mem())
            .field("cpu", &self.cpu())
            .finish()
    }
}

impl PartialOrd for ExecutionUnits {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.cmp(rhs))
    }
}

impl Ord for ExecutionUnits {
    fn cmp(&self, rhs: &Self) -> Ordering {
        match self.mem().cmp(&rhs.mem()) {
            Ordering::Equal => self.cpu().cmp(&rhs.cpu()),
            ordering @ Ordering::Less | ordering @ Ordering::Greater => ordering,
        }
    }
}

// ------------------------------------------------------------------ Inspecting

impl ExecutionUnits {
    pub fn mem(&self) -> u64 {
        self.0.mem
    }

    pub fn cpu(&self) -> u64 {
        self.0.steps
    }
}

// -------------------------------------------------------------------- Building

impl Default for ExecutionUnits {
    fn default() -> Self {
        Self(pallas::ExUnits { mem: 0, steps: 0 })
    }
}

impl ExecutionUnits {
    pub fn new(mem: u64, cpu: u64) -> Self {
        Self(pallas::ExUnits { mem, steps: cpu })
    }
}

// ----------------------------------------------------------- Converting (from)

impl From<pallas::ExUnits> for ExecutionUnits {
    fn from(ex_units: pallas::ExUnits) -> Self {
        Self(ex_units)
    }
}

// ------------------------------------------------------------- Converting (to)

impl From<ExecutionUnits> for pallas::ExUnits {
    fn from(ex_units: ExecutionUnits) -> Self {
        ex_units.0
    }
}

#[cfg(any(test, feature = "test-utils"))]
pub mod tests {
    use crate::{ExecutionUnits, any, pallas};
    use proptest::prelude::*;

    // -------------------------------------------------------------- Unit tests

    #[test]
    fn display_execution_units() {
        assert_eq!(
            ExecutionUnits::default().to_string(),
            "ExecutionUnits { mem: 0, cpu: 0 }",
        );
    }

    // -------------------------------------------------------------- Properties

    proptest! {
        #[test]
        fn pallas_roundtrip(execution_units in any::execution_units()) {
            let pallas_execution_units = pallas::ExUnits::from(execution_units);
            let execution_units_back = ExecutionUnits::from(pallas_execution_units);
            prop_assert_eq!(execution_units, execution_units_back);
        }
    }

    // -------------------------------------------------------------- Generators

    pub mod generators {
        use super::*;

        prop_compose! {
            pub fn execution_units()(mem in any::<u64>(), cpu in any::<u64>()) -> ExecutionUnits {
                ExecutionUnits::new(mem, cpu)
            }
        }
    }
}

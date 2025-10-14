//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{Hash, PlutusData};
use std::fmt;

/// A datum as found in [`Output`](crate::Output).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Datum {
    Hash(Hash<32>),
    Inline(PlutusData<'static>),
}

impl fmt::Display for Datum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Datum::Hash(hash) => write!(f, "Hash({})", hash),
            Datum::Inline(data) => write!(f, "Inline({})", data),
        }
    }
}

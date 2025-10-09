//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{Hash, PlutusData};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum InlineDatum {
    Hash(Hash<32>),
    Data(PlutusData),
}

impl fmt::Display for InlineDatum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InlineDatum::Hash(hash) => write!(f, "Hash({})", hash),
            InlineDatum::Data(data) => write!(f, "Data({})", data),
        }
    }
}

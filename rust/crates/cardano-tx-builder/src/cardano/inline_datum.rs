//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{Hash, PlutusData};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum InlineDatum {
    Hash(Hash<32>),
    Data(PlutusData),
}

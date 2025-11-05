//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.
//

use pallas_addresses::Slot;

/// A slot boundary to define validity intervals on transactions. The given argument is expressed
/// in (absolute) slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotBound {
    None,
    Inclusive(Slot),
    Exclusive(Slot),
}

impl Default for SlotBound {
    fn default() -> Self {
        Self::None
    }
}

//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::NetworkId;

/// A type to allow bundling a network id with another. Useful to write trait instances that
/// require a network id.
#[derive(Debug)]
pub struct WithNetworkId<'a, T> {
    pub inner: &'a T,
    pub network_id: NetworkId,
}

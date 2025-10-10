//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

/// A trait that is only public within the crate, but doesn't get exposed. This pattern allows to
/// define restrictions on generic types using traits, without allowing implementations of the
/// non-extensible trait.
pub trait NonExtensible {}

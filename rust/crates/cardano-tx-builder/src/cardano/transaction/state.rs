//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

pub trait KnownState: crate::protected::Protected {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Malleable;
impl crate::protected::Protected for Malleable {}
impl KnownState for Malleable {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Sealed;
impl crate::protected::Protected for Sealed {}
impl KnownState for Sealed {}

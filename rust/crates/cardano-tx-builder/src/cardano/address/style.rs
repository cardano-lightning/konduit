//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

pub trait KnownStyle: crate::protected::Protected {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Any;
impl crate::protected::Protected for Any {}
impl KnownStyle for Any {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Byron;
impl crate::protected::Protected for Byron {}
impl KnownStyle for Byron {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Shelley;
impl crate::protected::Protected for Shelley {}
impl KnownStyle for Shelley {}

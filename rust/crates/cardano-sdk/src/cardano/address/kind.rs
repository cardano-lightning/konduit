//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Type-level utilities for guiding the construction & inspection of [`Address`](super::Address).

/// Restricts the inhabitants that a generic parameter can take. This is used in the context of
/// [`Address`](super::Address) to carry certain predicates at the type-level and enable
/// infaillible methods on addresses.
///
/// See also: [`Address::as_shelley`](super::Address::as_shelley) and/or
/// [`Address::as_byron`](super::Address::as_byron).
pub trait IsAddressKind: crate::non_extensible::NonExtensible {}

/// Indicates that the underlying [`Address`](super::Address) is one of any kind. Many methods are
/// available on _any_ addresses.
///
/// Note that there's no ways to construct this struct; its only role is to live at the type-level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Any;
impl crate::non_extensible::NonExtensible for Any {}
impl IsAddressKind for Any {}

/// Indicates that the underlying [`Address`](super::Address) is a Byron (a.k.a Bootstrap) address.
/// Specific methods may be available only to Byron addresses.
///
/// Note that there's no ways to construct this struct; its only role is to live at the type-level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Byron;
impl crate::non_extensible::NonExtensible for Byron {}
impl IsAddressKind for Byron {}

/// Indicates that the underlying [`Address`](super::Address) is a Shelley address. Specific
/// methods may be available only to Shelley addresses.
///
/// Note that there's no ways to construct this struct; its only role is to live at the type-level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Shelley;
impl crate::non_extensible::NonExtensible for Shelley {}
impl IsAddressKind for Shelley {}

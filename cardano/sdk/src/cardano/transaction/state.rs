//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Type-level utilities for guiding the construction of [`Transaction`](super::Transaction).

/// Restricts the inhabitants that a generic parameter can take. This is used in the context of
/// [`Transaction`](super::Transaction) to allow some methods to be always accessible, or
/// restricted to a particular state.
///
/// For example, the [`Transaction::sign`](super::Transaction::sign) method is only available in the state
/// [`ReadyForSigning`]. This ensures that the body is not inadvertently modified once constructed,
/// as it would invalidate all existing signatures.
pub trait IsTransactionBodyState: crate::non_extensible::NonExtensible {
    type ChangeStrategy;
}

/// Indicates that a [`Transaction`](super::Transaction) is in an unknown state; only some methods
/// will be available.
///
/// Note that there's no ways to construct this struct; its only role is to live at the type-level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Unknown;
impl crate::non_extensible::NonExtensible for Unknown {}
impl IsTransactionBodyState for Unknown {
    type ChangeStrategy = ();
}

/// Indicates that a [`Transaction`](super::Transaction) is under construction, and its body may
/// still be modified.
///
/// Note that there's no ways to construct this struct; its only role is to live at the type-level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct InConstruction;
impl crate::non_extensible::NonExtensible for InConstruction {}
impl IsTransactionBodyState for InConstruction {
    type ChangeStrategy = crate::cardano::output::change_strategy::ChangeStrategy;
}

/// Indicates that a [`Transaction`](super::Transaction)'s body is now complete, and the
/// transaction is either awaiting signatures, or fully signed. In particular, methods such as
/// [`Transaction::sign`](super::Transaction::sign) and [`Transaction::id`](super::Transaction::id)
/// becomes available.
///
/// Note that there's no ways to construct this struct; its only role is to live at the type-level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReadyForSigning;
impl crate::non_extensible::NonExtensible for ReadyForSigning {}
impl IsTransactionBodyState for ReadyForSigning {
    type ChangeStrategy = ();
}

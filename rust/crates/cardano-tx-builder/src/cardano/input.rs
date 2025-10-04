//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{Hash, cbor, pallas};
use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, cbor::Encode, cbor::Decode)]
#[repr(transparent)]
#[cbor(transparent)]
pub struct Input<'a>(#[n(0)] Cow<'a, pallas::TransactionInput>);

// --------------------------------------------------------------------- Building

impl<'a> Input<'a> {
    pub fn new(transaction_id: Hash<32>, index: u64) -> Self {
        Self(Cow::Owned(pallas::TransactionInput {
            transaction_id: pallas::Hash::from(transaction_id),
            index,
        }))
    }
}

// ------------------------------------------------------------ Converting (from)

impl<'a> From<&'a pallas::TransactionInput> for Input<'a> {
    fn from(i: &'a pallas::TransactionInput) -> Self {
        Input(Cow::Borrowed(i))
    }
}

impl From<pallas::TransactionInput> for Input<'static> {
    fn from(i: pallas::TransactionInput) -> Self {
        Input(Cow::Owned(i))
    }
}

// -------------------------------------------------------------- Converting (to)

impl<'a> From<Input<'a>> for pallas::TransactionInput {
    fn from(i: Input<'a>) -> Self {
        i.0.into_owned()
    }
}

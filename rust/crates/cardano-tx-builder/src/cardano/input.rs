//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{Hash, cbor, pallas};
use std::{fmt, rc::Rc};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Input(Rc<pallas::TransactionInput>);

impl fmt::Display for Input {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{}", &self.0.transaction_id, self.0.index)
    }
}

// -------------------------------------------------------------------- Building

impl Input {
    pub fn new(transaction_id: Hash<32>, index: u64) -> Self {
        Self(Rc::new(pallas::TransactionInput {
            transaction_id: pallas::Hash::from(transaction_id),
            index,
        }))
    }
}

// ----------------------------------------------------------- Converting (from)

impl From<pallas::TransactionInput> for Input {
    fn from(i: pallas::TransactionInput) -> Self {
        Input(Rc::new(i))
    }
}

// ------------------------------------------------------------- Converting (to)

impl From<Input> for pallas::TransactionInput {
    fn from(i: Input) -> Self {
        Rc::unwrap_or_clone(i.0)
    }
}

// -------------------------------------------------------------------- Encoding

impl<C> cbor::Encode<C> for Input {
    fn encode<W: cbor::encode::write::Write>(
        &self,
        e: &mut cbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), cbor::encode::Error<W::Error>> {
        e.encode_with(self.0.as_ref(), ctx)?;
        Ok(())
    }
}

impl<'d, C> cbor::Decode<'d, C> for Input {
    fn decode(d: &mut cbor::Decoder<'d>, ctx: &mut C) -> Result<Self, cbor::decode::Error> {
        Ok(Self(Rc::new(d.decode_with(ctx)?)))
    }
}

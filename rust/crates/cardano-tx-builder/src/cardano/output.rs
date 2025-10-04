//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{Address, Value, cbor, pallas};
use std::{borrow::Cow, ops::Deref};

#[derive(Debug)]
#[repr(transparent)]
pub struct Output<'a>(Cow<'a, (Address, Value<u64>)>);

// -------------------------------------------------------------------- Building

impl<'a> Output<'a> {
    pub fn new(address: Address, value: Value<u64>) -> Self {
        Self(Cow::Owned((address, value)))
    }

    pub fn address(&self) -> &Address {
        &self.0.0
    }

    pub fn value(&self) -> &Value<u64> {
        &self.0.1
    }
}

// ------------------------------------------------------------ Converting (from)

impl TryFrom<&pallas::TransactionOutput> for Output<'static> {
    type Error = anyhow::Error;

    fn try_from(source: &pallas::TransactionOutput) -> anyhow::Result<Self> {
        let address = match source {
            pallas::TransactionOutput::Legacy(legacy) => {
                Address::try_from(legacy.address.as_slice())
            }
            pallas::TransactionOutput::PostAlonzo(modern) => {
                Address::try_from(modern.address.as_slice())
            }
        }?;

        let value = match source {
            pallas::TransactionOutput::Legacy(legacy) => Value::from(&legacy.amount),
            pallas::TransactionOutput::PostAlonzo(modern) => Value::from(&modern.value),
        };

        Ok(Output(Cow::Owned((address, value))))
    }
}

// -------------------------------------------------------------- Converting (to)

impl<'a> From<&Output<'a>> for pallas::TransactionOutput {
    fn from(this: &Output<'a>) -> Self {
        let (address, value) = this.0.deref();
        pallas::TransactionOutput::PostAlonzo(pallas::PostAlonzoTransactionOutput {
            address: pallas::Bytes::from(<Vec<u8>>::from(address)),
            value: pallas::Value::from(value),
            datum_option: None,
            script_ref: None,
        })
    }
}

impl<'a> From<Output<'a>> for pallas::TransactionOutput {
    fn from(this: Output<'a>) -> Self {
        pallas::TransactionOutput::from(&this)
    }
}

// -------------------------------------------------------------------- Encoding

impl<'a, C> cbor::Encode<C> for Output<'a> {
    fn encode<W: cbor::encode::write::Write>(
        &self,
        e: &mut cbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), cbor::encode::Error<W::Error>> {
        pallas::TransactionOutput::from(self).encode(e, ctx)
    }
}

impl<'d, C> cbor::Decode<'d, C> for Output<'static> {
    fn decode(d: &mut cbor::Decoder<'d>, ctx: &mut C) -> Result<Self, cbor::decode::Error> {
        let output: pallas::TransactionOutput = d.decode_with(ctx)?;
        Self::try_from(&output).map_err(cbor::decode::Error::message)
    }
}

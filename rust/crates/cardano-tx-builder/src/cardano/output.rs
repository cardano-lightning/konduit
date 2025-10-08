//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{Address, Value, address, cbor, pallas};
use std::borrow::Cow;

#[derive(Debug)]
pub struct Output<'a>(Address<'static, address::Any>, Cow<'a, Value<u64>>);

// -------------------------------------------------------------------- Building

impl<'a> Output<'a> {
    pub fn new(address: Address<'static, address::Any>, value: Value<u64>) -> Self {
        Self(address, Cow::Owned(value))
    }

    pub fn address(&'a self) -> Address<'a, address::Any> {
        self.0.borrow()
    }

    pub fn value(&self) -> &Value<u64> {
        &self.1
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

        Ok(Output(address, Cow::Owned(value)))
    }
}

// -------------------------------------------------------------- Converting (to)

impl<'a> From<&Output<'a>> for pallas::TransactionOutput {
    fn from(Output(address, value): &Output<'a>) -> Self {
        pallas::TransactionOutput::PostAlonzo(pallas::PostAlonzoTransactionOutput {
            address: pallas::Bytes::from(<Vec<u8>>::from(address)),
            value: pallas::Value::from(value.as_ref()),
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

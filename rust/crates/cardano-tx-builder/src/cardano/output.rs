//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{Address, PlutusScript, Value, address, cbor, cbor::ToCbor, pallas};
use anyhow::anyhow;
use std::{borrow::Cow, cell::RefCell, ops::Deref, rc::Rc};

pub mod change_strategy;

/// Technically, this is a protocol parameter. It is however usually the same on all networks, and
/// hasn't changed in many years. If it ever change, we can always adjust the library to the
/// maximum of the two values. It is much more convenient than carrying protocol parameters around.
const MIN_VALUE_PER_UTXO_BYTE: u64 = 4310;

/// The CBOR overhead accounting for the in-memory size of inputs and utxo, as per [CIP-0055](https://github.com/cardano-foundation/CIPs/tree/master/CIP-0055#the-new-minimum-lovelace-calculation).
const MIN_LOVELACE_VALUE_CBOR_OVERHEAD: u64 = 160;

#[derive(Debug, Clone)]
pub struct Output<'a>(
    Address<'static, address::Any>,
    DeferredValue<'a>,
    Option<Cow<'a, PlutusScript>>,
);

#[derive(Debug, Clone)]
enum DeferredValue<'a> {
    Minimum(RefCell<Rc<Value<u64>>>),
    Explicit(Cow<'a, Value<u64>>),
}

// ------------------------------------------------------------------ Inspecting

impl<'a> Output<'a> {
    pub fn address(&'a self) -> Address<'a, address::Any> {
        self.0.borrow()
    }

    pub fn min_acceptable_value(&'a self) -> Value<u64> {
        Value::new(MIN_VALUE_PER_UTXO_BYTE * (self.size() + MIN_LOVELACE_VALUE_CBOR_OVERHEAD))
    }

    pub fn value(&'a self) -> Box<dyn AsRef<Value<u64>> + 'a> {
        match &self.1 {
            DeferredValue::Minimum(cell) => Box::new(cell.borrow().clone()),
            DeferredValue::Explicit(value) => Box::new(value),
        }
    }

    pub fn script(&'a self) -> Option<&'a PlutusScript> {
        self.2.as_deref()
    }

    /// Minimum lovelace value required at a UTxO.
    fn refresh_minimum_utxo_value(self) -> Self {
        if let DeferredValue::Minimum(cell) = &self.1 {
            *cell.borrow_mut() = Rc::new(self.min_acceptable_value());
        }

        self
    }

    fn size(&self) -> u64 {
        self.to_cbor().len() as u64
    }
}

// -------------------------------------------------------------------- Building

impl<'a> Output<'a> {
    /// Construct a new output from an address and a value.
    pub fn new(address: Address<'static, address::Any>, value: Value<u64>) -> Self {
        Self(address, DeferredValue::Explicit(Cow::Owned(value)), None)
    }

    /// Like [`Self::new`], but assumes a minimum Ada value as output.
    pub fn to(address: Address<'static, address::Any>) -> Self {
        Self(
            address,
            // We use an initial value that's at least 2 ^ 16, so that it is CBOR-encoded over 5
            // bytes and results in a correct minimum value based on the serialised size.
            DeferredValue::Minimum(RefCell::new(Rc::new(Value::new(2 ^ 16)))),
            None,
        )
        .refresh_minimum_utxo_value()
    }

    /// Attach a reference script to the output
    pub fn with_plutus_script(mut self, plutus_script: PlutusScript) -> Self {
        self.2 = Some(Cow::Owned(plutus_script));
        self.refresh_minimum_utxo_value()
    }
}

// ------------------------------------------------------------ Converting (from)

impl TryFrom<pallas::TransactionOutput> for Output<'static> {
    type Error = anyhow::Error;

    fn try_from(source: pallas::TransactionOutput) -> anyhow::Result<Self> {
        let (address, value, plutus_script_opt) = match source {
            pallas::TransactionOutput::Legacy(legacy) => {
                let address = Address::try_from(legacy.address.as_slice())?;
                let value = Value::from(&legacy.amount);
                let plutus_script_opt = None;

                Ok::<_, anyhow::Error>((address, value, plutus_script_opt))
            }

            pallas::TransactionOutput::PostAlonzo(modern) => {
                let address = Address::try_from(modern.address.as_slice())?;
                let value = Value::from(&modern.value);
                let plutus_script_opt = match modern.script_ref.map(|wrap| wrap.unwrap()) {
                    None => Ok(None),
                    Some(pallas::ScriptRef::NativeScript(_)) => {
                        Err(anyhow!("found unsupported native script at output"))
                    }
                    Some(pallas::ScriptRef::PlutusV1Script(script)) => {
                        Ok(Some(PlutusScript::from(script)))
                    }
                    Some(pallas::ScriptRef::PlutusV2Script(script)) => {
                        Ok(Some(PlutusScript::from(script)))
                    }
                    Some(pallas::ScriptRef::PlutusV3Script(script)) => {
                        Ok(Some(PlutusScript::from(script)))
                    }
                }?;

                Ok::<_, anyhow::Error>((address, value, plutus_script_opt))
            }
        }?;

        let mut output = Output::new(address, value);

        if let Some(plutus_script) = plutus_script_opt {
            output = output.with_plutus_script(plutus_script);
        }

        Ok(output)
    }
}

// -------------------------------------------------------------- Converting (to)

impl<'a> From<&Output<'a>> for pallas::TransactionOutput {
    fn from(output: &Output<'a>) -> Self {
        pallas::TransactionOutput::PostAlonzo(pallas::PostAlonzoTransactionOutput {
            address: pallas::Bytes::from(<Vec<u8>>::from(&output.address())),
            value: pallas::Value::from(output.value().deref().as_ref()),
            datum_option: None,
            script_ref: output
                .script()
                .map(|script| pallas::CborWrap(pallas::ScriptRef::from(script.clone()))),
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
        Self::try_from(output).map_err(cbor::decode::Error::message)
    }
}

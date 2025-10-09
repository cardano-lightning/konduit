//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{Address, Hash, PlutusScript, Value, address, cbor, cbor::ToCbor, pallas};
use anyhow::anyhow;
use std::rc::Rc;

pub mod change_strategy;

/// Technically, this is a protocol parameter. It is however usually the same on all networks, and
/// hasn't changed in many years. If it ever change, we can always adjust the library to the
/// maximum of the two values. It is much more convenient than carrying protocol parameters around.
const MIN_VALUE_PER_UTXO_BYTE: u64 = 4310;

/// The CBOR overhead accounting for the in-memory size of inputs and utxo, as per [CIP-0055](https://github.com/cardano-foundation/CIPs/tree/master/CIP-0055#the-new-minimum-lovelace-calculation).
const MIN_LOVELACE_VALUE_CBOR_OVERHEAD: u64 = 160;

#[derive(Debug, Clone)]
pub struct Output(
    Address<address::Any>,
    DeferredValue,
    Option<Rc<PlutusScript>>,
);

#[derive(Debug, Clone)]
enum DeferredValue {
    Minimum(Rc<Value<u64>>),
    Explicit(Rc<Value<u64>>),
}

// ------------------------------------------------------------------ Inspecting

impl Output {
    pub fn address(&self) -> &Address<address::Any> {
        &self.0
    }

    pub fn value(&self) -> &Value<u64> {
        match &self.1 {
            DeferredValue::Minimum(value) | DeferredValue::Explicit(value) => value.as_ref(),
        }
    }

    pub fn script(&self) -> Option<&PlutusScript> {
        self.2.as_deref()
    }

    /// The minimum quantity of lovelace acceptable to carry this output. Address' delegation,
    /// assets, scripts and datums may increase this value.
    pub fn min_acceptable_value(&self) -> u64 {
        // In case where values are too small, we still count for 5 bytes to avoid having to search
        // for a fixed point. This is because CBOR uses variable-length encoding for integers,
        // according to the following rules:
        //
        // | value                  | encoding size       |
        // | ---------------------- | ------------------- |
        // | 0      <= n < 24       | 1 byte              |
        // | 24     <= n < 2 ^ 8    | 2 bytes             |
        // | 2 ^ 8  <= n < 2 ^ 16   | 3 bytes             |
        // | 2 ^ 16 <= n < 2 ^ 32   | 5 bytes             |
        // | 2 ^ 32 <= n < 2 ^ 64   | 9 bytes             |
        //
        // Values are at least MIN_VALUE_PER_UTXO_BYTE * MIN_LOVELACE_VALUE_CBOR_OVERHEAD = 689600,
        // so that means the encoding will never be smaller than 5 bytes; if it is, we must inflate
        // the size artificially to compensate.
        let current_value = self.value().lovelace();

        let extra_size = match current_value {
            _ if current_value < 24 => 4,
            _ if current_value < 256 => 3,
            _ if current_value < 65535 => 2,
            _ => 0,
        };

        MIN_VALUE_PER_UTXO_BYTE * (self.size() + MIN_LOVELACE_VALUE_CBOR_OVERHEAD + extra_size)
    }

    /// Adjust the lovelace quantities of deferred values using the size of the serialised output.
    /// This does nothing on explicitly given values -- EVEN WHEN they are below the minimum
    /// threshold.
    fn set_minimum_utxo_value(&mut self) {
        // Only compute the minimum when it's actually required. Note that we cannot do this within
        // the next block because of the immutable borrow that occurs already.
        let min_acceptable_value = match &self.1 {
            DeferredValue::Explicit(_) => 0,
            DeferredValue::Minimum(_) => self.min_acceptable_value(),
        };

        if let DeferredValue::Minimum(rc) = &mut self.1 {
            let value: &mut Value<u64> = Rc::make_mut(rc);
            value.with_lovelace(min_acceptable_value);
        }
    }

    fn size(&self) -> u64 {
        self.to_cbor().len() as u64
    }
}

// -------------------------------------------------------------------- Building

impl Output {
    /// Construct a new output from an address and a value.
    pub fn new(address: Address<address::Any>, value: Value<u64>) -> Self {
        Self(address, DeferredValue::Explicit(Rc::new(value)), None)
    }

    /// Like [`Self::new`], but assumes a minimum Ada value as output.
    pub fn to(address: Address<address::Any>) -> Self {
        let mut value = Self(
            address,
            DeferredValue::Minimum(Rc::new(Value::default())),
            None,
        );
        value.set_minimum_utxo_value();
        value
    }

    /// Attach assets to the output, while preserving the Ada value. If minimum was assumed (e.g.
    /// using [`to`]), then the minimum will automatically grow to compensate for the new assets.
    pub fn with_assets(
        mut self,
        assets: impl IntoIterator<Item = (Hash<28>, impl IntoIterator<Item = (Vec<u8>, u64)>)>,
    ) -> Self {
        self.1 = DeferredValue::Minimum(Rc::new(Value::default().with_assets(assets)));
        self.set_minimum_utxo_value();
        self
    }

    /// Attach a reference script to the output
    pub fn with_plutus_script(mut self, plutus_script: PlutusScript) -> Self {
        self.2 = Some(Rc::new(plutus_script));
        self.set_minimum_utxo_value();
        self
    }
}

// ------------------------------------------------------------ Converting (from)

impl TryFrom<pallas::TransactionOutput> for Output {
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

impl From<&Output> for pallas::TransactionOutput {
    fn from(output: &Output) -> Self {
        pallas::TransactionOutput::PostAlonzo(pallas::PostAlonzoTransactionOutput {
            address: pallas::Bytes::from(<Vec<u8>>::from(output.address())),
            value: pallas::Value::from(output.value()),
            datum_option: None,
            script_ref: output
                .script()
                .map(|script| pallas::CborWrap(pallas::ScriptRef::from(script.clone()))),
        })
    }
}

impl From<Output> for pallas::TransactionOutput {
    fn from(this: Output) -> Self {
        pallas::TransactionOutput::from(&this)
    }
}

// -------------------------------------------------------------------- Encoding

impl<C> cbor::Encode<C> for Output {
    fn encode<W: cbor::encode::write::Write>(
        &self,
        e: &mut cbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), cbor::encode::Error<W::Error>> {
        pallas::TransactionOutput::from(self).encode(e, ctx)
    }
}

impl<'d, C> cbor::Decode<'d, C> for Output {
    fn decode(d: &mut cbor::Decoder<'d>, ctx: &mut C) -> Result<Self, cbor::decode::Error> {
        let output: pallas::TransactionOutput = d.decode_with(ctx)?;
        Self::try_from(output).map_err(cbor::decode::Error::message)
    }
}

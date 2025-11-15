//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{
    Address, Datum, Hash, PlutusData, PlutusScript, Value, address::kind::*, cbor, cbor::ToCbor,
    pallas, pretty,
};
use anyhow::anyhow;
use std::{fmt, sync::Arc};

#[cfg(feature = "wasm")]
use crate::cardano::value::OutputAssets;
#[cfg(feature = "wasm")]
use std::str::FromStr;
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

pub mod change_strategy;

/// Technically, this is a protocol parameter. It is however usually the same on all networks, and
/// hasn't changed in many years. If it ever change, we can always adjust the library to the
/// maximum of the two values. It is much more convenient than carrying protocol parameters around.
const MIN_VALUE_PER_UTXO_BYTE: u64 = 4310;

/// The CBOR overhead accounting for the in-memory size of inputs and utxo, as per [CIP-0055](https://github.com/cardano-foundation/CIPs/tree/master/CIP-0055#the-new-minimum-lovelace-calculation).
const MIN_LOVELACE_VALUE_CBOR_OVERHEAD: u64 = 160;

/// A transaction output, which comprises of at least an [`Address`] and a [`Value<u64>`].
///
/// The value can be either explicit set using [`Self::new`] or defined to the minimum acceptable
/// by the protocol using [`Self::to`].
///
/// Optionally, one can attach an [`Datum`] and/or a [`PlutusScript`] via
/// [`Self::with_datum`]/[`Self::with_datum_hash`] and [`Self::with_plutus_script`] respectively.
///
/// <div class="warning">Native scripts as reference scripts aren't yet supported. Only Plutus
/// scripts are.</div>
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "wasm",
    wasm_bindgen,
    doc = "A transaction output, which comprises of at least an Address and a Value."
)]
pub struct Output {
    address: Address<Any>,
    value: DeferredValue,
    datum: Option<Arc<Datum>>,
    script: Option<Arc<PlutusScript>>,
}

impl fmt::Display for Output {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:#?}",
            pretty::Fmt(|f: &mut fmt::Formatter<'_>| {
                let mut debug_struct = f.debug_struct("Output");

                debug_struct.field("address", &pretty::ViaDisplayNoAlloc(self.address()));

                debug_struct.field("value", &pretty::ViaDisplayNoAlloc(self.value()));

                if let Some(datum) = self.datum() {
                    debug_struct.field("datum", &pretty::ViaDisplayNoAlloc(datum));
                }

                if let Some(script) = self.script() {
                    debug_struct.field("script", &pretty::ViaDisplayNoAlloc(script));
                }

                debug_struct.finish()
            })
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DeferredValue {
    Minimum(Arc<Value<u64>>),
    Explicit(Arc<Value<u64>>),
}

// -------------------------------------------------------------------- Building

impl Output {
    /// Construct a new output from an [`Address`] and a [`Value<u64>`]. See also [`Self::to`] for
    /// constructing a value without an explicit value.
    pub fn new(address: Address<Any>, value: Value<u64>) -> Self {
        Self {
            address,
            value: DeferredValue::Explicit(Arc::new(value)),
            datum: None,
            script: None,
        }
    }

    /// Like [`Self::new`], but assumes a minimum lovelace value as output. The value automatically
    /// adjusts based on the other Output's elements (assets, scripts, etc..).
    pub fn to(address: Address<Any>) -> Self {
        let mut output = Self {
            address,
            value: DeferredValue::Minimum(Arc::new(Value::default())),
            datum: None,
            script: None,
        };

        output.set_minimum_utxo_value();

        output
    }

    /// Attach assets to the output, while preserving the lovelace value.
    pub fn with_assets<AssetName>(
        mut self,
        assets: impl IntoIterator<Item = (Hash<28>, impl IntoIterator<Item = (AssetName, u64)>)>,
    ) -> Self
    where
        AssetName: AsRef<[u8]>,
    {
        self.value = DeferredValue::Minimum(Arc::new(Value::default().with_assets(assets)));
        self.set_minimum_utxo_value();
        self
    }

    /// Attach a reference script to the output.
    pub fn with_plutus_script(mut self, plutus_script: PlutusScript) -> Self {
        self.script = Some(Arc::new(plutus_script));
        self.set_minimum_utxo_value();
        self
    }

    /// Attach a datum reference as [`struct@Hash<32>`] to the output.
    pub fn with_datum_hash(mut self, hash: Hash<32>) -> Self {
        self.datum = Some(Arc::new(Datum::Hash(hash)));
        self.set_minimum_utxo_value();
        self
    }

    /// Attach a plain [`PlutusData`] datum to the output.
    pub fn with_datum(mut self, data: PlutusData<'static>) -> Self {
        self.datum = Some(Arc::new(Datum::Inline(data)));
        self.set_minimum_utxo_value();
        self
    }

    /// Adjust the lovelace quantities of deferred values using the size of the serialised output.
    /// This does nothing on explicitly given values -- EVEN WHEN they are below the minimum
    /// threshold.
    fn set_minimum_utxo_value(&mut self) {
        // Only compute the minimum when it's actually required. Note that we cannot do this within
        // the next block because of the immutable borrow that occurs already.
        let min_acceptable_value = match &self.value {
            DeferredValue::Explicit(_) => 0,
            DeferredValue::Minimum(_) => self.min_acceptable_value(),
        };

        if let DeferredValue::Minimum(rc) = &mut self.value {
            let value: &mut Value<u64> = Arc::make_mut(rc);
            value.with_lovelace(min_acceptable_value);
        }
    }
}

// ------------------------------------------------------------------ Inspecting

impl Output {
    pub fn address(&self) -> &Address<Any> {
        &self.address
    }

    pub fn value(&self) -> &Value<u64> {
        match &self.value {
            DeferredValue::Minimum(value) | DeferredValue::Explicit(value) => value.as_ref(),
        }
    }

    pub fn script(&self) -> Option<&PlutusScript> {
        self.script.as_deref()
    }

    pub fn datum(&self) -> Option<&Datum> {
        self.datum.as_deref()
    }

    /// The minimum quantity of lovelace acceptable to carry this output. Address' delegation,
    /// assets, scripts and datums may increase this value.
    ///
    /// # examples
    ///
    /// ```rust
    /// # use cardano_tx_builder::*;
    /// // Simple, undelegated address. About as low as we can go.
    /// assert_eq!(
    ///   output!("addr1v83gkkw3nqzakg5xynlurqcfqhgd65vkfvf5xv8tx25ufds2yvy2h")
    ///     .min_acceptable_value(),
    ///   857690,
    /// );
    ///
    /// // Address with delegation.
    /// assert_eq!(
    ///   output!("addr1qytp6yfl9wwamcqu3j5kqhjz8hlgkt62nd82d837g9dlsmn85wjc8sjtq2wqxfmahmpn6h85y0ug7mzclf2jl4zyt3vq587s69")
    ///     .min_acceptable_value(),
    ///   978370,
    /// );
    ///
    /// // Undelegated address with some native assets.
    /// assert_eq!(
    ///   output!("addr1v83gkkw3nqzakg5xynlurqcfqhgd65vkfvf5xv8tx25ufds2yvy2h")
    ///     .with_assets([
    ///         (
    ///             hash!("279c909f348e533da5808898f87f9a14bb2c3dfbbacccd631d927a3f"),
    ///             [(b"SNEK".to_vec(), 1_000_000_000)]
    ///         )
    ///     ])
    ///     .min_acceptable_value(),
    ///   1043020,
    /// );
    ///
    /// // Undelegated address with some inline datum.
    /// assert_eq!(
    ///   output!("addr1v83gkkw3nqzakg5xynlurqcfqhgd65vkfvf5xv8tx25ufds2yvy2h")
    ///     .with_datum(PlutusData::list([
    ///         PlutusData::integer(14),
    ///         PlutusData::integer(42),
    ///         PlutusData::bytes(b"foobar"),
    ///     ]))
    ///     .min_acceptable_value(),
    ///   935270,
    /// );
    ///
    /// // Undelegated address with some datum hash.
    /// assert_eq!(
    ///   output!("addr1v83gkkw3nqzakg5xynlurqcfqhgd65vkfvf5xv8tx25ufds2yvy2h")
    ///     .with_datum_hash(hash!("279c909f348e533da5808898f87f9a14bb2c3dfbbacccd631d927a3f00000000"))
    ///     .min_acceptable_value(),
    ///   1017160,
    /// );
    /// ```
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

    fn size(&self) -> u64 {
        self.to_cbor().len() as u64
    }
}

// ------------------------------------------------------------ Converting (from)

impl TryFrom<pallas::TransactionOutput> for Output {
    type Error = anyhow::Error;

    fn try_from(source: pallas::TransactionOutput) -> anyhow::Result<Self> {
        let (address, value, datum_opt, plutus_script_opt) = match source {
            pallas::TransactionOutput::Legacy(legacy) => {
                let address = Address::try_from(legacy.address.as_slice())?;
                let value = Value::from(&legacy.amount);
                let datum_opt = legacy.datum_hash.map(|hash| Datum::Hash(Hash::from(hash)));
                let plutus_script_opt = None;

                Ok::<_, anyhow::Error>((address, value, datum_opt, plutus_script_opt))
            }

            pallas::TransactionOutput::PostAlonzo(modern) => {
                let address = Address::try_from(modern.address.as_slice())?;
                let value = Value::from(&modern.value);
                let datum_opt = match modern.datum_option {
                    None => None,
                    Some(pallas::DatumOption::Hash(hash)) => Some(Datum::Hash(Hash::from(hash))),
                    Some(pallas::DatumOption::Data(data)) => {
                        Some(Datum::Inline(PlutusData::from(data.0)))
                    }
                };
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

                Ok::<_, anyhow::Error>((address, value, datum_opt, plutus_script_opt))
            }
        }?;

        let mut output = Output::new(address, value);

        output = match datum_opt {
            Some(Datum::Inline(data)) => output.with_datum(data),
            Some(Datum::Hash(hash)) => output.with_datum_hash(hash),
            None => output,
        };

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
            datum_option: output.datum().map(|datum| match datum {
                Datum::Hash(hash) => pallas::DatumOption::Hash(pallas::DatumHash::from(hash)),
                Datum::Inline(data) => pallas::DatumOption::Data(pallas::CborWrap(
                    pallas::PlutusData::from(data.clone()),
                )),
            }),
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

// ------------------------------------------------------------------------ WASM

#[cfg(feature = "wasm")]
#[cfg_attr(feature = "wasm", wasm_bindgen, doc(hidden))]
impl Output {
    #[cfg(feature = "wasm")]
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "new"))]
    pub fn _wasm_new(address: &str, amount: u64) -> Self {
        Self::new(
            Address::from_str(address).expect("invalid address"),
            Value::new(amount),
        )
    }

    #[cfg(feature = "wasm")]
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "to"))]
    pub fn _wasm_to(address: &str) -> Self {
        Self::to(Address::from_str(address).expect("invalid address"))
    }

    #[cfg(feature = "wasm")]
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "withAssets"))]
    pub fn _wasm_with_assets(&mut self, assets: &OutputAssets) {
        self.value = DeferredValue::Minimum(Arc::new(Value::default().with_assets(assets.clone())));
        self.set_minimum_utxo_value();
    }

    #[cfg(feature = "wasm")]
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = "toString"))]
    pub fn _wasm_to_string(&self) -> String {
        self.to_string()
    }
}

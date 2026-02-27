use crate::{
    helpers::try_into_array,
    types::asset_object::{AssetObject, from_asset_objects},
};
use cardano_sdk::{Address, Hash, Input, Output, address::kind, cbor};

/// A synthetic representation of a transaction used by the Connector.
#[derive(Debug, Clone)]
pub struct TransactionSummary {
    pub id: Hash<32>,
    pub index: u64,
    pub depth: u64,
    pub inputs: Vec<(Input, Output, Option<Hash<28>>)>,
    pub outputs: Vec<(Output, Option<Hash<28>>)>,
    pub timestamp_secs: u64,
}

impl<'de> serde::Deserialize<'de> for TransactionSummary {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        pub struct JsonTransaction<'a> {
            id: &'a str,
            index: u64,
            depth: u64,
            inputs: Vec<JsonInput<'a>>,
            outputs: Vec<JsonOutput<'a>>,
            timestamp: u64,
        }

        #[derive(serde::Deserialize)]
        pub struct JsonInput<'a> {
            transaction_id: &'a str,
            output_index: u64,
            #[serde(flatten)]
            output: JsonOutput<'a>,
        }

        #[derive(serde::Deserialize)]
        pub struct JsonOutput<'a> {
            address: &'a str,
            value: Vec<AssetObject>,
            datum_hash: Option<&'a str>,
            datum_inline: Option<&'a str>,
            reference_script_hash: Option<&'a str>,
        }

        impl<'a> TryFrom<JsonInput<'a>> for (Input, Output, Option<Hash<28>>) {
            type Error = anyhow::Error;
            fn try_from(json: JsonInput<'a>) -> anyhow::Result<Self> {
                let input = Input::new(
                    try_into_array(hex::decode(json.transaction_id)?)?.into(),
                    json.output_index,
                );

                let (partial_output, reference_script_hash) = json.output.try_into()?;

                Ok((input, partial_output, reference_script_hash))
            }
        }

        impl<'a> TryFrom<JsonOutput<'a>> for (Output, Option<Hash<28>>) {
            type Error = anyhow::Error;
            fn try_from(json: JsonOutput<'a>) -> anyhow::Result<Self> {
                let address = <Address<kind::Shelley>>::try_from(json.address)?;

                let mut output = Output::new(address.into(), from_asset_objects(&json.value[..])?);

                if let Some(hash_str) = json.datum_hash {
                    output = output.with_datum_hash(Hash::try_from(hash_str)?);
                }

                if let Some(data_str) = json.datum_inline {
                    let plutus_data = cbor::decode(&hex::decode(data_str)?)?;
                    output = output.with_datum(plutus_data);
                }

                let reference_script_hash =
                    json.reference_script_hash.map(Hash::try_from).transpose()?;

                Ok((output, reference_script_hash))
            }
        }

        let json = JsonTransaction::deserialize(deserializer)?;

        Ok(TransactionSummary {
            id: Hash::<32>::try_from(json.id).map_err(serde::de::Error::custom)?,
            index: json.index,
            depth: json.depth,
            inputs: json
                .inputs
                .into_iter()
                .map(JsonInput::try_into)
                .collect::<Result<Vec<_>, _>>()
                .map_err(serde::de::Error::custom)?,
            outputs: json
                .outputs
                .into_iter()
                .map(JsonOutput::try_into)
                .collect::<Result<Vec<_>, _>>()
                .map_err(serde::de::Error::custom)?,
            timestamp_secs: json.timestamp,
        })
    }
}

#[cfg(feature = "wasm")]
pub mod wasm {
    use crate::types::{
        self,
        wasm::{InputSummary, OutputSummary},
    };
    use cardano_sdk::{wasm::Hash32, wasm_proxy};
    use wasm_bindgen::{JsValue, prelude::*};

    wasm_proxy! {
        #[derive(Debug, Clone)]
        #[doc = "A synthetic representation of a transaction used by the Connector."]
        TransactionSummary
    }

    #[wasm_bindgen]
    impl TransactionSummary {
        #[wasm_bindgen(getter, js_name = "id")]
        pub fn _wasm_id(&self) -> Hash32 {
            Hash32::from(self.id)
        }

        #[wasm_bindgen(getter, js_name = "index")]
        pub fn _wasm_index(&self) -> u64 {
            self.index
        }

        #[wasm_bindgen(getter, js_name = "depth")]
        pub fn depth(&self) -> u64 {
            self.depth
        }

        #[wasm_bindgen(getter, js_name = "outputs")]
        pub fn _wasm_outputs(&self) -> Vec<OutputSummary> {
            self.outputs
                .iter()
                .map(|(partial_output, reference_script_hash)| {
                    types::OutputSummary::new(partial_output.clone(), *reference_script_hash).into()
                })
                .collect()
        }

        #[wasm_bindgen(getter, js_name = "inputs")]
        pub fn _wasm_inputs(&self) -> Vec<InputSummary> {
            self.inputs
                .iter()
                .map(|(input, partial_output, reference_script_hash)| {
                    types::InputSummary {
                        input: input.clone(),
                        output: types::OutputSummary::new(
                            partial_output.clone(),
                            *reference_script_hash,
                        ),
                    }
                    .into()
                })
                .collect()
        }

        #[wasm_bindgen(getter, js_name = "timestamp")]
        pub fn _wasm_timestamp(&self) -> js_sys::Date {
            js_sys::Date::new(&JsValue::from_f64((self.timestamp_secs * 1000) as f64))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TransactionSummary;

    #[test]
    fn deserialize_json_golden_1() {
        let json: &str = r#"{
          "id": "9cde01ec0abab4a0ea623b1151b97fd54dfb6b12dca52fcd3d23a86c4cd5c08a",
          "index": 0,
          "depth": 4435693,
          "timestamp": 1771459333,
          "invalid_before": "115776058",
          "inputs": [
            {
              "transaction_id": "5bbe52cff47e3a3a903f224060447fd7e9bc0babca701719a91c20482101529a",
              "output_index": 0,
              "address": "addr_test1vrpynvza5vswczszkjhe5cvqz2awmzukf84xa5wway8durqpmfm2m",
              "value": [
                {
                  "unit": "lovelace",
                  "quantity": "49797740"
                }
              ],
              "reference_script_hash": "68f3d3eaffeb93ccac7ffc52a385c82d18073d452e5502bb24234a09"
            },
            {
              "transaction_id": "a815ffd09fb6d98b574a8caf58b75949002eab2572b705763db3093cd60e8e66",
              "output_index": 1,
              "address": "addr_test1vqe4hywyz43w8tw4ddwgjpzvn3j970dtth30ua5qa3wqd8cz2rwa8",
              "value": [
                {
                  "unit": "lovelace",
                  "quantity": "851296162"
                }
              ]
            },
            {
              "transaction_id": "a815ffd09fb6d98b574a8caf58b75949002eab2572b705763db3093cd60e8e66",
              "output_index": 1,
              "address": "addr_test1vqe4hywyz43w8tw4ddwgjpzvn3j970dtth30ua5qa3wqd8cz2rwa8",
              "value": [
                {
                  "unit": "lovelace",
                  "quantity": "851296162"
                }
              ]
            }
          ],
          "outputs": [
            {
              "address": "addr_test1wp5085l2ll4e8n9v0l799gu9eqk3speag5h92q4mys355zguytrmh",
              "value": [
                {
                  "unit": "lovelace",
                  "quantity": "16000000"
                }
              ],
              "datum_hash": "f48dff50e95b7f483380c43b2fc76a8f4cdc38d5d9ef6d5932209ae32d3873ff",
              "datum_inline": "9f581c68f3d3eaffeb93ccac7ffc52a385c82d18073d452e5502bb24234a09d8799f5820ce9cc9d1218226891087ddf260d6ba380548c896381d42d301606573bf22ffc3582073a01b4e86177b93c398d9ce094ecbabc6283e534c41c0922e8c455391d0d52258204bab61da692a9af1b8d24a46d3c91ce6f8f31754ab824c6bb3f556af2d5846f91a05265c00ffd8799f0080ffff"
            },
            {
              "address": "addr_test1vqe4hywyz43w8tw4ddwgjpzvn3j970dtth30ua5qa3wqd8cz2rwa8",
              "value": [
                {
                  "unit": "lovelace",
                  "quantity": "834946729"
                }
              ]
            },
            {
              "address": "addr_test1vqe4hywyz43w8tw4ddwgjpzvn3j970dtth30ua5qa3wqd8cz2rwa8",
              "value": [
                {
                  "unit": "lovelace",
                  "quantity": "850772012"
                }
              ]
            }
          ]
        }"#;

        assert!(dbg!(serde_json::from_str::<TransactionSummary>(json)).is_ok())
    }
}

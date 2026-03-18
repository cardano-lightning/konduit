use crate::core;
use anyhow::anyhow;
use cardano_sdk::SigningKey;
use cardano_sdk::transaction::state::ReadyForSigning;
use cardano_sdk::{Input, NetworkId, Output, Transaction, VerificationKey, cbor::ToCbor};
use konduit_data::Duration;
use konduit_tx::Bounds;
use konduit_tx::NetworkParameters;
use konduit_tx::consumer;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

pub use crate::wasm_core::Result;
pub use crate::wasm_proxy;

wasm_proxy! {
    #[derive(Debug)]
    #[doc = "A reference to fully built transaction"]
    TransactionReadyForSigning => Transaction<ReadyForSigning>
}

#[wasm_bindgen]
impl TransactionReadyForSigning {
    #[wasm_bindgen(js_name = "toCbor")]
    pub fn _wasm_to_cbor(&self) -> Vec<u8> {
        ToCbor::to_cbor(&self.0)
    }

    #[wasm_bindgen(js_name = "getId")]
    pub fn _wasm_id(&self) -> Vec<u8> {
        self.id().as_ref().into()
    }

    #[wasm_bindgen(js_name = "sign")]
    pub fn _wasm_sign(&mut self, secret_key: &[u8]) -> Result<()> {
        let signing_key: SigningKey = <[u8; 32]>::try_from(secret_key)
            .map_err(|_| anyhow!("invalid signing key length, expected 32 bytes"))?
            .into();
        self.0.sign(&signing_key);
        Ok(())
    }
}

wasm_proxy! {
    #[doc = "Network protocol parameters used to expose predefined network configs."]
    Network => cardano_sdk::Network
}

#[wasm_bindgen]
impl Network {
    #[wasm_bindgen(js_name = "mainnet")]
    pub fn mainnet() -> Self {
        Self(cardano_sdk::Network::Mainnet)
    }

    #[wasm_bindgen(js_name = "preprod")]
    pub fn preprod() -> Self {
        Self(cardano_sdk::Network::Preprod)
    }

    #[wasm_bindgen(js_name = "preview")]
    pub fn preview() -> Self {
        Self(cardano_sdk::Network::Preview)
    }
}

// CIP-30:
// ```
// If we have CBOR specified by the following CDDL referencing the Shelley-MA CDDL:
//
// transaction_unspent_output = [
//   input: transaction_input,
//   output: transaction_output,
// ]
// ```
struct TransactionUnspentOutput {
    input: cardano_sdk::Input,
    output: cardano_sdk::Output,
}

impl<C> minicbor::Encode<C> for TransactionUnspentOutput {
    fn encode<W: minicbor::encode::write::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> std::result::Result<(), minicbor::encode::Error<W::Error>> {
        // => encode as a 2-element CBOR array: [input, output]
        e.array(2)?;
        e.encode_with(&self.input, ctx)?;
        e.encode_with(&self.output, ctx)?;
        Ok(())
    }
}

impl<'d, C> minicbor::Decode<'d, C> for TransactionUnspentOutput {
    fn decode(
        d: &mut minicbor::Decoder<'d>,
        ctx: &mut C,
    ) -> std::result::Result<Self, minicbor::decode::Error> {
        let len = d.array()?;
        if len != Some(2) {
            return Err(minicbor::decode::Error::message(format!(
                "expected array of length 2 for TransactionUnspentOutput, got {:?}",
                len
            )));
        }

        let input: cardano_sdk::Input = d.decode_with(ctx)?;
        let output: cardano_sdk::Output = d.decode_with(ctx)?;

        Ok(TransactionUnspentOutput { input, output })
    }
}

#[wasm_bindgen]
pub fn open_tx(
    channel_tag: &[u8],
    consumer_vk: &[u8],
    adaptor_vk: &[u8],
    // CIP-30 UnspentTransactionOutput
    funding_utxos: Vec<js_sys::Uint8Array>,
    network: &Network,
    close_period_sec: u64,
    amount: u64,
) -> Result<TransactionReadyForSigning> {
    let consumer_vk = VerificationKey::try_from(Vec::from(consumer_vk))
        .map_err(|_| anyhow!("invalid verification key length"))?;

    let adaptor_vk = VerificationKey::try_from(Vec::from(adaptor_vk))
        .map_err(|_| anyhow!("invalid verification key length"))?;

    let tag = core::Tag::from(channel_tag);

    let close_period = Duration(std::time::Duration::from_secs(close_period_sec));

    let Network(network) = network;

    let network_id: NetworkId = network.clone().into();

    let protocol_parameters: cardano_sdk::ProtocolParameters =
        cardano_sdk::ProtocolParameters::from(*network);

    let network_parameters = NetworkParameters {
        network_id,
        protocol_parameters,
    };

    let opens = vec![core::consumer::OpenIntent {
        tag: tag.clone(),
        sub_vkey: adaptor_vk,
        close_period,
        amount,
    }];

    // Decode CIP-30 UTxOs (each is CBOR-encoded TransactionUnspentOutput)
    let mut utxos: BTreeMap<Input, Output> = BTreeMap::new();

    for js_u8arr in funding_utxos {
        let mut bytes = vec![0u8; js_u8arr.length() as usize];
        js_u8arr.copy_to(&mut bytes);

        let tuso: TransactionUnspentOutput =
            minicbor::decode(&bytes).map_err(|e| anyhow!("failed to decode funding_utxo: {e}"))?;

        utxos.insert(tuso.input, tuso.output);
    }

    let tx = consumer::tx(
        &network_parameters,
        &consumer_vk,
        opens,
        Default::default(), // intents,
        &utxos,
        Bounds::twenty_mins(),
    )
    .map_err(|e| anyhow!("failed to build transaction: {e}"))?;
    Ok(tx.into())
}

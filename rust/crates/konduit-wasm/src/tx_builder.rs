use crate::core;
use anyhow::anyhow;
use cardano_sdk::SigningKey;
use cardano_sdk::transaction::state::ReadyForSigning;
use cardano_sdk::{Input, NetworkId, Output, Transaction, VerificationKey, cbor::ToCbor};
use konduit_data::Duration;
use konduit_tx::Bounds;
use konduit_tx::NetworkParameters;
use konduit_tx::consumer;
use konduit_tx::consumer::{Intent as CoreIntent, OpenIntent as CoreOpenIntent};
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
    #[derive(Debug, Clone)]
    #[doc = "Consumer intent for an existing channel (add funds or close)."]
    Intent => CoreIntent
}

#[wasm_bindgen]
impl Intent {
    #[wasm_bindgen(js_name = "add")]
    pub fn add(amount: u64) -> Intent {
        CoreIntent::Add(amount).into()
    }

    #[wasm_bindgen(js_name = "close")]
    pub fn close() -> Intent {
        CoreIntent::Close.into()
    }
}

#[wasm_bindgen]
#[derive(Debug, Clone)]
#[doc = "Intent associated with a channel tag (used by general tx builder)."]
pub struct IntentWithTag {
    tag: Vec<u8>,
    intent: Intent,
}

#[wasm_bindgen]
impl IntentWithTag {
    #[wasm_bindgen(constructor)]
    pub fn new(tag: &[u8], intent: Intent) -> IntentWithTag {
        IntentWithTag {
            tag: tag.to_vec(),
            intent,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn tag(&self) -> Vec<u8> {
        self.tag.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn intent(&self) -> Intent {
        self.intent.clone()
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
//
// If we have CBOR specified by the following CDDL referencing the Shelley-MA CDDL:
//
// transaction_unspent_output = [
//   input: transaction_input,
//   output: transaction_output,
// ]
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
#[derive(Debug, Clone)]
#[doc = "Open intent for a new channel."]
pub struct OpenIntent {
    tag: Vec<u8>,
    adaptor_vk: Vec<u8>,
    close_period_sec: u64,
    amount: u64,
}

#[wasm_bindgen]
impl OpenIntent {
    #[wasm_bindgen(constructor)]
    pub fn new(
        channel_tag: &[u8],
        adaptor_vk: &[u8],
        close_period_sec: u64,
        amount: u64,
    ) -> OpenIntent {
        OpenIntent {
            tag: channel_tag.to_vec(),
            adaptor_vk: adaptor_vk.to_vec(),
            close_period_sec,
            amount,
        }
    }

    #[wasm_bindgen(getter, js_name = "tag")]
    pub fn tag(&self) -> Vec<u8> {
        self.tag.clone()
    }

    #[wasm_bindgen(getter, js_name = "adaptorVk")]
    pub fn adaptor_vk(&self) -> Vec<u8> {
        self.adaptor_vk.clone()
    }

    #[wasm_bindgen(getter, js_name = "closePeriodSec")]
    pub fn close_period_sec(&self) -> u64 {
        self.close_period_sec
    }

    #[wasm_bindgen(getter, js_name = "amount")]
    pub fn amount(&self) -> u64 {
        self.amount
    }
}

impl From<OpenIntent> for CoreOpenIntent {
    fn from(w: OpenIntent) -> CoreOpenIntent {
        let tag = core::Tag::from(w.tag.as_slice());

        let adaptor_vk = VerificationKey::try_from(w.adaptor_vk.clone())
            .expect("invalid adaptor verification key length");

        let close_period = Duration(std::time::Duration::from_secs(w.close_period_sec));

        CoreOpenIntent {
            tag,
            sub_vkey: adaptor_vk,
            close_period,
            amount: w.amount,
        }
    }
}

fn decode_funding_utxos(funding_utxos: Vec<js_sys::Uint8Array>) -> Result<BTreeMap<Input, Output>> {
    funding_utxos
        .into_iter()
        .map(|js_u8arr| {
            let mut bytes = vec![0u8; js_u8arr.length() as usize];
            js_u8arr.copy_to(&mut bytes);
            let tuo: TransactionUnspentOutput = minicbor::decode(&bytes)
                .map_err(|e| anyhow!("failed to decode funding_utxo: {e}"))?;
            Ok((tuo.input, tuo.output))
        })
        .collect()
}

fn build_network_parameters(network: &Network) -> (NetworkParameters, NetworkId) {
    let Network(network) = network;
    let network_id: NetworkId = network.clone().into();
    let protocol_parameters: cardano_sdk::ProtocolParameters =
        cardano_sdk::ProtocolParameters::from(*network);

    (
        NetworkParameters {
            network_id,
            protocol_parameters,
        },
        network_id,
    )
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

    let (network_parameters, _network_id) = build_network_parameters(network);

    let opens = vec![CoreOpenIntent::from(OpenIntent::new(
        channel_tag,
        adaptor_vk,
        close_period_sec,
        amount,
    ))];

    let utxos = decode_funding_utxos(funding_utxos)?;

    let tx = consumer::tx(
        &network_parameters,
        &consumer_vk,
        opens,
        BTreeMap::new(),
        &utxos,
        Bounds::twenty_mins(),
    )
    .map_err(|e| anyhow!("failed to build transaction: {e}"))?;
    Ok(tx.into())
}

#[wasm_bindgen]
pub fn add_tx(
    channel_tag: &[u8],
    consumer_vk: &[u8],
    amount: u64,
    // CIP-30 UnspentTransactionOutput
    funding_utxos: Vec<js_sys::Uint8Array>,
    network: &Network,
) -> Result<TransactionReadyForSigning> {
    let consumer_vk = VerificationKey::try_from(Vec::from(consumer_vk))
        .map_err(|_| anyhow!("invalid verification key length"))?;

    let (network_parameters, _network_id) = build_network_parameters(network);

    let tag = core::Tag::from(channel_tag);

    let mut intents: BTreeMap<core::Tag, CoreIntent> = BTreeMap::new();
    intents.insert(tag, CoreIntent::Add(amount));

    let utxos = decode_funding_utxos(funding_utxos)?;

    let tx = consumer::tx(
        &network_parameters,
        &consumer_vk,
        Vec::new(),
        intents,
        &utxos,
        Bounds::twenty_mins(),
    )
    .map_err(|e| anyhow!("failed to build transaction: {e}"))?;
    Ok(tx.into())
}

#[wasm_bindgen]
pub fn close_tx(
    channel_tag: &[u8],
    consumer_vk: &[u8],
    // CIP-30 UnspentTransactionOutput
    funding_utxos: Vec<js_sys::Uint8Array>,
    network: &Network,
) -> Result<TransactionReadyForSigning> {
    let consumer_vk = VerificationKey::try_from(Vec::from(consumer_vk))
        .map_err(|_| anyhow!("invalid verification key length"))?;

    let (network_parameters, _network_id) = build_network_parameters(network);

    let tag = core::Tag::from(channel_tag);

    let mut intents: BTreeMap<core::Tag, CoreIntent> = BTreeMap::new();
    intents.insert(tag, CoreIntent::Close);

    let utxos = decode_funding_utxos(funding_utxos)?;

    let bounds = Bounds::twenty_mins();
    if bounds.upper.is_none() {
        return Err(anyhow!("close_tx requires an upper bound").into());
    }

    let tx = consumer::tx(
        &network_parameters,
        &consumer_vk,
        Vec::new(),
        intents,
        &utxos,
        bounds,
    )
    .map_err(|e| anyhow!("failed to build transaction: {e}"))?;
    Ok(tx.into())
}

#[wasm_bindgen]
pub fn tx(
    opens: Vec<OpenIntent>,
    intents: Vec<IntentWithTag>,
    consumer_vk: &[u8],
    funding_utxos: Vec<js_sys::Uint8Array>,
    network: &Network,
) -> Result<TransactionReadyForSigning> {
    let consumer_vk = VerificationKey::try_from(Vec::from(consumer_vk))
        .map_err(|_| anyhow!("invalid verification key length"))?;

    let (network_parameters, _network_id) = build_network_parameters(network);

    let opens: Vec<CoreOpenIntent> = opens.into_iter().map(CoreOpenIntent::from).collect();

    let intents: BTreeMap<core::Tag, CoreIntent> = intents
        .into_iter()
        .map(|i| (core::Tag::from(i.tag.as_slice()), i.intent.0.clone()))
        .collect();

    let utxos = decode_funding_utxos(funding_utxos)?;

    let tx = consumer::tx(
        &network_parameters,
        &consumer_vk,
        opens,
        intents,
        &utxos,
        Bounds::twenty_mins(),
    )
    .map_err(|e| anyhow!("failed to build transaction: {e}"))?;
    Ok(tx.into())
}

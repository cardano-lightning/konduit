//! AI generated.
//!
//! Extract the Plutus V3 cost model from a local Cardano node's protocol
//! parameters response.
//!
//! This could be dropped when we bump pallas-network version so it exposes
//! the v3 cost model directly.

use anyhow::{Context, anyhow};
use minicbor::decode::Error as DecodeError;
use minicbor::{self, Decoder};
use pallas_codec::utils::AnyCbor;
use pallas_network::miniprotocols::localstate::Client;
use pallas_network::miniprotocols::localstate::queries_v16::{
    BlockQuery, Era, LedgerQuery, Request,
};

const COST_MODELS_FIELD_INDEX: u64 = 15;
const PLUTUS_V3_MAP_KEY: u64 = 2;

pub async fn extract(client: &mut Client, era: Era) -> anyhow::Result<Vec<i64>> {
    let request = AnyCbor::from_encode(Request::LedgerQuery(LedgerQuery::BlockQuery(
        era,
        BlockQuery::GetCurrentPParams,
    )));

    let response = client
        .query_any(request)
        .await
        .map_err(|error| anyhow!(error))
        .context("failed to query raw protocol parameters from local node for V3 cost model")?;

    decode_v3_from_raw(response.raw_bytes()).ok_or_else(|| {
        anyhow!("local node did not advertise a Plutus V3 cost model in its protocol parameters")
    })
}

fn decode_v3_from_raw(bytes: &[u8]) -> Option<Vec<i64>> {
    let mut decoder = Decoder::new(bytes);

    // Outer array: Vec<ProtocolParam>. Newer nodes return exactly one entry;
    // older (pre-Conway) nodes return one entry per era. We only need the first.
    let outer_len = decoder.array().ok()??;
    if outer_len == 0 {
        return None;
    }

    // ProtocolParam is encoded as a 22-element CBOR array with index-matched
    // fields (#[n(0)] .. #[n(21)]). Skip the first 15 fields (indices 0..=14),
    // landing on `cost_models_for_script_languages`.
    let params_len = decoder.array().ok()?? as usize;
    if params_len <= COST_MODELS_FIELD_INDEX as usize {
        return None;
    }

    for _ in 0..COST_MODELS_FIELD_INDEX {
        decoder.skip().ok()?;
    }

    use minicbor::data::Type;
    match decoder.datatype().ok()? {
        Type::Null => {
            decoder.skip().ok()?;
            None
        }
        Type::Map => {
            let map_len = decoder.map().ok()??;
            let mut v3 = None;
            for _ in 0..map_len {
                let key = decoder.u64().ok()?;
                if key == PLUTUS_V3_MAP_KEY {
                    v3 = Some(decode_cost_model(&mut decoder).ok()?);
                } else {
                    decoder.skip().ok()?;
                }
            }
            v3
        }
        _ => {
            decoder.skip().ok()?;
            None
        }
    }
}

fn decode_cost_model(decoder: &mut Decoder<'_>) -> Result<Vec<i64>, DecodeError> {
    let len = decoder.array()?;
    match len {
        Some(n) => {
            let mut out = Vec::with_capacity(n as usize);
            for _ in 0..n {
                let value = i64::try_from(decoder.int()?)
                    .map_err(|_| DecodeError::message("cost model value out of i64 range"))?;
                out.push(value);
            }
            Ok(out)
        }
        None => {
            let mut out = Vec::new();
            while decoder.datatype()? != minicbor::data::Type::Break {
                let value = i64::try_from(decoder.int()?)
                    .map_err(|_| DecodeError::message("cost model value out of i64 range"))?;
                out.push(value);
            }
            decoder.skip()?;
            Ok(out)
        }
    }
}

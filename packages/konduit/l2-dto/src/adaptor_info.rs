use crate::AdaptorChannelParameters;
#[cfg(feature = "cbor")]
use minicbor::{Decode, Encode};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "cbor", derive(Encode, Decode))]
pub struct AdaptorInfo<T> {
    #[cfg_attr(feature = "cbor", n(0))]
    pub tos: TosInfo,
    #[cfg_attr(feature = "cbor", n(1))]
    pub channel_parameters: AdaptorChannelParameters,
    #[cfg_attr(feature = "cbor", n(2))]
    pub tx_help: T,
}

impl From<AdaptorInfo<TxHelp>> for AdaptorInfo<()> {
    fn from(info: AdaptorInfo<TxHelp>) -> Self {
        Self {
            tos: info.tos,
            channel_parameters: info.channel_parameters,
            tx_help: (),
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "cbor", derive(Encode, Decode))]
pub struct TosInfo {
    #[cfg_attr(feature = "cbor", n(0))]
    pub flat_fee: u64,
    #[cfg_attr(feature = "cbor", n(1))]
    pub time_margin: konduit_core::Duration,
    #[cfg_attr(feature = "cbor", n(2))]
    pub rails: Vec<String>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "cbor", derive(Encode, Decode))]
pub struct TxHelp {
    #[cfg_attr(
        feature = "cbor",
        cbor(n(0), encode_with = "encode_address", decode_with = "decode_address")
    )]
    pub host_address: String,
    #[cfg_attr(feature = "serde", serde(with = "konduit_core::serde::bytes"))]
    #[cfg_attr(
        feature = "cbor",
        cbor(n(1), encode_with = "encode_hash", decode_with = "decode_hash")
    )]
    pub validator: [u8; 28],
}

#[cfg(feature = "cbor")]
fn encode_address<C, W: minicbor::encode::Write>(
    value: &str,
    e: &mut minicbor::Encoder<W>,
    _ctx: &mut C,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    use bech32::FromBase32;
    let (_, data, _) = bech32::decode(value)
        .map_err(|_| minicbor::encode::Error::message("invalid bech32 address"))?;
    let bytes = Vec::<u8>::from_base32(&data)
        .map_err(|_| minicbor::encode::Error::message("invalid bech32 address bytes"))?;
    e.bytes(&bytes)?;
    Ok(())
}

#[cfg(feature = "cbor")]
fn decode_address<'b, C>(
    d: &mut minicbor::Decoder<'b>,
    _ctx: &mut C,
) -> Result<String, minicbor::decode::Error> {
    use bech32::ToBase32;
    let bytes = d.bytes()?;
    let header = *bytes
        .first()
        .ok_or_else(|| minicbor::decode::Error::message("empty Shelley address"))?;
    let hrp = if header & 0x0f == 1 {
        "addr"
    } else {
        "addr_test"
    };
    bech32::encode(hrp, bytes.to_base32(), bech32::Variant::Bech32)
        .map_err(|_| minicbor::decode::Error::message("invalid Shelley address bytes"))
}

#[cfg(feature = "cbor")]
fn encode_hash<C, W: minicbor::encode::Write>(
    value: &[u8; 28],
    e: &mut minicbor::Encoder<W>,
    _ctx: &mut C,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    e.bytes(value)?;
    Ok(())
}

#[cfg(feature = "cbor")]
fn decode_hash<'b, C>(
    d: &mut minicbor::Decoder<'b>,
    _ctx: &mut C,
) -> Result<[u8; 28], minicbor::decode::Error> {
    d.bytes()?
        .try_into()
        .map_err(|_| minicbor::decode::Error::message("expected 28-byte hash"))
}

#[cfg(test)]
mod tests {
    use super::TxHelp;
    #[cfg(feature = "cbor")]
    use bech32::ToBase32;

    fn sample() -> TxHelp {
        #[cfg(feature = "cbor")]
        let host_address = {
            let raw_address = [0x61; 29];
            bech32::encode("addr", raw_address.to_base32(), bech32::Variant::Bech32).unwrap()
        };
        #[cfg(not(feature = "cbor"))]
        let host_address = String::from("addr1example");
        TxHelp {
            host_address,
            validator: [0x42; 28],
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn tx_help_roundtrips_json() {
        let tx_help = sample();
        let json = serde_json::to_string(&tx_help).unwrap();
        assert_eq!(
            serde_json::from_str::<TxHelp>(&json).unwrap().host_address,
            tx_help.host_address
        );
    }

    #[cfg(feature = "cbor")]
    #[test]
    fn tx_help_roundtrips_cardano_cbor_shapes() {
        let tx_help = sample();
        let cbor = minicbor::to_vec(&tx_help).unwrap();
        assert_eq!(
            minicbor::decode::<TxHelp>(&cbor).unwrap().host_address,
            tx_help.host_address
        );
    }
}

use bitcoin::secp256k1::PublicKey;
use minicbor::decode::Error as CborError;
use minicbor::{Decode, Decoder, Encode, Encoder};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[derive(
    Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize, Encode, Decode,
)]
#[serde(transparent)]
#[cbor(transparent)]
pub struct RouteHint(#[n(0)] pub Vec<RouteHintHop>);

impl From<lightning_invoice::RouteHint> for RouteHint {
    fn from(value: lightning_invoice::RouteHint) -> Self {
        Self(value.0.into_iter().map(|x| x.into()).collect::<Vec<_>>())
    }
}

impl From<RouteHint> for lightning_invoice::RouteHint {
    fn from(value: RouteHint) -> Self {
        Self(value.0.into_iter().map(|x| x.into()).collect::<Vec<_>>())
    }
}

#[serde_as]
#[derive(
    Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize, Encode, Decode,
)]
#[cbor(map)]
pub struct RouteHintHop {
    #[serde_as(as = "serde_with::DisplayFromStr")]
    #[n(0)]
    #[cbor(with = "cbor_pubkey")]
    pub src_node_id: PublicKey,

    #[n(1)]
    pub short_channel_id: u64,

    #[n(2)]
    pub fees: RoutingFees,

    #[n(3)]
    pub cltv_expiry_delta: u16,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[n(4)]
    pub htlc_minimum_msat: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[n(5)]
    pub htlc_maximum_msat: Option<u64>,
}

#[derive(
    Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize, Encode, Decode,
)]
#[cbor(map)]
pub struct RoutingFees {
    #[n(0)]
    pub base_msat: u32,
    #[n(1)]
    pub proportional_millionths: u32,
}

// --- Conversions for RouteHintHop ---

impl From<lightning_invoice::RouteHintHop> for RouteHintHop {
    fn from(value: lightning_invoice::RouteHintHop) -> Self {
        let lightning_invoice::RouteHintHop {
            src_node_id,
            short_channel_id,
            fees,
            cltv_expiry_delta,
            htlc_minimum_msat,
            htlc_maximum_msat,
        } = value;
        Self {
            src_node_id,
            short_channel_id,
            fees: RoutingFees::from(fees),
            cltv_expiry_delta,
            htlc_minimum_msat,
            htlc_maximum_msat,
        }
    }
}

impl From<RouteHintHop> for lightning_invoice::RouteHintHop {
    fn from(value: RouteHintHop) -> Self {
        let RouteHintHop {
            src_node_id,
            short_channel_id,
            fees,
            cltv_expiry_delta,
            htlc_minimum_msat,
            htlc_maximum_msat,
        } = value;
        Self {
            src_node_id,
            short_channel_id,
            fees: lightning_invoice::RoutingFees::from(fees),
            cltv_expiry_delta,
            htlc_minimum_msat,
            htlc_maximum_msat,
        }
    }
}

// --- Conversions for RoutingFees ---

impl From<lightning_invoice::RoutingFees> for RoutingFees {
    fn from(value: lightning_invoice::RoutingFees) -> Self {
        let lightning_invoice::RoutingFees {
            base_msat,
            proportional_millionths,
        } = value;
        Self {
            base_msat,
            proportional_millionths,
        }
    }
}

impl From<RoutingFees> for lightning_invoice::RoutingFees {
    fn from(value: RoutingFees) -> Self {
        let RoutingFees {
            base_msat,
            proportional_millionths,
        } = value;
        Self {
            base_msat,
            proportional_millionths,
        }
    }
}

/// Helper module to bridge secp256k1::PublicKey with minicbor's byte handling.
mod cbor_pubkey {
    use super::*;

    pub fn encode<W: minicbor::encode::Write, C>(
        v: &PublicKey,
        e: &mut Encoder<W>,
        _: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.bytes(&v.serialize())?;
        Ok(())
    }

    pub fn decode<'b, C>(d: &mut Decoder<'b>, _: &mut C) -> Result<PublicKey, CborError> {
        let bytes = d.bytes()?;
        PublicKey::from_slice(bytes)
            .map_err(|_| CborError::message("Invalid secp256k1 public key bytes"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_hint_cbor_roundtrip() {
        let hop = RouteHintHop {
            src_node_id: PublicKey::from_slice(&[0x02; 33]).unwrap(),
            short_channel_id: 840000,
            fees: RoutingFees {
                base_msat: 1000,
                proportional_millionths: 1,
            },
            cltv_expiry_delta: 144,
            htlc_minimum_msat: Some(1),
            htlc_maximum_msat: None,
        };
        let rh = RouteHint(vec![hop]);

        let mut buffer = Vec::new();
        minicbor::encode(&rh, &mut buffer).expect("CBOR Encoding failed");

        let decoded: RouteHint = minicbor::decode(&buffer).expect("CBOR Decoding failed");
        assert_eq!(rh, decoded);
    }
}

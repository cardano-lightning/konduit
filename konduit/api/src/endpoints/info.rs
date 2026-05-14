use cardano_sdk::VerificationKey;
use cardano_sdk::{Address, Hash, address::kind::Shelley};
use konduit_data::Duration;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[cfg_attr(feature = "cddl", derive(konduit_cddl::ToCddl))]
#[cfg_attr(feature = "cddl", cddl(name = "info-response"))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Response {
    /// Terms of service. Purely informational
    #[n(0)]
    pub tos: TosInfo,
    /// Channel parameters
    #[n(1)]
    pub channel_parameters: ChannelParameters,
    /// Tx building
    #[n(2)]
    pub tx_help: TxHelp,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[cfg_attr(feature = "cddl", derive(konduit_cddl::ToCddl))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TosInfo {
    #[n(0)]
    pub flat_fee: u64,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[cfg_attr(feature = "cddl", derive(konduit_cddl::ToCddl))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TxHelp {
    #[cbor(n(0), with = "cbor_with::display_from_str")]
    #[serde_as(as = "serde_with::DisplayFromStr")]
    #[cfg_attr(feature = "cddl", cddl(ty = "text"))]
    pub host_address: Address<Shelley>,
    #[n(1)]
    #[serde_as(as = "serde_with::hex::Hex")]
    #[cfg_attr(feature = "cddl", cddl(bytes))]
    pub validator: Hash<28>,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[cfg_attr(feature = "cddl", derive(konduit_cddl::ToCddl))]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ChannelParameters {
    #[cbor(n(0), with = "cbor_with::fixed_bytes_32")]
    #[serde_as(as = "serde_with::hex::Hex")]
    #[cfg_attr(feature = "cddl", cddl(bytes))]
    pub adaptor_key: VerificationKey,
    #[cbor(n(1), with = "konduit_data::cbor_with::plutus_data")]
    #[cfg_attr(feature = "cddl", cddl(ty = "duration"))]
    pub close_period: Duration,
    #[n(2)]
    pub tag_length: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_vk() -> VerificationKey {
        VerificationKey::from([0u8; 32])
    }

    fn sample_vk_nonzero() -> VerificationKey {
        VerificationKey::from([0xabu8; 32])
    }

    fn sample() -> ChannelParameters {
        ChannelParameters {
            adaptor_key: sample_vk(),
            close_period: Duration::from_secs(3600),
            tag_length: 8,
        }
    }

    fn roundtrip(val: &ChannelParameters) -> ChannelParameters {
        let bytes = minicbor::to_vec(val).expect("encode failed");
        minicbor::decode(&bytes).expect("decode failed")
    }

    #[test]
    fn roundtrip_basic() {
        let original = sample();
        let decoded = roundtrip(&original);
        assert_eq!(
            <[u8; 32]>::from(original.adaptor_key),
            <[u8; 32]>::from(decoded.adaptor_key)
        );
        assert_eq!(original.close_period, decoded.close_period);
        assert_eq!(original.tag_length, decoded.tag_length);
    }

    #[test]
    fn roundtrip_nonzero_vk() {
        let original = ChannelParameters {
            adaptor_key: sample_vk_nonzero(),
            ..sample()
        };
        let decoded = roundtrip(&original);
        assert_eq!(
            <[u8; 32]>::from(original.adaptor_key),
            <[u8; 32]>::from(decoded.adaptor_key)
        );
    }

    #[test]
    fn roundtrip_zero_duration() {
        let original = ChannelParameters {
            close_period: Duration::from_secs(0),
            ..sample()
        };
        let decoded = roundtrip(&original);
        assert_eq!(original.close_period, decoded.close_period);
    }

    #[test]
    fn roundtrip_large_duration() {
        let original = ChannelParameters {
            close_period: Duration::from_millis(u64::MAX),
            ..sample()
        };
        let decoded = roundtrip(&original);
        assert_eq!(original.close_period, decoded.close_period);
    }

    #[test]
    fn roundtrip_tag_length_zero() {
        let original = ChannelParameters {
            tag_length: 0,
            ..sample()
        };
        let decoded = roundtrip(&original);
        assert_eq!(original.tag_length, decoded.tag_length);
    }

    #[test]
    fn roundtrip_tag_length_large() {
        let original = ChannelParameters {
            tag_length: usize::MAX,
            ..sample()
        };
        let decoded = roundtrip(&original);
        assert_eq!(original.tag_length, decoded.tag_length);
    }

    #[test]
    fn encoding_is_stable() {
        let bytes1 = minicbor::to_vec(&sample()).unwrap();
        let bytes2 = minicbor::to_vec(&sample()).unwrap();
        assert_eq!(bytes1, bytes2);
    }
}

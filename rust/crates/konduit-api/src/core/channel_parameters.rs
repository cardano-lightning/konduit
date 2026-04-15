use cardano_sdk::VerificationKey;
use konduit_data::Duration;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::cbor_with;

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct ChannelParameters {
    #[cbor(n(0), with = "cbor_with::via_bytes_fixed")]
    #[serde_as(as = "serde_with::hex::Hex")]
    pub adaptor_key: VerificationKey,
    #[cbor(n(1), with = "cbor_with::via_plutus_data")]
    pub close_period: Duration,
    #[cbor(n(2))]
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

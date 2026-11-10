use konduit_core::{Constants, Stage};
#[cfg(feature = "cbor")]
use minicbor::{Decode, Encode};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "cbor", derive(Encode, Decode))]
pub struct L1Channel {
    #[cfg_attr(feature = "cbor", n(0))]
    pub amount: u64,
    #[cfg_attr(feature = "cbor", n(1))]
    pub constants: Constants,
    #[cfg_attr(feature = "cbor", n(2))]
    pub stage: Stage,
}

#[cfg(test)]
mod tests {
    use super::L1Channel;
    use konduit_core::{Constants, Duration, Stage, Tag, VerifyingKey};

    #[test]
    fn full_channel_roundtrips_json_and_cbor() {
        let channel = L1Channel {
            amount: 42,
            constants: Constants {
                tag: Tag::from(vec![7; 8]),
                add_vkey: VerifyingKey::from([1; 32]),
                sub_vkey: VerifyingKey::from([2; 32]),
                close_period: Duration::from_secs(60),
            },
            stage: Stage::Opened(0, vec![]),
        };

        #[cfg(feature = "serde")]
        {
            let json = serde_json::to_string(&channel).unwrap();
            assert_eq!(serde_json::from_str::<L1Channel>(&json).unwrap(), channel);
        }

        #[cfg(feature = "cbor")]
        {
            let cbor = minicbor::to_vec(&channel).unwrap();
            assert_eq!(minicbor::decode::<L1Channel>(&cbor).unwrap(), channel);
        }
    }

    #[cfg(feature = "cbor")]
    #[test]
    fn constants_cbor_matches_cardano_plutus_data() {
        use cardano_sdk::{PlutusData, cbor::ToCbor};

        let constants = Constants {
            tag: Tag::from(vec![7; 8]),
            add_vkey: VerifyingKey::from([1; 32]),
            sub_vkey: VerifyingKey::from([2; 32]),
            close_period: Duration::from_secs(60),
        };
        let plutus = PlutusData::list(vec![
            PlutusData::bytes(constants.tag.as_ref()),
            PlutusData::bytes(constants.add_vkey.as_ref()),
            PlutusData::bytes(constants.sub_vkey.as_ref()),
            PlutusData::integer(constants.close_period.as_millis() as u64),
        ]);

        assert_eq!(minicbor::to_vec(&constants).unwrap(), plutus.to_cbor());
    }
}

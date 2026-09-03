#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum AssetError {
    #[error("asset name exceeds Cardano's 32-byte limit")]
    AssetNameTooLong,
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum AssetId {
    Ada,
    Native(NativeAsset),
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct NativeAsset {
    policy_id: [u8; 28],
    asset_name: Vec<u8>,
}

impl AssetId {
    pub fn native(policy_id: [u8; 28], asset_name: Vec<u8>) -> Result<Self, AssetError> {
        if asset_name.len() > 32 {
            return Err(AssetError::AssetNameTooLong);
        }
        Ok(Self::Native(NativeAsset {
            policy_id,
            asset_name,
        }))
    }

    pub fn policy_id(&self) -> Option<&[u8; 28]> {
        let Self::Native(native) = self else {
            return None;
        };
        Some(&native.policy_id)
    }

    pub fn asset_name(&self) -> Option<&[u8]> {
        let Self::Native(native) = self else {
            return None;
        };
        Some(&native.asset_name)
    }
}

impl<C> minicbor::Encode<C> for AssetId {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        match self {
            Self::Ada => {
                e.tag(minicbor::data::Tag::new(121))?;
                e.array(0)?;
            }
            Self::Native(NativeAsset {
                policy_id,
                asset_name,
            }) => {
                e.tag(minicbor::data::Tag::new(122))?;
                e.begin_array()?
                    .bytes(policy_id)?
                    .bytes(asset_name)?
                    .end()?;
            }
        }
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for AssetId {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        let variant = match d.tag()?.as_u64() {
            121 => 0,
            122 => 1,
            tag => {
                return Err(minicbor::decode::Error::message(format!(
                    "unknown AssetId CBOR tag {tag}"
                )));
            }
        };
        let len = d.array()?;
        let asset = match variant {
            0 if len == Some(0) => Self::Ada,
            0 => {
                return Err(minicbor::decode::Error::message(
                    "Ada AssetId must have no fields",
                ));
            }
            1 if len.is_none() || len == Some(2) => {
                let policy_id = <[u8; 28]>::try_from(d.bytes()?)
                    .map_err(|_| minicbor::decode::Error::message("policy id must be 28 bytes"))?;
                let asset_name = d.bytes()?.to_vec();
                Self::native(policy_id, asset_name).map_err(|_| {
                    minicbor::decode::Error::message("asset name must be at most 32 bytes")
                })?
            }
            1 => {
                return Err(minicbor::decode::Error::message(
                    "native AssetId must have two fields",
                ));
            }
            _ => unreachable!(),
        };
        if len.is_none() {
            if d.datatype()? != minicbor::data::Type::Break {
                return Err(minicbor::decode::Error::message(
                    "expected end of AssetId array",
                ));
            }
            d.skip()?;
        }
        Ok(asset)
    }
}

#[cfg(feature = "serde")]
mod via_serde {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

    #[derive(Serialize)]
    #[serde(tag = "kind", rename_all = "snake_case")]
    enum Ref {
        Ada,
        Native {
            policy_id: String,
            asset_name: String,
        },
    }

    #[derive(Deserialize)]
    #[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
    enum Owned {
        Ada,
        Native {
            policy_id: String,
            asset_name: String,
        },
    }

    impl Serialize for AssetId {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            match self {
                AssetId::Ada => Ref::Ada.serialize(serializer),
                AssetId::Native(NativeAsset {
                    policy_id,
                    asset_name,
                }) => Ref::Native {
                    policy_id: hex::encode(policy_id),
                    asset_name: hex::encode(asset_name),
                }
                .serialize(serializer),
            }
        }
    }

    impl<'de> Deserialize<'de> for AssetId {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            match Owned::deserialize(deserializer)? {
                Owned::Ada => Ok(Self::Ada),
                Owned::Native {
                    policy_id,
                    asset_name,
                } => {
                    if policy_id.len() != 56
                        || asset_name.len() > 64
                        || asset_name.len() % 2 != 0
                        || !policy_id
                            .bytes()
                            .chain(asset_name.bytes())
                            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                    {
                        return Err(D::Error::custom(
                            "native asset requires 56 lowercase policy hex and 0-64 even lowercase name hex",
                        ));
                    }
                    let policy_id = hex::decode(policy_id).map_err(D::Error::custom)?;
                    let asset_name = hex::decode(asset_name).map_err(D::Error::custom)?;
                    Self::native(
                        policy_id.try_into().map_err(|_| {
                            D::Error::custom("native asset policy id must be 28 bytes")
                        })?,
                        asset_name,
                    )
                    .map_err(D::Error::custom)
                }
            }
        }
    }
}

#[cfg(feature = "cardano_sdk")]
mod via_plutus_data {
    use super::*;
    use anyhow::{Context, anyhow};
    use cardano_sdk::{PlutusData, constr};

    impl<'a> TryFrom<&PlutusData<'a>> for AssetId {
        type Error = anyhow::Error;

        fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
            let (variant, fields): (u64, Vec<PlutusData<'_>>) = data.try_into()?;
            match variant {
                0 => {
                    if fields.is_empty() {
                        Ok(Self::Ada)
                    } else {
                        Err(anyhow!("invalid Ada asset: expected no fields"))
                    }
                }
                1 => {
                    let [policy, name] = <[PlutusData; 2]>::try_from(fields)
                        .map_err(|fields| anyhow!("expected 2 fields, found {}", fields.len()))?;
                    Self::native(
                        <[u8; 28]>::try_from(<&[u8]>::try_from(&policy)?)
                            .context("policy id must be 28 bytes")?,
                        <&[u8]>::try_from(&name)?.to_vec(),
                    )
                    .map_err(Into::into)
                }
                _ => Err(anyhow!("unknown AssetId variant: {variant}")),
            }
        }
    }

    impl<'a> From<AssetId> for PlutusData<'a> {
        fn from(value: AssetId) -> Self {
            match value {
                AssetId::Ada => constr!(0),
                AssetId::Native(NativeAsset {
                    policy_id,
                    asset_name,
                }) => constr!(
                    1,
                    PlutusData::bytes(policy_id),
                    PlutusData::bytes(asset_name)
                ),
            }
        }
    }
}

#[cfg(feature = "proptest")]
impl proptest::arbitrary::Arbitrary for AssetId {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        prop_oneof![
            Just(Self::Ada),
            (
                any::<[u8; 28]>(),
                proptest::collection::vec(any::<u8>(), 0..=32)
            )
                .prop_map(|(policy_id, asset_name)| Self::native(policy_id, asset_name).unwrap()),
        ]
        .boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cardano_asset_name_bounds() {
        assert!(AssetId::native([0; 28], vec![]).is_ok());
        assert!(AssetId::native([0; 28], vec![0; 32]).is_ok());
        assert_eq!(
            AssetId::native([0; 28], vec![0; 33]),
            Err(AssetError::AssetNameTooLong)
        );
    }

    #[test]
    fn cbor_rejects_long_asset_names() {
        let mut encoder = minicbor::Encoder::new(Vec::new());
        encoder
            .tag(minicbor::data::Tag::new(122))
            .unwrap()
            .array(2)
            .unwrap()
            .bytes(&[0; 28])
            .unwrap()
            .bytes(&[0; 33])
            .unwrap();
        assert!(minicbor::decode::<AssetId>(&encoder.into_writer()).is_err());
    }

    #[cfg(feature = "json")]
    #[test]
    fn serde_rejects_long_asset_names() {
        let json = format!(
            r#"{{"kind":"native","policy_id":"{}","asset_name":"{}"}}"#,
            "00".repeat(28),
            "00".repeat(33),
        );
        assert!(serde_json::from_str::<AssetId>(&json).is_err());
    }

    #[cfg(feature = "cardano_sdk")]
    #[test]
    fn plutus_rejects_long_asset_names() {
        use cardano_sdk::{PlutusData, constr};

        let data = constr!(1, PlutusData::bytes([0; 28]), PlutusData::bytes([0; 33]));
        assert!(AssetId::try_from(&data).is_err());
    }

    #[cfg(feature = "proptest")]
    proptest::proptest! {
        #[test]
        fn cbor_and_plutus_roundtrip(asset: AssetId) {
            use cardano_sdk::{PlutusData, cbor::ToCbor};
            let cbor = minicbor::to_vec(&asset).unwrap();
            let decoded: AssetId = minicbor::decode(&cbor).unwrap();
            proptest::prop_assert_eq!(&asset, &decoded);
            let plutus = PlutusData::from(asset.clone());
            proptest::prop_assert_eq!(cbor, plutus.to_cbor());
            proptest::prop_assert_eq!(asset, AssetId::try_from(&plutus).unwrap());
        }
    }
}

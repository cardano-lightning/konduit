use std::marker::PhantomData;

use crate::{
    Indexes, SquashBody, Tag, Unverified, Verified,
    utils::{signature_from_plutus_data, signature_to_plutus_data},
};
use anyhow::anyhow;
use cardano_sdk::{PlutusData, Signature, SigningKey, VerificationKey};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[cfg_attr(feature = "cddl", derive(cuddly::ToCddl))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Squash<U = Unverified> {
    #[cfg_attr(feature = "cddl", n(0))]
    body: SquashBody,
    #[cfg_attr(feature = "cddl", n(1))]
    #[serde_as(as = "serde_with::hex::Hex")]
    #[cfg_attr(feature = "cddl", cddl(bytes))]
    signature: Signature,
    #[serde(skip)]
    #[cfg_attr(feature = "cddl", cddl(skip))]
    _marker: PhantomData<U>,
}

// =========================================================================
// Universal Methods (Available on both Verified and Unverified states)
// =========================================================================
impl<U> Squash<U> {
    /// Internal constructor to associate state markers.
    pub fn new_with_state(body: SquashBody, signature: Signature) -> Self {
        Self {
            body,
            signature,
            _marker: PhantomData,
        }
    }

    pub fn body(&self) -> &SquashBody {
        &self.body
    }

    pub fn index(&self) -> u64 {
        self.body.index()
    }

    pub fn amount(&self) -> u64 {
        self.body.amount()
    }

    pub fn exclude(&self) -> &Indexes {
        self.body.exclude()
    }

    pub fn is_index_squashed(&self, index: u64) -> bool {
        self.body.is_index_squashed(index)
    }
}

// =========================================================================
// Unverified State Methods
// =========================================================================
impl Squash<Unverified> {
    /// Creates a new, unverified cheque from a raw body and signature.
    pub fn new(body: SquashBody, signature: Signature) -> Self {
        Self::new_with_state(body, signature)
    }
}

// =========================================================================
// Unverified State Methods
// =========================================================================
impl Squash<Unverified> {
    /// Verifies the cryptographic signature against the verifying key and tag.
    /// On success, consumes the unverified cheque and transitions it to `Squash<Verified>`.
    pub fn try_verify(
        self,
        verification_key: &VerificationKey,
        tag: &Tag,
    ) -> anyhow::Result<Squash<Verified>> {
        if verification_key.verify(tag.data(self.body()), &self.signature) {
            Ok(Squash::new_with_state(self.body, self.signature))
        } else {
            Err(anyhow::anyhow!("Invalid cryptographic signature on Squash"))
        }
    }
}

// =========================================================================
// Verified State Methods
// =========================================================================
impl Squash<Verified> {
    /// Signing a new cheque inherently guarantees its authenticity,
    /// so the constructor immediately returns a `Squash<Verified>` instance.
    pub fn make(signing_key: &SigningKey, tag: &Tag, body: SquashBody) -> Self {
        let signature = signing_key.sign(tag.data(&body));
        Self::new_with_state(body, signature)
    }
}

#[cfg(feature = "proptest")]
impl proptest::arbitrary::Arbitrary for Squash {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;
    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        (any::<SquashBody>(), any::<[u8; 64]>())
            .prop_map(|(body, sig_bytes)| Squash::new(body, Signature::from(sig_bytes)))
            .boxed()
    }
}

impl<'a> TryFrom<Vec<PlutusData<'a>>> for Squash {
    type Error = anyhow::Error;

    fn try_from(value: Vec<PlutusData<'a>>) -> anyhow::Result<Self> {
        Self::try_from(<[PlutusData; 2]>::try_from(value).map_err(|_| anyhow!("Bad length"))?)
    }
}

impl<'a> TryFrom<[PlutusData<'a>; 2]> for Squash {
    type Error = anyhow::Error;

    fn try_from(value: [PlutusData<'a>; 2]) -> anyhow::Result<Self> {
        let [a, b] = value;
        Ok(Self::new(
            SquashBody::try_from(a)?,
            signature_from_plutus_data(&b)?,
        ))
    }
}

impl<'a> TryFrom<PlutusData<'a>> for Squash {
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        Self::try_from(<[PlutusData; 2]>::try_from(&data)?)
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for Squash {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        Self::try_from(<[PlutusData; 2]>::try_from(data)?)
    }
}

impl<'a> From<Squash> for [PlutusData<'a>; 2] {
    fn from(value: Squash) -> Self {
        [
            PlutusData::from(&value.body),
            signature_to_plutus_data(value.signature),
        ]
    }
}

impl<'a> From<Squash> for PlutusData<'a> {
    fn from(value: Squash) -> Self {
        Self::list(<[PlutusData; 2]>::from(value).to_vec())
    }
}

impl<'a> From<Squash<Verified>> for [PlutusData<'a>; 2] {
    fn from(value: Squash<Verified>) -> Self {
        [
            PlutusData::from(&value.body),
            signature_to_plutus_data(value.signature),
        ]
    }
}

impl<'a> From<Squash<Verified>> for PlutusData<'a> {
    fn from(value: Squash<Verified>) -> Self {
        Self::list(<[PlutusData; 2]>::from(value).to_vec())
    }
}

#[cfg(test)]
mod tests {
    use crate::Indexes;

    use super::*;

    #[test]
    fn test_squash_round_trip() {
        let sk = SigningKey::from([0; 32]);
        let tag = Tag::from([1; 20].to_vec());
        let body = SquashBody::new_no_verify(120309, 123, Indexes::new([22].to_vec()).unwrap());
        let original = Squash::make(&sk, &tag, body);

        println!("{}", serde_json::to_string_pretty(&original).unwrap());
        let ser = serde_json::to_vec(&original).expect("Failed to serialize ChequeBody");

        let de: Squash = serde_json::from_slice(&ser).expect("Failed to deserialize ChequeBody");

        assert_eq!(original.body, de.body);
    }
}

use crate::{
    Duration, Lock, Tag,
    cheque_body::ChequeBody,
    utils::{signature_from_plutus_data, signature_to_plutus_data},
};
use anyhow::{Error, Result, anyhow};
use cardano_sdk::{PlutusData, Signature, SigningKey, VerificationKey};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[serde_as]
#[cfg_attr(feature = "cddl", derive(cuddly::ToCddl))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Locked {
    #[cfg_attr(feature = "cddl", n(0))]
    pub body: ChequeBody,
    #[cfg_attr(feature = "cddl", n(1))]
    #[serde_as(as = "serde_with::hex::Hex")]
    #[cfg_attr(feature = "cddl", cddl(bytes))]
    pub signature: Signature,
}

impl Locked {
    pub fn new(body: ChequeBody, signature: Signature) -> Self {
        Self { body, signature }
    }

    pub fn index(&self) -> u64 {
        self.body.index
    }

    pub fn amount(&self) -> u64 {
        self.body.amount
    }

    pub fn timeout(&self) -> Duration {
        self.body.timeout
    }

    pub fn lock(&self) -> &Lock {
        &self.body.lock
    }

    pub fn make(signing_key: &SigningKey, tag: &Tag, body: ChequeBody) -> Self {
        let signature = signing_key.sign(tag.data(&body));
        Self::new(body, signature)
    }

    pub fn verify(&self, verification_key: &VerificationKey, tag: &Tag) -> bool {
        verification_key.verify(tag.data(&self.body), &self.signature)
    }
}

#[cfg(feature = "proptest")]
impl proptest::arbitrary::Arbitrary for Locked {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;
    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        (any::<ChequeBody>(), any::<[u8; 64]>())
            .prop_map(|(body, sig_bytes)| Locked::new(body, Signature::from(sig_bytes)))
            .boxed()
    }
}

impl<'a> TryFrom<&PlutusData<'a>> for Locked {
    type Error = Error;

    fn try_from(data: &PlutusData<'a>) -> Result<Self> {
        let fields: Vec<PlutusData<'_>> = Vec::try_from(data)?;
        Self::try_from(fields)
    }
}

impl<'a> TryFrom<PlutusData<'a>> for Locked {
    type Error = Error;

    fn try_from(data: PlutusData<'a>) -> Result<Self> {
        let fields: Vec<PlutusData<'_>> = Vec::try_from(&data)?;
        Self::try_from(fields)
    }
}

impl<'a> TryFrom<Vec<PlutusData<'a>>> for Locked {
    type Error = Error;

    fn try_from(list: Vec<PlutusData<'a>>) -> Result<Self> {
        let [a, b] = <[PlutusData; 2]>::try_from(list).map_err(|_| anyhow!("invalid 'Locked'"))?;
        Ok(Self::new(
            ChequeBody::try_from(a)?,
            signature_from_plutus_data(&b)?,
        ))
    }
}

impl<'a> From<Locked> for PlutusData<'a> {
    fn from(locked: Locked) -> Self {
        PlutusData::list(Vec::from(locked))
    }
}

impl<'a> From<Locked> for Vec<PlutusData<'a>> {
    fn from(locked: Locked) -> Self {
        vec![
            PlutusData::from(locked.body),
            signature_to_plutus_data(locked.signature),
        ]
    }
}

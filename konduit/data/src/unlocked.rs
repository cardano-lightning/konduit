use crate::{
    ChequeSigned, Locked, Secret, Tag, Unverified, Verified, VerifyState,
    cheque_body::ChequeBody,
    utils::{signature_from_plutus_data, signature_to_plutus_data},
};
use cardano_sdk::{PlutusData, SigningKey, VerificationKey};

pub type Unlocked<V = Unverified> = ChequeSigned<Secret, V>;

// =========================================================================
// Universal Methods (Available on both Verified and Unverified states)
// =========================================================================
impl<V: VerifyState> Unlocked<V> {
    pub fn try_from_locked(locked: &Locked<V>, secret: Secret) -> Result<Self, &str> {
        locked
            .body()
            .try_unlocked(secret)
            .map(|body| Self::new_with_state(body, locked.signature))
    }

    pub fn locked(&self) -> Locked<V> {
        Locked::new_with_state(self.body().locked(), self.signature)
    }

    pub fn secret(&self) -> &Secret {
        self.latch()
    }
}

// =========================================================================
// Unverified State Methods
// =========================================================================
impl Unlocked<Unverified> {
    /// Verifies the cryptographic signature against the verifying key and tag.
    /// On success, consumes the unverified cheque and transitions it to `Unlocked<Verified>`.
    pub fn try_verify(
        self,
        verification_key: &VerificationKey,
        tag: &Tag,
    ) -> anyhow::Result<Unlocked<Verified>> {
        if verification_key.verify(tag.data(&self.body), &self.signature) {
            Ok(Unlocked::new_with_state(self.body, self.signature))
        } else {
            Err(anyhow::anyhow!(
                "Invalid cryptographic signature on locked cheque"
            ))
        }
    }
}

// =========================================================================
// Verified State Methods
// =========================================================================
impl Unlocked<Verified> {
    /// Signing a new cheque inherently guarantees its authenticity,
    /// so the constructor immediately returns a `Unlocked<Verified>` instance.
    pub fn make(signing_key: &SigningKey, tag: &Tag, body: ChequeBody<Secret>) -> Self {
        let signature = signing_key.sign(tag.data(&body.locked()));
        Self::new_with_state(body, signature)
    }
}

// =========================================================================
// Testing Utilities
// =========================================================================
#[cfg(feature = "proptest")]
impl proptest::arbitrary::Arbitrary for Unlocked<Unverified> {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;
    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        (any::<ChequeBody>(), any::<[u8; 64]>())
            .prop_map(|(body, sig_bytes)| Unlocked::new(body, Signature::from(sig_bytes)))
            .boxed()
    }
}

// =========================================================================
// Serialization & Deserialization (PlutusData Conversions)
// Incoming deserializations strictly default to `Unverified`.
// Outgoing serializations are supported for any state `S`.
// =========================================================================
impl<'a> TryFrom<&PlutusData<'a>> for Unlocked<Unverified> {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        let fields: Vec<PlutusData<'_>> = Vec::try_from(data)?;
        Self::try_from(fields)
    }
}

impl<'a> TryFrom<PlutusData<'a>> for Unlocked<Unverified> {
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        let fields: Vec<PlutusData<'_>> = Vec::try_from(&data)?;
        Self::try_from(fields)
    }
}

impl<'a> TryFrom<Vec<PlutusData<'a>>> for Unlocked<Unverified> {
    type Error = anyhow::Error;

    fn try_from(list: Vec<PlutusData<'a>>) -> anyhow::Result<Self> {
        let [a, b] =
            <[PlutusData; 2]>::try_from(list).map_err(|_| anyhow::anyhow!("invalid 'Unlocked'"))?;
        Ok(Self::new(
            ChequeBody::try_from(a)?,
            signature_from_plutus_data(&b)?,
        ))
    }
}

impl<'a, V: VerifyState> From<Unlocked<V>> for PlutusData<'a> {
    fn from(locked: Unlocked<V>) -> Self {
        PlutusData::list(Vec::from(locked))
    }
}

impl<'a, V: VerifyState> From<Unlocked<V>> for Vec<PlutusData<'a>> {
    fn from(locked: Unlocked<V>) -> Self {
        vec![
            PlutusData::from(locked.body),
            signature_to_plutus_data(locked.signature),
        ]
    }
}

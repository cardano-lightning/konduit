use cryptoxide::ed25519;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Ed25519 verification key (public key), 32 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VerifyingKey(
    #[cfg_attr(
        feature = "serde",
        serde(with = "serde_with::As::<serde_with::hex::Hex>")
    )]
    [u8; 32],
);

impl VerifyingKey {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn to_bytes(self) -> [u8; 32] {
        self.0
    }

    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        ed25519::verify(message, &self.0, &signature.0)
    }
}

impl AsRef<[u8]> for VerifyingKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl TryFrom<&[u8]> for VerifyingKey {
    type Error = std::array::TryFromSliceError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        <[u8; 32]>::try_from(value).map(Self::from)
    }
}

impl TryFrom<Vec<u8>> for VerifyingKey {
    type Error = Vec<u8>;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        <[u8; 32]>::try_from(value).map(Self::from)
    }
}

impl From<[u8; 32]> for VerifyingKey {
    fn from(b: [u8; 32]) -> Self {
        Self(b)
    }
}

impl From<VerifyingKey> for [u8; 32] {
    fn from(k: VerifyingKey) -> Self {
        k.0
    }
}

impl<C> minicbor::Encode<C> for VerifyingKey {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.bytes(&self.0)?;
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for VerifyingKey {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        let raw: Vec<u8> = d.bytes_iter()?.try_fold(Vec::new(), |mut acc, c| {
            acc.extend_from_slice(c?);
            Ok(acc)
        })?;
        let arr: [u8; 32] = raw
            .try_into()
            .map_err(|_| minicbor::decode::Error::message("VerifyingKey must be 32 bytes"))?;
        Ok(Self(arr))
    }
}

// =========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Signature(
    #[cfg_attr(
        feature = "serde",
        serde(with = "serde_with::As::<serde_with::hex::Hex>")
    )]
    [u8; 64],
);

/// Ed25519 signature, 64 bytes.
impl Signature {
    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        Self(bytes)
    }

    pub fn to_bytes(self) -> [u8; 64] {
        self.0
    }
}

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; 64]> for Signature {
    fn from(b: [u8; 64]) -> Self {
        Self(b)
    }
}

impl From<Signature> for [u8; 64] {
    fn from(s: Signature) -> Self {
        s.0
    }
}

impl<C> minicbor::Encode<C> for Signature {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.bytes(&self.0)?;
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for Signature {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<Self, minicbor::decode::Error> {
        let raw: Vec<u8> = d.bytes_iter()?.try_fold(Vec::new(), |mut acc, c| {
            acc.extend_from_slice(c?);
            Ok(acc)
        })?;
        let arr: [u8; 64] = raw
            .try_into()
            .map_err(|_| minicbor::decode::Error::message("Signature must be 64 bytes"))?;
        Ok(Self(arr))
    }
}

// =========================================================================

/// Ed25519 signing key (secret key), 32 bytes. Not serialized.
#[derive(Clone)]
pub struct SigningKey([u8; 32]);

impl SigningKey {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn sign(&self, message: &[u8]) -> Signature {
        let (kp, _) = ed25519::keypair(&self.0);
        Signature(ed25519::signature(message, &kp))
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        let (_, pk) = ed25519::keypair(&self.0);
        VerifyingKey(pk)
    }
}

impl From<[u8; 32]> for SigningKey {
    fn from(b: [u8; 32]) -> Self {
        Self(b)
    }
}

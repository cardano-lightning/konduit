use crate::{Duration, Unverified, Verified, VerifyState, cheque_body::ChequeBody};
use cardano_sdk::Signature;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::marker::PhantomData;

#[serde_as]
#[cfg_attr(feature = "cddl", derive(cuddly::ToCddl))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(bound = "T: Serialize + for<'de2> Deserialize<'de2>")]
pub struct ChequeSigned<T, V: VerifyState = Unverified> {
    #[cfg_attr(feature = "cddl", n(0))]
    pub body: ChequeBody<T>,
    #[cfg_attr(feature = "cddl", n(1))]
    #[serde_as(as = "serde_with::hex::Hex")]
    #[cfg_attr(feature = "cddl", cddl(bytes))]
    pub signature: Signature,
    #[serde(skip)]
    #[cfg_attr(feature = "cddl", cddl(skip))]
    pub _marker: PhantomData<V>,
}

// =========================================================================
// Universal Methods (Available on both Verified and Unverified states)
// =========================================================================
impl<S, V: VerifyState> ChequeSigned<S, V> {
    /// Internal constructor to associate state markers.
    pub fn new_with_state(body: ChequeBody<S>, signature: Signature) -> Self {
        Self {
            body,
            signature,
            _marker: PhantomData,
        }
    }

    pub fn body(&self) -> &ChequeBody<S> {
        &self.body
    }

    pub fn index(&self) -> u64 {
        self.body.index()
    }

    pub fn amount(&self) -> u64 {
        self.body.amount()
    }

    pub fn timeout(&self) -> Duration {
        self.body.timeout()
    }

    pub fn latch(&self) -> &S {
        self.body.latch()
    }
}

// =========================================================================
// Unverified State Methods
// =========================================================================
impl<T: Clone> ChequeSigned<T, Unverified> {
    /// Creates a new, unverified cheque from a raw body and signature.
    pub fn new(body: ChequeBody<T>, signature: Signature) -> Self {
        Self::new_with_state(body, signature)
    }

    /// The unsafe version. Suitable when the data comes from a trusted source,
    /// such as your own database.
    pub fn skip_verify(self) -> ChequeSigned<T, Verified> {
        ChequeSigned {
            body: self.body,
            signature: self.signature,
            _marker: PhantomData,
        }
    }
}

// =========================================================================
// minicbor Serialization
//
// Encoding: indefinite-length array of [body, signature_bytes].
// The body is itself an indefinite-length array (from ChequeBody's impl).
// =========================================================================
impl<C, T, V: VerifyState> minicbor::Encode<C> for ChequeSigned<T, V>
where
    T: minicbor::Encode<C>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.begin_array()?;
        e.encode_with(&self.body, ctx)?;
        e.bytes(self.signature.as_ref())?;
        e.end()?;
        Ok(())
    }
}

/// Decoding always produces `Unverified` — verification must be done explicitly.
impl<'b, C, T> minicbor::Decode<'b, C> for ChequeSigned<T, Unverified>
where
    T: minicbor::Decode<'b, C>,
{
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        let body: ChequeBody<T> = d.decode_with(ctx)?;
        let sig_raw = d.bytes()?;
        let sig_array: [u8; 64] = sig_raw
            .try_into()
            .map_err(|_| minicbor::decode::Error::message("signature must be exactly 64 bytes"))?;
        let signature = Signature::from(sig_array);
        if d.datatype()? != minicbor::data::Type::Break {
            return Err(minicbor::decode::Error::message(
                "expected end of ChequeSigned array",
            ));
        }
        d.skip()?;
        Ok(Self::new_with_state(body, signature))
    }
}

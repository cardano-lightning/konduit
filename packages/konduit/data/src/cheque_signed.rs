use crate::{Duration, Signature, Unverified, Verified, VerifyState, cheque_body::ChequeBody};
use std::marker::PhantomData;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(bound(
        serialize = "T: Serialize",
        deserialize = "T: for<'de2> Deserialize<'de2>, V: Default",
    ))
)]
pub struct ChequeSigned<T, V: VerifyState = Unverified> {
    pub body: ChequeBody<T>,
    pub signature: Signature,
    #[cfg_attr(feature = "serde", serde(skip))]
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
        e.encode_with(self.signature, ctx)?;
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
        let signature: Signature = d.decode_with(ctx)?;
        if d.datatype()? != minicbor::data::Type::Break {
            return Err(minicbor::decode::Error::message(
                "expected end of ChequeSigned array",
            ));
        }
        d.skip()?;
        Ok(Self::new_with_state(body, signature))
    }
}

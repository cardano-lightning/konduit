use crate::{Duration, Unverified, cheque_body::ChequeBody};
use cardano_sdk::Signature;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::marker::PhantomData;

#[serde_as]
#[cfg_attr(feature = "cddl", derive(cuddly::ToCddl))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(bound = "T: Serialize + for<'de2> Deserialize<'de2>")]
pub struct ChequeSigned<T, U = Unverified> {
    #[cfg_attr(feature = "cddl", n(0))]
    pub body: ChequeBody<T>,
    #[cfg_attr(feature = "cddl", n(1))]
    #[serde_as(as = "serde_with::hex::Hex")]
    #[cfg_attr(feature = "cddl", cddl(bytes))]
    pub signature: Signature,
    #[serde(skip)]
    #[cfg_attr(feature = "cddl", cddl(skip))]
    pub _marker: PhantomData<U>,
}

// =========================================================================
// Universal Methods (Available on both Verified and Unverified states)
// =========================================================================
impl<S, U> ChequeSigned<S, U> {
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
        &self.body.latch()
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
}

// =========================================================================
// Testing Utilities
// =========================================================================
#[cfg(feature = "proptest")]
impl proptest::arbitrary::Arbitrary for ChequeSigned<Unverified> {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;
    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        (any::<ChequeBody>(), any::<[u8; 64]>())
            .prop_map(|(body, sig_bytes)| ChequeSigned::new(body, Signature::from(sig_bytes)))
            .boxed()
    }
}

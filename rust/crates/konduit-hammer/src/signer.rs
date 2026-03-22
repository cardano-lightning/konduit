use cardano_sdk::{Hash, Signature, SigningKey, VerificationKey};
use std::{collections::HashMap, sync::Arc};

pub type SignerFn = Box<dyn Fn(Hash<32>) -> (Arc<VerificationKey>, Arc<Signature>) + Send + Sync>;

pub fn to_signer_fn(key: SigningKey) -> SignerFn {
    let key = Arc::new(key);
    Box::new(move |msg| (Arc::new(key.to_verification_key()), Arc::new(key.sign(msg))))
}

pub struct Signer(HashMap<Hash<28>, SignerFn>);

impl Signer {
    fn new(value: HashMap<Hash<28>, SignerFn>) -> Self {
        Self(value)
    }

    pub fn add<F, VK, Sig>(&mut self, vk_hash: Hash<28>, f: F)
    where
        F: Fn(Hash<32>) -> (VK, Sig) + Send + Sync + 'static,
        VK: Into<Arc<VerificationKey>> + 'static,
        Sig: Into<Arc<Signature>> + 'static,
    {
        let normalized_fn: SignerFn = Box::new(move |hash| {
            let (vk, sig) = f(hash);
            (vk.into(), sig.into())
        });

        self.0.insert(vk_hash, normalized_fn);
    }

    pub fn get(&self, vk: &Hash<28>) -> Option<&SignerFn> {
        self.0.get(vk)
    }
}

impl<I> From<I> for Signer
where
    I: IntoIterator<Item = SigningKey>,
{
    fn from(value: I) -> Self {
        Self(
            value
                .into_iter()
                .map(|key| {
                    (
                        Hash::<28>::new(key.to_verification_key()),
                        to_signer_fn(key),
                    )
                })
                .collect(),
        )
    }
}

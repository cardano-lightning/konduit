use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, Strategy};

use cryptoxide::ed25519::{self, PRIVATE_KEY_LENGTH, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH, keypair};

use konduit_data::{self, ChequeBody, Locked, Tag, Verified, VerifyingKey};

use crate::AikenFn;

pub type SigningKey = [u8; PRIVATE_KEY_LENGTH];
pub type Signature = [u8; SIGNATURE_LENGTH];

#[derive(Debug, Clone)]
pub struct KeyedLocked {
    key: SigningKey,
    tag: Tag,
    locked: Locked<Verified>,
}

impl Arbitrary for KeyedLocked {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        (any::<[u8; 32]>(), any::<Vec<u8>>(), any::<ChequeBody>())
            .prop_map(|(key, tag_bytes, body)| {
                let tag = Tag::from(tag_bytes[0..32].to_vec());
                let locked = konduit_data::Locked::make(&key.into(), &tag, body);
                KeyedLocked { key, tag, locked }
            })
            .boxed()
    }
}

impl KeyedLocked {
    fn verifying_key(&self) -> VerifyingKey {
        let vk: [u8; PUBLIC_KEY_LENGTH] = keypair(&self.key).1.into();
        VerifyingKey::from(vk)
    }
}

// TODO!

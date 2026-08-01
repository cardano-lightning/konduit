use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, Strategy, ValueTree};
use proptest::test_runner::TestRunner;

use cryptoxide::ed25519::{PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH, keypair};

use konduit_data::{
    self, Cheque, ChequeBody, Duration, Lock, Locked, Secret, Signature, Tag, Unlocked, Unverified,
    VerifyingKey,
};

use crate::AikenFn;

fn verifying_key(seed: &[u8; 32]) -> VerifyingKey {
    let vk: [u8; PUBLIC_KEY_LENGTH] = keypair(seed).1;
    VerifyingKey::from(vk)
}

#[derive(Debug, Clone)]
pub struct Verify {
    key: VerifyingKey,
    tag: Tag,
    cheque: Cheque<Unverified>,
}

impl Arbitrary for Verify {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        (
            any::<[u8; 32]>(),
            any::<Tag>(),
            any::<ChequeBody>(),
            any::<bool>(),
        )
            .prop_map(|(seed, tag, body, is_unlocked)| {
                let cheque = if is_unlocked {
                    // Pretend the lock's bytes are a secret; this is only
                    // for exercising the Unlocked wire shape structurally,
                    // not a real preimage.
                    let secret = Secret(body.lock().0);
                    let body = ChequeBody::new(body.index(), body.amount(), body.timeout(), secret);
                    Cheque::from(Unlocked::make(&seed.into(), &tag, body).into_unverified())
                } else {
                    Cheque::from(Locked::make(&seed.into(), &tag, body).into_unverified())
                };
                Verify {
                    key: verifying_key(&seed),
                    tag,
                    cheque,
                }
            })
            .boxed()
    }
}

impl Verify {
    fn fixed() -> Self {
        let mut runner = TestRunner::deterministic();
        Self::arbitrary()
            .new_tree(&mut runner)
            .expect("failed to generate a fixed Verify")
            .current()
    }

    pub fn corrupt(self) -> Self {
        let Self { key, tag, cheque } = self;
        let cheque = match cheque {
            Cheque::Unlocked(x) => {
                let body = ChequeBody::new(
                    x.index(),
                    x.amount().wrapping_add(1),
                    x.timeout(),
                    *x.secret(),
                );
                Cheque::Unlocked(Unlocked::new(body, *x.signature()))
            }
            Cheque::Locked(x) => {
                let body = ChequeBody::new(
                    x.index(),
                    x.amount().wrapping_add(1),
                    x.timeout(),
                    *x.lock(),
                );
                Cheque::Locked(Locked::new(body, *x.signature()))
            }
        };
        Self { key, tag, cheque }
    }
}

impl<C> minicbor::Encode<C> for Verify
where
    Cheque<Unverified>: minicbor::Encode<C>,
{
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.tag(minicbor::data::Tag::new(121))?;
        e.begin_array()?;
        e.encode_with(self.key, ctx)?;
        e.encode_with(&self.tag, ctx)?;
        e.encode_with(&self.cheque, ctx)?;
        e.end()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum Field {
    Key,
    Tag,
    Index,
    Amount,
    Timeout,
    Lock,
    Signature,
}

impl Arbitrary for Field {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;
    fn arbitrary_with(_args: ()) -> Self::Strategy {
        prop_oneof![
            Just(Field::Key),
            Just(Field::Tag),
            Just(Field::Index),
            Just(Field::Amount),
            Just(Field::Timeout),
            Just(Field::Lock),
            Just(Field::Signature),
        ]
        .boxed()
    }
}

impl Verify {
    #[allow(clippy::too_many_arguments)]
    fn corrupt_field(
        mut self,
        field: Field,
        alt_seed: [u8; 32],
        alt_tag: Tag,
        alt_index: u64,
        alt_amount: u64,
        alt_timeout: Duration,
        alt_lock: Lock,
        alt_sig_bytes: [u8; SIGNATURE_LENGTH],
    ) -> Self {
        match field {
            Field::Key => {
                self.key = verifying_key(&alt_seed);
                return self;
            }
            Field::Tag => {
                self.tag = alt_tag;
                return self;
            }
            _ => {}
        }

        self.cheque = match self.cheque {
            Cheque::Unlocked(x) => {
                let index = if matches!(field, Field::Index) {
                    alt_index
                } else {
                    x.index()
                };
                let amount = if matches!(field, Field::Amount) {
                    alt_amount
                } else {
                    x.amount()
                };
                let timeout = if matches!(field, Field::Timeout) {
                    alt_timeout
                } else {
                    x.timeout()
                };
                let secret = if matches!(field, Field::Lock) {
                    Secret(alt_lock.0)
                } else {
                    *x.secret()
                };
                let signature = if matches!(field, Field::Signature) {
                    Signature::from_bytes(alt_sig_bytes)
                } else {
                    *x.signature()
                };
                let body = ChequeBody::new(index, amount, timeout, secret);
                Cheque::Unlocked(Unlocked::new(body, signature))
            }
            Cheque::Locked(x) => {
                let index = if matches!(field, Field::Index) {
                    alt_index
                } else {
                    x.index()
                };
                let amount = if matches!(field, Field::Amount) {
                    alt_amount
                } else {
                    x.amount()
                };
                let timeout = if matches!(field, Field::Timeout) {
                    alt_timeout
                } else {
                    x.timeout()
                };
                let lock = if matches!(field, Field::Lock) {
                    alt_lock
                } else {
                    *x.lock()
                };
                let signature = if matches!(field, Field::Signature) {
                    Signature::from_bytes(alt_sig_bytes)
                } else {
                    *x.signature()
                };
                let body = ChequeBody::new(index, amount, timeout, lock);
                Cheque::Locked(Locked::new(body, signature))
            }
        };
        self
    }
}

fn cheque_verify_fn() -> AikenFn {
    AikenFn::from_shortcut("cheque/verify")
}

#[test]
fn cheque_def() {
    assert!(cheque_verify_fn().eval_true(&Verify::fixed()));
}

#[test]
fn cheque_corrupt() {
    assert!(cheque_verify_fn().eval_err(&Verify::fixed().corrupt()));
}

proptest! {
    #[test]
    fn prop_cheque_conforms(verify: Verify) {
        prop_assert!(cheque_verify_fn().eval_true(&verify));
    }

    #[test]
    fn prop_cheque_corrupt_fails(
        verify: Verify,
        field in any::<Field>(),
        alt_seed: [u8; 32],
        alt_tag: Tag,
        alt_index: u64,
        alt_amount: u64,
        alt_timeout: Duration,
        alt_lock: Lock,
        alt_sig_bytes: [u8; SIGNATURE_LENGTH],
    ) {
        let corrupted = verify.corrupt_field(
            field,
            alt_seed,
            alt_tag,
            alt_index,
            alt_amount,
            alt_timeout,
            alt_lock,
            alt_sig_bytes,
        );
        prop_assert!(cheque_verify_fn().eval_err(&corrupted));
    }
}

//! The konduit side of the outbound rail boundary.
//!
//! Konduit verifies inbound and owns its own fee and time margin; a rail
//! owns its network, its addressing and its settlement. This crate is the
//! whole of what the two say to each other: the constraints konduit
//! supplies, the terms a rail reports back, and nothing about HTTP.
//!
//! A rail is compiled in and selected by configuration at startup, which
//! is what [`Registry`] holds.

pub mod problem;
pub use problem::{PostProblem, PreProblem, QuoteProblem};

pub mod registry;
pub use registry::Registry;

use konduit_data::{Duration, Lock, Secret};
use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin};

/// The name no rail may take: it is konduit's own, for what no rail owns.
pub const RESERVED_NAME: &str = "konduit";

/// A duration forward from now, never a point in time.
///
/// `konduit_data::Duration` is used both ways across these crates, and
/// the boundary is where the two get mixed: every timeout here is
/// relative, and a rail converts back to absolute exactly once, when it
/// builds its own outbound commitment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Relative(pub Duration);

impl Relative {
    pub fn from_millis(millis: u64) -> Self {
        Self(Duration::from_millis(millis))
    }

    pub fn from_secs(secs: u64) -> Self {
        Self(Duration::from_secs(secs))
    }

    /// The absolute deadline this names, given the clock reading `now`.
    pub fn absolute(&self, now: Duration) -> Duration {
        now.saturating_add(self.0)
    }

    /// How long is left of an absolute `deadline`, zero once past it.
    pub fn until(deadline: Duration, now: Duration) -> Self {
        Self(deadline.saturating_sub(now))
    }

    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

impl std::fmt::Display for Relative {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// What konduit has verified it can claim, net of konduit's own fee and
/// time margin. A rail spends at most `amount`, resolves before
/// `timeout`, and commits against `lock`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Constraints {
    /// In the instance asset, net of konduit's fee.
    pub amount: u64,
    /// Net of konduit's time margin.
    pub timeout: Relative,
    pub lock: Lock,
}

/// Everything downstream of konduit: the rail's network's own cost, which
/// is not the rail's charge — a rail charges nothing of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quoted {
    pub fee: u64,
    pub timeout: Relative,
}

/// The determinate answer to a commit. There is no error arm: an error
/// read as a non-payment would chase a payment that was never sent, so a
/// rail internal fault is `Pending`, the only honest answer when it does
/// not know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Success(Secret),
    Pending,
    /// Nothing was committed outbound.
    FailedPre(PreProblem),
    FailedPost(PostProblem),
}

/// Narrower than [`Outcome`]: nothing fails before sending once sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    Success(Secret),
    Pending,
    /// A lock the rail never committed, or one whose cache has expired.
    /// Konduit reads this past the commitment timeout as failed, and
    /// before it as a rail fault.
    Unknown,
    Failed(PostProblem),
}

/// A rail's own instruction type, as it arrives over a content-negotiated
/// boundary. Both codecs, because the boundary serves both.
pub trait Instruction: for<'de> Deserialize<'de> + for<'b> minicbor::Decode<'b, ()> + Send {}

impl<T> Instruction for T where
    T: for<'de> Deserialize<'de> + for<'b> minicbor::Decode<'b, ()> + Send
{
}

/// An instruction still in the codec it arrived in: konduit does not
/// parse instructions, it carries them, and the variant says which codec
/// to read it back with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Encoded {
    Json(Vec<u8>),
    Cbor(Vec<u8>),
}

impl Encoded {
    /// Deserializes into a rail's own instruction type. The erasing layer
    /// calls this, so no rail hand rolls `bad-instruction`.
    pub fn decode<T: Instruction>(&self) -> Result<T, QuoteProblem> {
        match self {
            Self::Json(bytes) => serde_json::from_slice(bytes)
                .map_err(|err| QuoteProblem::BadInstruction(err.to_string())),
            Self::Cbor(bytes) => {
                minicbor::decode(bytes).map_err(|err| QuoteProblem::BadInstruction(err.to_string()))
            }
        }
    }
}

/// Konduit supplies the inbound constraints and the rail owns everything
/// after them, so constraints come first. Each operation takes the rail's
/// own instruction type: a rail quotes on less than it commits on, and konduit
/// names neither.
pub trait Rail: Send + Sync {
    /// Identifier, and the config key. Konduit serves it at `/pay/{NAME}/`.
    const NAME: &'static str;
    type QuoteInstruction: Instruction;
    type CommitInstruction: Instruction;

    /// Establish what the rail's network costs. Keeps an error, because a
    /// failed quote commits nothing.
    fn quote(
        &self,
        amount: u64,
        instruction: Self::QuoteInstruction,
    ) -> impl Future<Output = Result<Quoted, QuoteProblem>> + Send;

    /// Commit outbound within `inbound`. The lock is the idempotency key:
    /// repeated commits under one lock pay at most once.
    fn commit(
        &self,
        inbound: Constraints,
        instruction: Self::CommitInstruction,
    ) -> impl Future<Output = Outcome> + Send;

    /// Delayed resolution. A rail caches a revealed secret at least as
    /// long as the corresponding commitment timeout.
    fn reveal(&self, lock: &Lock) -> impl Future<Output = Resolution> + Send;
}

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// [`Rail`] with its associated types erased, so the registry can hold
/// one of each. Deserializing the instruction happens here, which is why
/// `bad-instruction` is nobody's hand rolled error.
pub trait DynRail: Send + Sync {
    fn name(&self) -> &'static str;

    fn quote<'a>(
        &'a self,
        amount: u64,
        encoded: &'a Encoded,
    ) -> BoxFuture<'a, Result<Quoted, QuoteProblem>>;

    fn commit<'a>(&'a self, inbound: Constraints, encoded: &'a Encoded) -> BoxFuture<'a, Outcome>;

    fn reveal<'a>(&'a self, lock: &'a Lock) -> BoxFuture<'a, Resolution>;
}

impl<R: Rail> DynRail for R {
    fn name(&self) -> &'static str {
        R::NAME
    }

    fn quote<'a>(
        &'a self,
        amount: u64,
        encoded: &'a Encoded,
    ) -> BoxFuture<'a, Result<Quoted, QuoteProblem>> {
        Box::pin(async move {
            let instruction = encoded.decode::<R::QuoteInstruction>()?;
            Rail::quote(self, amount, instruction).await
        })
    }

    fn commit<'a>(&'a self, inbound: Constraints, encoded: &'a Encoded) -> BoxFuture<'a, Outcome> {
        Box::pin(async move {
            match encoded.decode::<R::CommitInstruction>() {
                Ok(instruction) => Rail::commit(self, inbound, instruction).await,
                // Nothing is sent, so this is a pre-commitment failure.
                Err(problem) => Outcome::FailedPre(problem.into()),
            }
        })
    }

    fn reveal<'a>(&'a self, lock: &'a Lock) -> BoxFuture<'a, Resolution> {
        Box::pin(Rail::reveal(self, lock))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minicbor::{Decode, Encode};

    #[derive(Debug, Serialize, Deserialize, Encode, Decode)]
    struct Payee(#[n(0)] u8);

    struct Stub;

    impl Rail for Stub {
        const NAME: &'static str = "stub";
        type QuoteInstruction = Payee;
        type CommitInstruction = Payee;

        async fn quote(&self, amount: u64, _: Payee) -> Result<Quoted, QuoteProblem> {
            Ok(Quoted {
                fee: amount,
                timeout: Relative::from_secs(1),
            })
        }

        async fn commit(&self, _: Constraints, _: Payee) -> Outcome {
            Outcome::Pending
        }

        async fn reveal(&self, _: &Lock) -> Resolution {
            Resolution::Unknown
        }
    }

    struct Reserved;

    impl Rail for Reserved {
        const NAME: &'static str = RESERVED_NAME;
        type QuoteInstruction = Payee;
        type CommitInstruction = Payee;

        async fn quote(&self, _: u64, _: Payee) -> Result<Quoted, QuoteProblem> {
            unreachable!()
        }
        async fn commit(&self, _: Constraints, _: Payee) -> Outcome {
            unreachable!()
        }
        async fn reveal(&self, _: &Lock) -> Resolution {
            unreachable!()
        }
    }

    fn constraints() -> Constraints {
        Constraints {
            amount: 100,
            timeout: Relative::from_secs(10),
            lock: Lock([0; 32]),
        }
    }

    #[test]
    fn konduit_is_rejected_at_registration() {
        let mut registry = Registry::new();
        assert_eq!(registry.insert(Reserved), Err(registry::Error::Reserved));
        assert!(registry.is_empty());
    }

    #[test]
    fn one_rail_per_name() {
        let mut registry = Registry::new();
        assert!(registry.insert(Stub).is_ok());
        assert_eq!(
            registry.insert(Stub),
            Err(registry::Error::Duplicate("stub"))
        );
        assert_eq!(registry.names(), vec!["stub"]);
    }

    #[test]
    fn an_instruction_reaches_a_rail_in_either_codec() {
        let mut registry = Registry::new();
        registry.insert(Stub).unwrap();
        let rail = registry.get("stub").unwrap().clone();

        let json = Encoded::Json(serde_json::to_vec(&Payee(7)).unwrap());
        let mut buf = Vec::new();
        minicbor::encode(Payee(7), &mut buf).unwrap();
        let cbor = Encoded::Cbor(buf);

        for instruction in [json, cbor] {
            let quoted = futures_lite_block_on(rail.quote(42, &instruction));
            assert_eq!(quoted.unwrap().fee, 42);
        }
    }

    #[test]
    fn an_undecodable_instruction_is_a_pre_commitment_failure() {
        let mut registry = Registry::new();
        registry.insert(Stub).unwrap();
        let rail = registry.get("stub").unwrap().clone();
        let instruction = Encoded::Json(b"not a payee".to_vec());

        match futures_lite_block_on(rail.commit(constraints(), &instruction)) {
            Outcome::FailedPre(problem) => assert_eq!(problem.slug(), "bad-instruction"),
            other => panic!("expected FailedPre, got {other:?}"),
        }
    }

    /// Enough of an executor to poll a future that never yields, so the
    /// crate's tests need no runtime of their own.
    fn futures_lite_block_on<T>(future: impl Future<Output = T>) -> T {
        use std::task::{Context, Poll, Waker};
        let mut future = Box::pin(future);
        let mut cx = Context::from_waker(Waker::noop());
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(value) => value,
            Poll::Pending => panic!("stub rail futures never yield"),
        }
    }
}

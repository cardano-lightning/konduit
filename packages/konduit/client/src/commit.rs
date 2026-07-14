// ---------- Attempt ----------

use konduit_data::{Duration, Secret, Tag};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Attempt {
    /// Time attempt requested
    #[n(0)]
    at: Duration,
    /// Amount on the locked
    #[n(1)]
    amount: u64,
    /// Timeout on the locked, as a Duration since UNIX_EPOCH.
    #[n(2)]
    timeout: Duration,
    /// None if no result yet established.
    #[n(3)]
    outcome: Option<Outcome>,
}

impl Attempt {
    /// A brand-new attempt always starts with no outcome.
    pub fn new(at: Duration, amount: u64, timeout: Duration) -> Self {
        Self {
            at,
            amount,
            timeout,
            outcome: None,
        }
    }

    // accessors
    pub fn at(&self) -> Duration {
        self.at
    }
    pub fn amount(&self) -> u64 {
        self.amount
    }
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
    pub fn outcome(&self) -> Option<&Outcome> {
        self.outcome.as_ref()
    }

    pub fn is_pending(&self) -> bool {
        self.outcome.is_none()
    }

    pub fn is_ok(&self) -> bool {
        matches!(
            self.outcome,
            Some(Outcome {
                status: Status::Ok(_),
                ..
            })
        )
    }

    pub fn is_ko(&self) -> bool {
        matches!(
            self.outcome,
            Some(Outcome {
                status: Status::Ko(_),
                ..
            })
        )
    }

    /// Get the secret if this attempt resolved Ok.
    pub fn secret(&self) -> Option<&Secret> {
        self.outcome.as_ref().and_then(|o| o.status.secret())
    }

    /// Get the Ko reason if this attempt resolved Ko.
    pub fn ko(&self) -> Option<&Ko> {
        self.outcome.as_ref().and_then(|o| o.status.ko())
    }

    /// Settle this attempt as Ok. Allowed to overwrite a prior Ko — a
    /// late-arriving secret still proves redemption — but never
    /// overwrites an existing Ok.
    pub fn set_ok(&mut self, at: Duration, secret: Secret) -> Result<(), AttemptError> {
        if self.is_ok() {
            return Err(AttemptError::AlreadyOk);
        }
        self.outcome = Some(Outcome {
            at,
            status: Status::Ok(secret),
        });
        Ok(())
    }

    /// Mark this attempt as Ko. May overwrite a prior Ko (e.g. updated
    /// reason/timestamp) but never an existing Ok.
    pub fn set_ko(&mut self, at: Duration, ko: Ko) -> Result<(), AttemptError> {
        if self.is_ok() {
            return Err(AttemptError::AlreadyOk);
        }
        self.outcome = Some(Outcome {
            at,
            status: Status::Ko(ko),
        });
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AttemptError {
    #[error("attempt is already settled Ok; cannot be overwritten")]
    AlreadyOk,
}

// ---------- Outcome / Status ----------

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Outcome {
    /// Outcome established at:
    #[n(0)]
    at: Duration,
    /// result
    #[n(1)]
    status: Status,
}

impl Outcome {
    pub fn at(&self) -> Duration {
        self.at
    }
    pub fn status(&self) -> &Status {
        &self.status
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub enum Status {
    /// Pay Ok
    #[n(0)]
    Ok(#[n(0)] Secret),
    /// Pay Ko aka failed.
    #[n(1)]
    Ko(#[n(0)] Ko),
}

impl Status {
    pub fn is_ok(&self) -> bool {
        matches!(self, Status::Ok(_))
    }
    pub fn is_ko(&self) -> bool {
        matches!(self, Status::Ko(_))
    }

    pub fn secret(&self) -> Option<&Secret> {
        match self {
            Status::Ok(s) => Some(s),
            Status::Ko(_) => None,
        }
    }

    pub fn ko(&self) -> Option<&Ko> {
        match self {
            Status::Ko(k) => Some(k),
            Status::Ok(_) => None,
        }
    }
}

/// TODO: provide Ko types.
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub enum Ko {
    /// Any other reason reported with string
    #[n(0)]
    Any(#[n(0)] String),
}

// ---------- Commit ----------

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Commit {
    #[n(0)]
    pay_request: Vec<u8>,
    /// Index assigned
    #[n(2)]
    index: u64,
    #[n(3)]
    attempts: Vec<Attempt>,
}

impl Commit {
    /// A new commit always starts with exactly one attempt.
    pub fn new(
        pay_request: Vec<u8>,
        tag: Tag,
        index: u64,
        at: Duration,
        amount: u64,
        timeout: Duration,
    ) -> Self {
        Self {
            pay_request,
            tag,
            index,
            attempts: vec![Attempt::new(at, amount, timeout)],
        }
    }

    // accessors
    pub fn pay_request(&self) -> &[u8] {
        &self.pay_request
    }
    pub fn tag(&self) -> &Tag {
        &self.tag
    }
    pub fn index(&self) -> u64 {
        self.index
    }
    pub fn attempts(&self) -> &[Attempt] {
        &self.attempts
    }
    pub fn attempt_count(&self) -> usize {
        self.attempts.len()
    }

    /// Invariant "at least one attempt" holds for the lifetime of a Commit,
    /// as long as it was constructed via `new` or `decode_validated`.
    pub fn current_attempt(&self) -> &Attempt {
        self.attempts.last().expect("commit always has >=1 attempt")
    }

    pub fn current_attempt_mut(&mut self) -> &mut Attempt {
        self.attempts
            .last_mut()
            .expect("commit always has >=1 attempt")
    }

    /// True once any attempt has resolved Ok — at most one ever will.
    pub fn is_settled(&self) -> bool {
        self.attempts.iter().any(Attempt::is_ok)
    }

    pub fn settled_attempt(&self) -> Option<&Attempt> {
        self.attempts.iter().find(|a| a.is_ok())
    }

    /// Append a new attempt. Only allowed when the current attempt has
    /// been declared Ko and the commit isn't already settled.
    pub fn retry(
        &mut self,
        at: Duration,
        amount: u64,
        timeout: Duration,
    ) -> Result<&mut Attempt, CommitError> {
        if self.is_settled() {
            return Err(CommitError::AlreadySettled);
        }
        if !self.current_attempt().is_ko() {
            return Err(CommitError::NotKo);
        }
        self.attempts.push(Attempt::new(at, amount, timeout));
        Ok(self.attempts.last_mut().unwrap())
    }

    /// Settle the current (last) attempt as Ok. Targets the current
    /// attempt regardless of which physical attempt the secret
    /// actually belongs to — once retried, only the current attempt
    /// is reachable here.
    pub fn set_ok(&mut self, at: Duration, secret: Secret) -> Result<(), CommitError> {
        if self.is_settled() {
            return Err(CommitError::AlreadySettled);
        }
        self.current_attempt_mut().set_ok(at, secret)?;
        Ok(())
    }

    /// Mark the current (last) attempt as Ko.
    pub fn set_ko(&mut self, at: Duration, ko: Ko) -> Result<(), CommitError> {
        if self.is_settled() {
            return Err(CommitError::AlreadySettled);
        }
        self.current_attempt_mut().set_ko(at, ko)?;
        Ok(())
    }

    /// Structural invariants that the type itself can't enforce against
    /// arbitrary `Decode` input.
    pub fn validate(&self) -> Result<(), CommitError> {
        if self.attempts.is_empty() {
            return Err(CommitError::NoAttempts);
        }
        if self.attempts.iter().filter(|a| a.is_ok()).count() > 1 {
            return Err(CommitError::MultipleOk);
        }
        Ok(())
    }

    /// Decode a `Commit` from CBOR bytes and validate its structural
    /// invariants before handing it back. This is the single chokepoint
    /// untrusted bytes (wire, disk) should go through — callers that use
    /// this never need to remember to call `validate()` separately.
    pub fn decode_validated(bytes: &[u8]) -> Result<Self, CommitError> {
        let commit: Commit = minicbor::decode(bytes)?;
        commit.validate()?;
        Ok(commit)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CommitError {
    #[error("commit has no attempts")]
    NoAttempts,
    #[error("commit has more than one attempt settled Ok")]
    MultipleOk,
    #[error("current attempt has not been declared Ko; cannot retry")]
    NotKo,
    #[error("commit already settled with a successful attempt")]
    AlreadySettled,
    #[error(transparent)]
    Attempt(#[from] AttemptError),
    #[error(transparent)]
    Decode(#[from] minicbor::decode::Error),
}

use crate::server::mediation::MediaType;
use crate::{
    Channel, Margins, Paymes, admin,
    channel::{self, apply_locked, apply_secrets, apply_squash, apply_timeout},
    db, margins,
    time::{self, now},
};
use konduit_data::Squash;
use konduit_rail::{
    Encoded, Outcome, PostProblem, PreProblem, QuoteProblem, Quoted, RESERVED_NAME, Registry,
    Relative,
};
use konduit_tmp::{
    AdaptorInfo, Keytag, PayOutcome, PayRequest, Payme, PaymeRequest, Quote, QuoteRequest, Receipt,
    SquashProposal, SquashStatus, TxHelp,
};
/// Actix web server "Data" ie the context of handlers.
use std::sync::Arc;
use tokio::sync::RwLock;

/// An instruction is carried in whichever codec the request was in, so
/// the rail decodes it with that one.
fn encoded(codec: MediaType, instruction: Vec<u8>) -> Encoded {
    match codec {
        MediaType::Json => Encoded::Json(instruction),
        MediaType::Cbor => Encoded::Cbor(instruction),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// This should be impossible
    #[error("no time")]
    Time(#[from] time::Error),

    /// A problem raised by a rail, or by konduit itself. Slug and
    /// status come with it; this type adds nothing.
    #[error("{slug}: {detail}")]
    Problem {
        slug: &'static str,
        status: u16,
        detail: String,
    },

    #[error("no such rail: {0}")]
    NoRail(String),

    #[error("Missing middleware data")]
    Auth,

    #[error("No channel")]
    NoChannel,

    #[error("channel : {0}")]
    Channel(#[from] channel::Error),

    #[error("DB Contended")]
    DbContended,

    #[error("DB returned: {0}")]
    DbBackend(String),

    #[error("commitment: {0}")]
    Commitment(#[from] CommitmentError),

    #[error("Other")]
    Other,
}

impl From<QuoteProblem> for Error {
    fn from(problem: QuoteProblem) -> Self {
        Error::Problem {
            slug: problem.slug(),
            status: problem.status(),
            detail: problem.to_string(),
        }
    }
}

impl From<PreProblem> for Error {
    fn from(problem: PreProblem) -> Self {
        Error::Problem {
            slug: problem.slug(),
            status: problem.status(),
            detail: problem.to_string(),
        }
    }
}

impl From<PostProblem> for Error {
    fn from(problem: PostProblem) -> Self {
        Error::Problem {
            slug: problem.slug(),
            status: problem.status(),
            detail: problem.to_string(),
        }
    }
}

impl From<margins::Error> for Error {
    fn from(problem: margins::Error) -> Self {
        Error::Problem {
            slug: problem.slug(),
            status: problem.status(),
            detail: problem.to_string(),
        }
    }
}

impl From<db::Error> for Error {
    fn from(value: db::Error) -> Self {
        match value {
            db::Error::Contended => Error::DbContended,
            db::Error::Backend(error) => Error::DbBackend(error),
            db::Error::NoChannel => Error::NoChannel,
            db::Error::Channel(error) => Error::Channel(error),
        }
    }
}

pub struct Data {
    /// The rails this adaptor was configured with. Nothing here knows
    /// which they are.
    rails: Arc<Registry>,
    /// Konduit's own paymes, for a payment that ends here.
    paymes: Arc<Paymes>,
    db: Arc<db::Db>,
    fx: Arc<RwLock<fx_client::State>>,
    info: Arc<AdaptorInfo<TxHelp>>,
    /// Konduit's own fee and time margin, and the only place either is
    /// added or stripped.
    margins: Margins,
    admin: Arc<dyn admin::SyncApi + Send + Sync + 'static>,
}

impl Data {
    pub fn new(
        rails: Arc<Registry>,
        paymes: Arc<Paymes>,
        db: Arc<db::Db>,
        fx: Arc<RwLock<fx_client::State>>,
        info: Arc<AdaptorInfo<TxHelp>>,
        margins: Margins,
        admin: Arc<dyn admin::SyncApi + Send + Sync + 'static>,
    ) -> Self {
        Self {
            rails,
            paymes,
            db,
            fx,
            info,
            margins,
            admin,
        }
    }

    pub fn fx(&self) -> Arc<tokio::sync::RwLock<fx_client::State>> {
        self.fx.clone()
    }

    pub fn db(&self) -> Arc<db::Db> {
        self.db.clone()
    }

    pub fn rails(&self) -> Arc<Registry> {
        self.rails.clone()
    }

    pub fn admin(&self) -> Arc<dyn admin::SyncApi + Send + Sync + 'static> {
        self.admin.clone()
    }

    pub fn info(&self) -> Arc<AdaptorInfo<TxHelp>> {
        self.info.clone()
    }

    pub fn channel(&self, keytag: &Keytag) -> Result<Channel, Error> {
        self.db.get(keytag)?.ok_or(Error::NoChannel)
    }

    pub fn receipt(&self, keytag: &Keytag) -> Result<Option<Receipt>, Error> {
        Ok(self.channel(keytag)?.receipt().to_owned())
    }

    #[tracing::instrument(level = "debug", skip_all, err)]
    pub fn squash_proposal(&self, keytag: &Keytag) -> Result<SquashProposal, Error> {
        self.db().update(keytag, apply_timeout(now()?))?;
        Ok(self.channel(keytag)?.propose_squash()?)
    }

    // FIXME :: This is permissive against stale and bad squashes
    #[tracing::instrument(level = "debug", skip_all, err)]
    pub fn squash(&self, keytag: &Keytag, squash: Squash) -> Result<(), Error> {
        match self.db().update(keytag, apply_squash(squash)) {
            Ok(()) | Err(db::Error::Channel(channel::Error::Receipt(_))) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }

    /// Konduit's own payme, for a payment that ends here.
    #[tracing::instrument(skip_all, fields(amount = request.amount), err)]
    pub fn payme(&self, request: PaymeRequest) -> Result<Payme, Error> {
        let payme = self.paymes.insert(request.amount, now()?);
        // The lock isn't known until the payme is created; logged here
        // (rather than as an instrument field) so it can be matched
        // against the same lock in quote/pay spans, including across
        // separate terminals.
        tracing::info!(lock = %payme.lock, "payme created");
        Ok(Payme {
            payee: self.info.channel_parameters.adaptor_key.into(),
            amount: payme.amount,
            lock: payme.lock,
            timeout: payme.timeout,
        })
    }

    /// What it costs to be carried by `rail`: its terms, padded with
    /// konduit's. The reserved name has nothing downstream to ask.
    #[tracing::instrument(skip_all, fields(rail, amount = request.amount), err)]
    pub async fn quote(
        &self,
        keytag: &Keytag,
        rail: &str,
        request: QuoteRequest,
        codec: MediaType,
    ) -> Result<Quote, Error> {
        let channel = self.channel(keytag)?;
        let downstream = if rail == RESERVED_NAME {
            Quoted {
                fee: 0,
                timeout: Relative(konduit_data::Duration::from_millis(0)),
            }
        } else {
            // The amount is the rail's to read out of the instruction.
            // Konduit keeps its own copy for the arithmetic below, and
            // tells the rail nothing it did not instruct.
            let instruction = encoded(codec, request.instruction);
            self.rail(rail)?.quote(&instruction).await?
        };
        let offered = self.margins.pad(downstream);
        let index = channel.can_commit(request.amount + offered.fee)?;
        Ok(Quote {
            index,
            amount: request.amount + offered.fee,
            fee: offered.fee,
            relative_timeout: offered.timeout.0.as_millis() as u64,
        })
    }

    /// Verifies the inbound commitment, strips konduit's fee and margin
    /// off it, and hands what is left to `rail` with the consumer's
    /// instruction. The reserved name has no rail: the payment ends
    /// here, and konduit reveals its own secret.
    #[tracing::instrument(skip_all, fields(rail, lock = %request.inbound.lock()), err)]
    pub async fn pay(
        &self,
        keytag: &Keytag,
        rail: &str,
        request: PayRequest,
        codec: MediaType,
    ) -> Result<PayOutcome, Error> {
        // Konduit's alone: that the channel is backed, and that the
        // cheque is one it can claim.
        self.channel(keytag)?.assert_backed()?;
        // Konduit's fee and margin come off here and nowhere else. A
        // cheque that cannot cover them calls no rail — and is not
        // banked either: refusing before the receipt is what keeps a
        // cheque konduit will not honour out of it, which is cheaper
        // than taking it in and trying to drop it afterwards.
        let constraints = self.margins.strip(&request.inbound, now()?)?;
        self.db()
            .update(keytag, apply_locked(request.inbound.clone()))?;

        let outcome = if rail == RESERVED_NAME {
            // Konduit is the payee, and pays itself out of what is left
            // after its own fee — being the end of the route is not a
            // discount.
            match self
                .paymes
                .reveal(&constraints.lock, constraints.amount, now()?)
            {
                Ok(secret) => Outcome::Success(secret),
                Err(err) => {
                    Outcome::FailedPre(PreProblem::from(QuoteProblem::Unroutable(err.to_string())))
                }
            }
        } else {
            let instruction = encoded(codec, request.instruction);
            self.rail(rail)?.commit(constraints, &instruction).await
        };

        match outcome {
            Outcome::Success(secret) => {
                tracing::info!(secret = %hex::encode(secret.as_ref()), "secret revealed");
                self.db().update(keytag, apply_secrets(vec![secret]))?;
                Ok(PayOutcome::Success(secret))
            }
            // Committed and unresolved: the consumer polls squash
            // status, where the secret arrives as an unlocked cheque.
            Outcome::Pending => Ok(PayOutcome::Pending),
            // The inbound cheque is left live to elapse either way, and
            // dropping it on a pre-commitment failure is not konduit's
            // to do: `Receipt::apply_locked` takes cheques strictly in
            // index order, so a hole would refuse the consumer's next
            // one until the two resynced. The protocol already answers
            // this — the consumer retries under the same lock and
            // `apply_replace` takes the better cheque at the same index
            // — and anything never retried elapses on its own timeout.
            Outcome::FailedPre(problem) => Err(problem.into()),
            // Committed outbound, so the cheque is owed whatever the
            // peer says: it stays.
            Outcome::FailedPost(problem) => Err(problem.into()),
        }
    }

    fn rail(&self, name: &str) -> Result<&Arc<dyn konduit_rail::DynRail>, Error> {
        self.rails
            .get(name)
            .ok_or_else(|| Error::NoRail(name.to_string()))
    }

    // FIXME :: REMOVE THIS TEMPORARY PATCH!!
    #[tracing::instrument(level = "debug", skip_all, err)]
    pub fn squash_status(&self, keytag: &Keytag) -> Result<SquashStatus, Error> {
        let squash_proposal = self.squash_proposal(keytag)?;
        Ok(SquashStatus::Incomplete(squash_proposal))
    }
}

/// Konduit's own grounds for not committing. Note what is absent: the
/// lock. Konduit takes it from the cheque and compares it to nothing —
/// whether an instruction disagrees with it is the rail's question, and
/// its `FailedPre` to raise.
#[derive(Debug, thiserror::Error)]
pub enum CommitmentError {
    #[error("no or insufficient fee")]
    Fee,
    #[error("no or insufficient time")]
    Time,
}

use crate::{
    Channel, admin,
    channel::{self, apply_locked, apply_secrets, apply_squash, apply_timeout},
    db,
    time::{self, now},
};
use konduit_data::{Locked, Secret, Squash};
use konduit_tmp::{
    AdaptorInfo, Keytag, Payme, PaymeRequest, Quote, QuoteBody, Receipt, SquashProposal,
    SquashStatus, TxHelp,
};
/// Actix web server "Data" ie the context of handlers.
use std::sync::Arc;
use tokio::sync::RwLock;

// TODO :: MOVE TO CONFIG
const FEE_PLACEHOLDER: u64 = 1000;
const QUOTE_PAY_TIME_MARGIN_MS: u64 = 30_000;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// This should be impossible
    #[error("no time")]
    Time(#[from] time::Error),

    #[error("CLN: {0}")]
    Cln(String),

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
    cln: Arc<crate::cln::Node>,
    db: Arc<db::Db>,
    fx: Arc<RwLock<fx_client::State>>,
    info: Arc<AdaptorInfo<TxHelp>>,
    admin: Arc<dyn admin::SyncApi + Send + Sync + 'static>,
}

impl Data {
    pub fn new(
        cln: Arc<crate::cln::Node>,
        db: Arc<db::Db>,
        fx: Arc<RwLock<fx_client::State>>,
        info: Arc<AdaptorInfo<TxHelp>>,
        admin: Arc<dyn admin::SyncApi + Send + Sync + 'static>,
    ) -> Self {
        Self {
            cln,
            db,
            fx,
            info,
            admin,
        }
    }

    pub fn fx(&self) -> Arc<tokio::sync::RwLock<fx_client::State>> {
        self.fx.clone()
    }

    pub fn db(&self) -> Arc<db::Db> {
        self.db.clone()
    }

    pub fn cln(&self) -> Arc<crate::cln::Node> {
        self.cln.clone()
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

    pub fn squash_proposal(&self, keytag: &Keytag) -> Result<SquashProposal, Error> {
        self.db().update(keytag, apply_timeout(now()?))?;
        Ok(self.channel(keytag)?.propose_squash()?)
    }

    // FIXME :: This is permissive against stale and bad squashes
    pub fn squash(&self, keytag: &Keytag, squash: Squash) -> Result<(), Error> {
        match self.db().update(keytag, apply_squash(squash)) {
            Ok(()) | Err(db::Error::Channel(channel::Error::Receipt(_))) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }

    pub fn payme(&self, request: PaymeRequest) -> Result<Payme, Error> {
        let response = self
            .cln
            .payme(cln_server::wire::payme::Request {
                amount: request.amount,
            })
            .map_err(|error| Error::Cln(error.to_string()))?;
        Ok(Payme {
            payee: response.payee,
            amount: response.amount,
            lock: response.lock,
            timeout: response.timeout,
        })
    }

    pub async fn quote(&self, keytag: &Keytag, body: QuoteBody) -> Result<Quote, Error> {
        let channel = self.channel(keytag)?;
        if body.timeout < time::now()? {
            return Err(CommitmentError::Time.into());
        }
        let response = self
            .cln
            .quote_outbound(cln_server::wire::quote::Request {
                payee: body.payee,
                amount: body.amount,
            })
            .await
            .map_err(|error| Error::Cln(error.to_string()))?;
        let amount = body.amount + response.fee + FEE_PLACEHOLDER + 1;
        // Actual commitment
        let index = channel.can_commit(amount)?;
        let relative_timeout = response
            .timeout
            .as_millis()
            .saturating_add(QUOTE_PAY_TIME_MARGIN_MS as u128) as u64;
        Ok(Quote {
            index,
            amount,
            relative_timeout,
            routing_fee: response.fee,
        })
    }

    fn align_commitments(&self, locked: &Locked, payme: &Payme) -> Result<(), CommitmentError> {
        if payme.lock != *locked.lock() {
            return Err(CommitmentError::Lock);
        }
        if locked.amount() < payme.amount.saturating_add(FEE_PLACEHOLDER) {
            return Err(CommitmentError::Fee);
        }
        let current_time = time::now().map_err(|_| CommitmentError::Time)?;
        if payme.timeout < current_time {
            return Err(CommitmentError::Time);
        }
        Ok(())
    }

    pub async fn pay(&self, keytag: &Keytag, body: PayBody) -> Result<PayResponse, Error> {
        let PayBody { locked, payme } = body;
        self.align_commitments(&locked, &payme)?;
        self.db().update(keytag, apply_locked(locked.clone()))?;
        let outbound = (payme.payee != self.cln.verifying_key()).then_some(
            cln_server::wire::commit::Outbound {
                key: payme.payee,
                amount: payme.amount,
            },
        );
        let pay_res = self
            .cln
            .commit_outbound(cln_server::wire::commit::Request {
                inbound: locked,
                outbound,
            })
            .await
            .map_err(|error| Error::Cln(error.to_string()))?;
        if let Some(secret) = pay_res.secret {
            self.db().update(keytag, apply_secrets(vec![secret]))?;
        };
        Ok(PayResponse::from(pay_res.secret.map(|secret| secret.0)))
    }

    pub async fn cln_quote(
        &self,
        keytag: cln_server::wire::auth::Keytag,
        request: cln_server::wire::quote::Request,
    ) -> Result<cln_server::wire::quote::Response, Error> {
        self.cln
            .quote(&keytag, request)
            .await
            .map_err(|error| Error::Cln(error.to_string()))
    }

    pub async fn cln_commit(
        &self,
        keytag: cln_server::wire::auth::Keytag,
        request: cln_server::wire::commit::Request,
    ) -> Result<cln_server::wire::commit::Response, Error> {
        self.cln
            .commit(&keytag, request)
            .await
            .map_err(|error| Error::Cln(error.to_string()))
    }

    pub fn cln_sync(
        &self,
        keytag: cln_server::wire::auth::Keytag,
        request: cln_server::wire::sync::Request,
    ) -> Result<cln_server::wire::sync::Response, Error> {
        self.cln
            .sync(&keytag, request)
            .map_err(|error| Error::Cln(error.to_string()))
    }

    // FIXME :: REMOVE THIS TEMPORARY PATCH!!
    pub fn squash_status(&self, keytag: &Keytag) -> Result<SquashStatus, Error> {
        let squash_proposal = self.squash_proposal(keytag)?;
        Ok(SquashStatus::Incomplete(squash_proposal))
    }
}

// FIXME :: API IMPROVEMENT. SIMPLIFICATION.
// NEEDS TO BE DOWNSTREAMED.
pub struct PayBody {
    pub locked: Locked,
    pub payme: Payme,
}

#[derive()]
pub enum PayResponse {
    Ok(Secret),
    Pending,
}

impl From<Option<[u8; 32]>> for PayResponse {
    fn from(value: Option<[u8; 32]>) -> Self {
        value
            .map(Secret)
            .map_or(PayResponse::Pending, PayResponse::Ok)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CommitmentError {
    #[error("lock mismatch")]
    Lock,
    #[error("no or insufficient fee")]
    Fee,
    #[error("no or insufficient time")]
    Time,
}

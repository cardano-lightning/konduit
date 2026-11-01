use crate::{
    Channel, admin,
    channel::{self, apply_locked, apply_secrets, apply_squash, apply_timeout},
    db,
    time::{self, now},
};
use cln_rail::{receipt, wire::commit::Request};
use konduit_data::{Duration, Locked, Secret, Squash};
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
    cln: Arc<cln_rail::Ctx>,
    db: Arc<db::Db>,
    fx: Arc<RwLock<fx_client::State>>,
    info: Arc<AdaptorInfo<TxHelp>>,
    admin: Arc<dyn admin::SyncApi + Send + Sync + 'static>,
}

impl Data {
    pub fn new(
        cln: Arc<cln_rail::Ctx>,
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

    pub fn cln(&self) -> Arc<cln_rail::Ctx> {
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

    /// Confirms the Konduit channel backing a CLN peer's keytag is funded
    /// on-chain before handing a wire-level CLN request off to `self.cln`'s
    /// peer routing. CLN's own inbound bookkeeping has no way to check
    /// that itself — it just trusts whatever commits a keytag it already
    /// recognizes — so every `cln_*` handler must call this first.
    fn assert_cln_peer_backed(&self, keytag: &cln_rail::wire::auth::Keytag) -> Result<(), Error> {
        let keytag = Keytag::try_from(keytag.as_bytes().to_vec())
            .map_err(|error| Error::Cln(format!("invalid CLN keytag: {error}")))?;
        Ok(self.channel(&keytag)?.assert_backed()?)
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

    #[tracing::instrument(skip_all, fields(amount = request.amount), err)]
    pub fn payme(&self, request: PaymeRequest) -> Result<Payme, Error> {
        let response = self
            .cln
            .payme(cln_rail::wire::payme::Request {
                amount: request.amount,
            })
            .map_err(|error| Error::Cln(error.to_string()))?;
        // The lock isn't known until the payme is created; logged here
        // (rather than as an instrument field) so it can be matched
        // against the same lock in quote/pay/cln_commit spans, including
        // across the gateway's and merchant's separate terminals.
        tracing::info!(lock = %response.lock, "payme created");
        Ok(Payme {
            payee: response.payee,
            amount: response.amount,
            lock: response.lock,
            timeout: response.timeout,
        })
    }

    #[tracing::instrument(skip_all, fields(amount = body.amount, lock = %body.lock), err)]
    pub async fn quote(&self, keytag: &Keytag, body: QuoteBody) -> Result<Quote, Error> {
        todo!("Superceded:: remove")
        // let channel = self.channel(keytag)?;
        // if body.timeout < time::now()? {
        //     return Err(CommitmentError::Time.into());
        // }
        // let response = self
        //     .cln
        //     .quote_outbound(cln_rail::wire::quote::Request {
        //         payee: body.payee,
        //         amount: body.amount,
        //     })
        //     .await
        //     .map_err(|error| Error::Cln(error.to_string()))?;
        // let amount = body.amount + response.fee + FEE_PLACEHOLDER + 1;
        // // Actual commitment
        // let index = channel.can_commit(amount)?;
        // let relative_timeout = response
        //     .timeout
        //     .as_millis()
        //     .saturating_add(QUOTE_PAY_TIME_MARGIN_MS as u128) as u64;
        // Ok(Quote {
        //     index,
        //     amount,
        //     relative_timeout,
        //     routing_fee: response.fee,
        // })
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

    #[tracing::instrument(skip_all, fields(amount = body.payme.amount, lock = %body.payme.lock), err)]
    pub async fn pay(&self, keytag: &Keytag, body: PayBody) -> Result<PayResponse, Error> {
        todo!("Superceded! to be removed");
        // let PayBody { locked, payme } = body;
        // self.align_commitments(&locked, &payme)?;
        // self.db().update(keytag, apply_locked(locked.clone()))?;
        // let outbound = (payme.payee != self.cln.verifying_key()).then_some(
        //     cln_rail::wire::commit::Outbound {
        //         key: payme.payee,
        //         amount: payme.amount,
        //     },
        // );
        // let pay_res = self
        //     .cln
        //     .commit_outbound(cln_rail::wire::commit::Request {
        //         inbound: locked,
        //         outbound,
        //     })
        //     .await
        //     .map_err(|error| Error::Cln(error.to_string()))?;
        // if let Some(secret) = pay_res.secret {
        //     tracing::info!(secret = %hex::encode(secret.as_ref()), "secret revealed");
        //     self.db().update(keytag, apply_secrets(vec![secret]))?;
        // };
        // Ok(PayResponse::from(pay_res.secret.map(|secret| secret.0)))
    }

    #[tracing::instrument(skip_all, fields(amount = request.amount), err)]
    pub async fn cln_quote(
        &self,
        keytag: cln_rail::wire::auth::Keytag,
        request: cln_rail::wire::quote::Request,
    ) -> Result<cln_rail::wire::quote::Response, Error> {
        let channel = self.channel(&keytag.as_bytes().to_vec().try_into().unwrap())?;
        let (fee, timeout) = if request.payee == self.cln.verifying_key() {
            // I am the payee. No fee
            (0_u64, Duration::from_millis(0))
        } else {
            let response = self
                .cln
                .quote(request.clone())
                .await
                .map_err(|e| Error::Cln(e.to_string()))?;
            (response.fee + FEE_PLACEHOLDER, response.timeout)
        };
        // Inbound index
        let index = channel.can_commit(request.amount + fee)?;
        // ALWAYS NEED TIME TO SUB!
        let timeout = timeout
            .as_millis()
            .saturating_add(QUOTE_PAY_TIME_MARGIN_MS as u128) as u64;
        Ok(cln_rail::wire::quote::Response {
            index,
            fee,
            timeout: Duration::from_millis(timeout),
        })
    }

    #[tracing::instrument(
        skip_all,
        fields(outbound = request.outbound.is_some(), lock = %request.inbound.lock()),
        err
    )]
    pub async fn cln_commit(
        &self,
        keytag: cln_rail::wire::auth::Keytag,
        request: cln_rail::wire::commit::Request,
    ) -> Result<cln_rail::wire::commit::Response, Error> {
        let keytag = keytag.as_bytes().to_vec().try_into().unwrap();
        self.db()
            .update(&keytag, apply_locked(request.inbound.clone()))?;
        let timeout = Duration::from_millis(
            request
                .inbound
                .timeout()
                .as_millis()
                .saturating_sub(QUOTE_PAY_TIME_MARGIN_MS as u128) as u64,
        );
        let secret = if request.outbound.is_none()
            || request
                .outbound
                .as_ref()
                .is_some_and(|payme| payme.key == self.cln.verifying_key())
        {
            // ASSUME PAYEE. You own the secret.
            let secret = self
                .cln()
                .paymes_reveal(request.inbound.lock(), request.inbound.amount())
                .map_err(|e| Error::Cln(e.to_string()))?;
            Some(secret)
        } else {
            // Youre routing.
            let payee = request.outbound.ok_or(Error::Other)?.key;
            let amount = request
                .inbound
                .amount()
                .checked_sub(FEE_PLACEHOLDER)
                .ok_or(Error::Other)?;
            let response = self
                .cln()
                .commit2(&payee, amount, timeout, &request.inbound.lock())
                .await
                .map_err(|e| Error::Cln(e.to_string()))?;
            response.secret
        };
        if let Some(secret) = secret {
            tracing::info!(secret = %hex::encode(secret.as_ref()), "secret revealed");
            self.db().update(&keytag, apply_secrets(vec![secret]))?;
        }
        Ok(cln_rail::wire::commit::Response { secret })
    }

    #[tracing::instrument(level = "debug", skip_all, err)]
    pub fn cln_sync(
        &self,
        keytag: cln_rail::wire::auth::Keytag,
        request: cln_rail::wire::sync::Request,
    ) -> Result<cln_rail::wire::sync::Response, Error> {
        let keytag = keytag.as_bytes().to_vec().try_into().unwrap();
        self.db()
            .update(&keytag, apply_squash(request.receipt.squash))?;
        let channel = self.channel(&keytag)?;
        let receipt = channel.receipt().clone().ok_or(Error::Other)?;
        let squash = receipt.squash().clone().into_unverified();
        let cheques = receipt
            .cheques()
            .cloned()
            .map(|c| c.into_unverified())
            .collect();
        let receipt = cln_rail::wire::sync::Receipt { squash, cheques };
        Ok(cln_rail::wire::sync::Response { receipt })
    }

    // FIXME :: REMOVE THIS TEMPORARY PATCH!!
    #[tracing::instrument(level = "debug", skip_all, err)]
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

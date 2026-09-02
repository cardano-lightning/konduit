use crate::{
    Channel, admin,
    channel::{self, apply_locked, apply_secrets, apply_squash, apply_timeout},
    db,
    time::{self, now},
};
use bln_client::types::{Invoice, RouteHint};
use konduit_data::{AssetDefinition, Duration, Locked, Pricing, Secret, Squash};
use konduit_tmp::{
    AdaptorInfo, Keytag, Quote, QuoteBody, Receipt, SquashProposal, SquashStatus, TxHelp,
};
/// Actix web server "Data" ie the context of handlers.
use std::sync::Arc;
use tokio::sync::RwLock;

// TODO :: MOVE TO CONFIG
const FEE_PLACEHOLDER: u64 = 1000;
/// This is ~ the same as the default on bitcoin: default (apparently) is 40 blocks
const ADAPTOR_TIME_DELTA: std::time::Duration = std::time::Duration::from_secs(40 * 10 * 60);
/// Extra time between the "quoted" rel time and the time that might be allowed for in a
/// "pay". I don't know why this has to be so high.
/// LND fails for values much smaller than this.
const QUOTE_PAY_TIME_MARGIN: std::time::Duration = std::time::Duration::from_secs(4 * 10 * 60);

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// This should be impossible
    #[error("no time")]
    Time(#[from] time::Error),

    #[error("Bln: {0}")]
    Bln(String),

    #[error("Missing middleware data")]
    Auth,

    #[error("No channel")]
    NoChannel,

    #[error("channel : {0}")]
    Channel(#[from] channel::Error),

    #[error("FX: {0}")]
    Fx(String),

    #[error("DB Contended")]
    DbContended,

    #[error("DB returned: {0}")]
    DbBackend(String),

    #[error("lease is invalid")]
    LeaseInvalid,

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
            db::Error::AlreadyExists => Error::DbBackend("entry already exists".into()),
            db::Error::LeaseInvalid => Error::LeaseInvalid,
            db::Error::Channel(error) => Error::Channel(error),
        }
    }
}

// TODO :: handle the pre/post distinction
impl From<bln_client::Error> for Error {
    fn from(value: bln_client::Error) -> Self {
        Error::Bln(value.to_string())
    }
}

pub struct Data {
    bln: Arc<dyn bln_client::Api + Send + Sync>,
    db: Arc<db::Db>,
    fx: Arc<RwLock<fx_client::State>>,
    info: Arc<AdaptorInfo<TxHelp>>,
    admin: Arc<dyn admin::SyncApi + Send + Sync + 'static>,
}

impl Data {
    pub fn new(
        bln: Arc<dyn bln_client::Api + Send + Sync>,
        db: Arc<db::Db>,
        fx: Arc<RwLock<fx_client::State>>,
        info: Arc<AdaptorInfo<TxHelp>>,
        admin: Arc<dyn admin::SyncApi + Send + Sync + 'static>,
    ) -> Self {
        Self {
            bln,
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

    pub fn bln(&self) -> Arc<dyn bln_client::Api + Send + Sync + 'static> {
        self.bln.clone()
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
    pub fn squash(
        &self,
        keytag: &Keytag,
        lease_token: &[u8; 32],
        squash: Squash,
    ) -> Result<(), Error> {
        match self.db().update_with_lease(
            keytag,
            lease_token,
            time::now()?.as_millis() as u64,
            apply_squash(squash),
        ) {
            Ok(()) | Err(db::Error::Channel(channel::Error::Receipt(_))) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }

    async fn bln_quote(
        &self,
        amount_msat: u64,
        payee: [u8; 33],
        route_hints: Vec<RouteHint>,
    ) -> Result<bln_client::types::QuoteResponse, Error> {
        Ok(self
            .bln()
            .quote(bln_client::types::QuoteRequest {
                amount_msat,
                payee,
                route_hints,
            })
            .await?)
    }

    pub async fn quote(&self, keytag: &Keytag, body: QuoteBody) -> Result<Quote, Error> {
        let channel = self.channel(keytag)?;
        let definition = channel.asset_definition().clone();
        let amount_msat = body.amount_msat();
        let min_amount = {
            let fx = self.fx.read().await;
            quote_amount(&fx, &definition, amount_msat)
                .map_err(|error| Error::Fx(error.to_string()))?
        };
        channel.can_commit(min_amount)?;
        let bln_res = self
            .bln_quote(amount_msat, body.payee(), body.route_hints())
            .await?;
        let quote_msat = amount_msat
            .checked_add(bln_res.fee_msat)
            .ok_or_else(|| Error::Fx("quote amount exceeds u64".into()))?;
        let amount = {
            let fx = self.fx.read().await;
            quote_amount(&fx, &definition, quote_msat)
                .map_err(|error| Error::Fx(error.to_string()))?
        };
        let index = channel.can_commit(amount)?;
        let relative_timeout =
            (ADAPTOR_TIME_DELTA + QUOTE_PAY_TIME_MARGIN + bln_res.relative_timeout).as_millis()
                as u64;
        Ok(Quote {
            index,
            amount,
            relative_timeout,
            routing_fee: bln_res.fee_msat,
        })
    }

    async fn bln_pay(
        &self,
        invoice: Invoice,
        fee_limit: u64,
        rel_timeout: Duration,
    ) -> Result<bln_client::types::PayResponse, Error> {
        let pay_request = bln_client::types::PayRequest {
            fee_limit,
            relative_timeout: time::from_konduit_duration(rel_timeout),
            invoice,
        };
        // TODO :: handle pre-commitment failure case
        self.bln()
            .pay(pay_request)
            .await
            .map_err(|err| Error::Bln(err.to_string()))
    }

    async fn align_commitments(
        &self,
        definition: &AssetDefinition,
        now: Duration,
        locked: &Locked,
        invoice: &Invoice,
    ) -> Result<(u64, Duration), CommitmentError> {
        if invoice.payment_hash != locked.lock().0 {
            return Err(CommitmentError::Lock);
        }
        let effective_asset_amount = locked
            .amount()
            .checked_sub(FEE_PLACEHOLDER)
            .ok_or(CommitmentError::Fee)?;
        let effective_amount_msat = {
            let fx = self.fx.read().await;
            fx.asset_units_to_msat(
                effective_asset_amount,
                definition.decimals,
                asset_usd(&fx, definition)
                    .map_err(|error| CommitmentError::Fx(error.to_string()))?,
            )
            .map_err(|error| CommitmentError::Fx(error.to_string()))?
        };
        let fee_limit = effective_amount_msat.saturating_sub(invoice.amount_msat);
        if fee_limit < 1 {
            return Err(CommitmentError::Fee);
        }
        let relative_timeout = locked
            .timeout()
            .saturating_sub(now)
            .saturating_sub(time::to_konduit_duration(ADAPTOR_TIME_DELTA));
        if relative_timeout.as_secs() < 1 {
            return Err(CommitmentError::Time);
        }
        Ok((fee_limit, relative_timeout))
    }

    pub async fn pay(
        &self,
        keytag: &Keytag,
        lease_token: &[u8; 32],
        body: PayBody,
    ) -> Result<PayResponse, Error> {
        let definition = self.channel(keytag)?.asset_definition().clone();
        let PayBody { locked, invoice } = body;
        let (fee_limit, rel_timeout) = self
            .align_commitments(&definition, time::now()?, &locked, &invoice)
            .await?;
        self.db().update_with_lease(
            keytag,
            lease_token,
            time::now()?.as_millis() as u64,
            apply_locked(locked),
        )?;
        let pay_res = self.bln_pay(invoice, fee_limit, rel_timeout).await?;
        if let Some(secret) = pay_res.secret {
            self.db()
                .update(keytag, apply_secrets(vec![Secret(secret)]))?;
        };
        Ok(PayResponse::from(pay_res.secret))
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
    pub invoice: Invoice,
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

pub(super) fn asset_usd(
    fx: &fx_client::State,
    definition: &AssetDefinition,
) -> fx_client::Result<f64> {
    match &definition.pricing {
        Pricing::Ada => Ok(fx.ada),
        Pricing::UsdPeg => Ok(1.0),
        Pricing::CoinGecko { .. } => fx.asset_usd(&definition.alias),
    }
}

pub(super) fn quote_amount(
    fx: &fx_client::State,
    definition: &AssetDefinition,
    amount_msat: u64,
) -> fx_client::Result<u64> {
    fx.msat_to_asset_units(amount_msat, definition.decimals, asset_usd(fx, definition)?)?
        .checked_add(FEE_PLACEHOLDER)
        .and_then(|amount| amount.checked_add(1))
        .ok_or_else(|| fx_client::Error::InvalidData("quote amount exceeds u64".into()))
}

#[derive(Debug, thiserror::Error)]
pub enum CommitmentError {
    #[error("lock mismatch")]
    Lock,
    #[error("no or insufficient fee")]
    Fee,
    #[error("no or insufficient time")]
    Time,
    #[error("FX: {0}")]
    Fx(String),
}

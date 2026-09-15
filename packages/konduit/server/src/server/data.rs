use crate::{
    Channel, admin,
    channel::{self, apply_squash, apply_timeout},
    db,
    time::{self, now},
};
use bln_client::types::{Invoice, RouteHint};
use konduit_data::{AssetDefinition, Duration, Locked, Pricing, Secret, Squash};
use konduit_tmp::{
    AdaptorInfo, Keytag, Quote, QuoteBody, Receipt, SquashProposal, SquashStatus, TxHelp,
};
use sha2::{Digest, Sha256};
/// Actix web server "Data" ie the context of handlers.
use std::sync::Arc;
use tokio::sync::RwLock;

const FEE_PLACEHOLDER_MSAT: u64 = 1000;
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

    #[error("FX unavailable: {0}")]
    FxUnavailable(String),

    #[error("DB Contended")]
    DbContended,

    #[error("DB returned: {0}")]
    DbBackend(String),

    #[error("lease is invalid")]
    LeaseInvalid,
    #[error("operation id conflicts with an existing request")]
    OperationConflict,
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
            db::Error::OperationConflict => Error::OperationConflict,
            error @ (db::Error::PaymentNotFound
            | db::Error::PaymentAmbiguous
            | db::Error::PaymentAuthorizationInvalid) => Error::DbBackend(error.to_string()),
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
    channel_operations: Arc<dyn super::channel_operations::Api>,
}

impl Data {
    pub fn new(
        bln: Arc<dyn bln_client::Api + Send + Sync>,
        db: Arc<db::Db>,
        fx: Arc<RwLock<fx_client::State>>,
        info: Arc<AdaptorInfo<TxHelp>>,
        admin: Arc<dyn admin::SyncApi + Send + Sync + 'static>,
        channel_operations: Arc<dyn super::channel_operations::Api>,
    ) -> Self {
        Self {
            bln,
            db,
            fx,
            info,
            admin,
            channel_operations,
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

    pub fn channel_operations(&self) -> Arc<dyn super::channel_operations::Api> {
        self.channel_operations.clone()
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
        payment_request: Option<String>,
        final_cltv_delta: u64,
    ) -> Result<bln_client::types::QuoteResponse, Error> {
        Ok(self
            .bln()
            .quote(bln_client::types::QuoteRequest {
                amount_msat,
                payee,
                route_hints,
                payment_request,
                final_cltv_delta,
            })
            .await?)
    }

    pub async fn quote(&self, keytag: &Keytag, body: QuoteBody) -> Result<Quote, Error> {
        let channel = self.channel(keytag)?;
        let definition = channel.asset_definition().clone();
        let amount_msat = body.amount_msat();
        let invoice_hash = body.invoice_hash().map(hex::encode);
        let payment_request = body.payment_request();
        let final_cltv_delta = body.final_cltv_delta();
        let pricing = self.fx.read().await.clone();
        channel.can_commit(quote_amount(&pricing, &definition, amount_msat)?)?;
        let bln_res = self
            .bln_quote(
                amount_msat,
                body.payee(),
                body.route_hints(),
                payment_request,
                final_cltv_delta,
            )
            .await?;
        let quote_msat = amount_msat
            .checked_add(bln_res.fee_msat)
            .ok_or_else(|| Error::Fx("quote amount exceeds u64".into()))?;
        let converted_total = quote_amount(&pricing, &definition, quote_msat)?;
        let routing_fee_amount = quote_amount(&pricing, &definition, bln_res.fee_msat)?;
        let payment_amount = converted_total
            .checked_sub(routing_fee_amount)
            .ok_or_else(|| Error::Fx("quote fee exceeds total".into()))?;
        let adaptor_fee = self.info.tos.flat_fee;
        let amount = converted_total
            .checked_add(adaptor_fee)
            .ok_or_else(|| Error::Fx("quote amount exceeds u64".into()))?;
        let index = channel.can_commit(amount)?;
        let relative_timeout =
            (ADAPTOR_TIME_DELTA + QUOTE_PAY_TIME_MARGIN + bln_res.relative_timeout).as_millis()
                as u64;
        let expires_at_epoch_millis = (time::now()?.as_millis() as u64)
            .checked_add(relative_timeout)
            .ok_or(Error::Time(time::Error::NoTime))?;
        Ok(Quote {
            index,
            amount,
            relative_timeout,
            routing_fee: bln_res.fee_msat,
            invoice_hash,
            invoice_amount_msat: amount_msat,
            payment_amount,
            routing_fee_amount,
            adaptor_fee,
            expires_at_epoch_millis,
        })
    }

    async fn bln_pay(
        &self,
        invoice: Invoice,
        fee_limit: u64,
        rel_timeout: Duration,
    ) -> Result<bln_client::types::PayResponse, bln_client::Error> {
        self.bln()
            .pay(bln_client::types::PayRequest {
                fee_limit,
                relative_timeout: time::from_konduit_duration(rel_timeout),
                invoice,
            })
            .await
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
        let effective_amount_msat = {
            let fx = self.fx.read().await;
            let usd = asset_usd(&fx, definition)
                .map_err(|error| CommitmentError::Fx(error.to_string()))?;
            let fee = fx
                .msat_to_asset_units(FEE_PLACEHOLDER_MSAT, definition.decimals, usd)
                .map_err(|error| CommitmentError::Fx(error.to_string()))?;
            let effective_asset_amount = locked
                .amount()
                .checked_sub(fee)
                .ok_or(CommitmentError::Fee)?;
            fx.asset_units_to_msat(effective_asset_amount, definition.decimals, usd)
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
        let payment_hash = locked.lock().0;
        let payment_index = locked.index();
        let payment_lock = *locked.lock();
        let mut digest = Sha256::new();
        digest.update(minicbor::to_vec(&locked).map_err(|_| Error::Other)?);
        digest.update(invoice.to_string().as_bytes());
        let request_digest: [u8; 32] = digest.finalize().into();
        let identity = db::payment_identity(keytag, payment_index);
        let inserted = self.db().reserve_payment(
            &identity,
            &request_digest,
            &payment_hash,
            keytag,
            lease_token,
            time::now()?.as_millis() as u64,
            locked,
        )?;
        if !inserted {
            let secret = self
                .bln()
                .reveal(bln_client::types::RevealRequest { lock: payment_hash })
                .await?
                .secret;
            if let Some(secret) = secret {
                self.db().complete_payment(
                    &identity,
                    &request_digest,
                    &payment_hash,
                    keytag,
                    Secret(secret),
                )?;
            }
            return Ok(PayResponse::from(secret));
        }
        match self.bln_pay(invoice, fee_limit, rel_timeout).await {
            Ok(pay_res) => {
                if let Some(secret) = pay_res.secret {
                    self.db().complete_payment(
                        &identity,
                        &request_digest,
                        &payment_hash,
                        keytag,
                        Secret(secret),
                    )?;
                }
                Ok(PayResponse::from(pay_res.secret))
            }
            Err(error) if error.is_terminal_payment_failure() => {
                self.db().cancel_payment(
                    &identity,
                    &request_digest,
                    &payment_hash,
                    keytag,
                    payment_index,
                    &payment_lock,
                )?;
                Err(Error::Bln(format!("Payment failed: {error}")))
            }
            Err(error) => Err(error.into()),
        }
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
) -> Result<u64, Error> {
    let usd = asset_usd(fx, definition).map_err(fx_error)?;
    let units = fx
        .msat_to_asset_units(amount_msat, definition.decimals, usd)
        .map_err(fx_error)?;
    let fee = fx
        .msat_to_asset_units(FEE_PLACEHOLDER_MSAT, definition.decimals, usd)
        .map_err(fx_error)?;
    units
        .checked_add(fee)
        .and_then(|amount| amount.checked_add(1))
        .ok_or_else(|| Error::Fx("quote amount exceeds u64".into()))
}

fn fx_error(error: fx_client::Error) -> Error {
    match error {
        fx_client::Error::InvalidData(message)
            if message.contains("missing") || message.contains("price") =>
        {
            Error::FxUnavailable(message)
        }
        other => Error::Fx(other.to_string()),
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
    #[error("FX: {0}")]
    Fx(String),
}

use crate::db;
use crate::server::{
    self,
    auth::{AuthKeytag, LeaseToken},
    data,
    mediation::{self, Mediate, Mediation, Unmediate},
};
use actix_web::{HttpResponse, ResponseError, http::StatusCode, web};
use konduit_data::Locked;
use konduit_tmp::{
    AdaptorInfo, Quote, Receipt, SessionClaimRequest, SessionClaimResponse, SquashProposal,
    SquashStatus, TxHelp,
};
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use std::{
    ops::Deref,
    time::{SystemTime, UNIX_EPOCH},
};

type Data = web::Data<server::Data>;

const SESSION_TIMESTAMP_SKEW_MILLIS: u64 = 5 * 60 * 1000;
const SESSION_LEASE_MILLIS: u64 = 2 * 60 * 1000;

fn session_timestamp_valid(timestamp: u64, now: u64) -> bool {
    timestamp.abs_diff(now) <= SESSION_TIMESTAMP_SKEW_MILLIS
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("mediation: {0}")]
    Mediation(#[from] mediation::Error),
    #[error("data: {0}")]
    Data(#[from] data::Error),
    #[error("session timestamp outside allowed skew")]
    InvalidSessionTimestamp,
    #[error("invalid session signature")]
    InvalidSessionSignature,
    #[error("session claim conflicts with active generation")]
    SessionConflict,
    #[error("invalid channel operation")]
    InvalidChannelOperation,
    #[error("operation id conflicts with an existing request")]
    OperationConflict,
    #[error("channel connector unavailable")]
    ChannelConnectorUnavailable,
    #[error("other")]
    Other,
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Error::Mediation(mediation::Error::Unmediate(_)) => StatusCode::BAD_REQUEST,
            Error::Mediation(mediation::Error::Backend(_)) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Data(data::Error::NoChannel) => StatusCode::NOT_FOUND,
            Error::Data(data::Error::LeaseInvalid) => StatusCode::UNAUTHORIZED,
            Error::Data(data::Error::DbContended) => StatusCode::SERVICE_UNAVAILABLE,
            Error::Data(data::Error::FxUnavailable(_)) => StatusCode::SERVICE_UNAVAILABLE,
            Error::Data(data::Error::DbBackend(_)) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Data(_) => StatusCode::BAD_REQUEST,
            Error::InvalidSessionTimestamp => StatusCode::BAD_REQUEST,
            Error::InvalidSessionSignature => StatusCode::UNAUTHORIZED,
            Error::SessionConflict => StatusCode::CONFLICT,
            Error::InvalidChannelOperation => StatusCode::BAD_REQUEST,
            Error::OperationConflict => StatusCode::CONFLICT,
            Error::ChannelConnectorUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            Error::Other => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status = self.status_code();
        if status.is_server_error() {
            // don't leak internal details to the client, but keep them for yourself
            log::error!("request failed: {self}");
            HttpResponse::build(status).body("internal server error")
        } else {
            HttpResponse::build(status).body(self.to_string())
        }
    }
}

pub async fn info(mediation: Mediation, data: Data) -> Result<Mediate<AdaptorInfo<TxHelp>>, Error> {
    Ok(Mediate(mediation.accept, data.info().deref().clone()))
}

pub async fn claim_session(
    data: Data,
    claim: web::Json<SessionClaimRequest>,
) -> Result<HttpResponse, Error> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::Other)?
        .as_millis() as u64;
    if !session_timestamp_valid(claim.timestamp, now) {
        return Err(Error::InvalidSessionTimestamp);
    }
    let expected_adaptor: [u8; 32] = data.info().channel_parameters.adaptor_key.into();
    if claim.adaptor_verification_key_hex != expected_adaptor || !claim.verify() {
        return Err(Error::InvalidSessionSignature);
    }

    let mut token = [0; 32];
    OsRng.fill_bytes(&mut token);
    let expires_at_epoch_millis = now + SESSION_LEASE_MILLIS;
    match data
        .db()
        .claim_lease(&claim, token, expires_at_epoch_millis)
    {
        Ok((token, expires_at_epoch_millis)) => Ok(HttpResponse::Ok().json(SessionClaimResponse {
            lease: hex::encode(token),
            expires_at_epoch_millis,
        })),
        Err(db::LeaseClaimError::Conflict) => Err(Error::SessionConflict),
        Err(db::LeaseClaimError::Database(error)) => Err(Error::Data(error.into())),
    }
}

pub async fn submit_channel_operation(
    keytag: AuthKeytag,
    lease: LeaseToken,
    data: Data,
    request: web::Json<server::channel_operations::Request>,
) -> Result<HttpResponse, Error> {
    let operation_id = parse_operation_id(&request.operation_id)?;
    let expected_transaction_id = parse_hash(&request.expected_transaction_id)?;
    let transaction = parse_transaction(&request.transaction)?;
    let transaction_digest = Sha256::digest(&transaction).into();
    let now = epoch_millis()?;
    let local = data
        .db()
        .reserve_channel_operation(
            &operation_id,
            &keytag,
            &lease.0,
            now,
            expected_transaction_id,
            transaction_digest,
            transaction,
        )
        .map_err(map_operation_db_error)?;
    if local.status != "reserved" {
        return Ok(HttpResponse::Ok().json(operation_response(&request.operation_id, local)));
    }
    let remote = data
        .channel_operations()
        .submit(&request)
        .await
        .map_err(|error| {
            log::error!("channel connector submit failed: {error:#}");
            Error::ChannelConnectorUnavailable
        })?;
    validate_remote_operation(&request, &remote)?;
    let updated = data
        .db()
        .update_channel_operation(
            &operation_id,
            &keytag,
            remote.status,
            remote
                .transaction_id
                .as_deref()
                .map(parse_hash)
                .transpose()?,
            remote.depth,
        )
        .map_err(map_operation_db_error)?;
    Ok(HttpResponse::Ok().json(operation_response(&request.operation_id, updated)))
}

pub async fn channel_operation(
    keytag: AuthKeytag,
    data: Data,
    operation_id: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let operation_key = parse_operation_id(&operation_id)?;
    let local = data
        .db()
        .channel_operation(&operation_key, &keytag)
        .map_err(map_operation_db_error)?
        .ok_or(Error::Data(data::Error::NoChannel))?;
    if matches!(local.status.as_str(), "settled" | "rejected") {
        return Ok(HttpResponse::Ok().json(operation_response(&operation_id, local)));
    }
    let remote = data
        .channel_operations()
        .lookup(&operation_id)
        .await
        .map_err(|error| {
            log::error!("channel connector lookup failed: {error:#}");
            Error::ChannelConnectorUnavailable
        })?;
    if remote.operation_id != *operation_id
        || remote.expected_transaction_id != hex::encode(local.expected_transaction_id)
    {
        return Err(Error::InvalidChannelOperation);
    }
    let updated = data
        .db()
        .update_channel_operation(
            &operation_key,
            &keytag,
            remote.status,
            remote
                .transaction_id
                .as_deref()
                .map(parse_hash)
                .transpose()?,
            remote.depth,
        )
        .map_err(map_operation_db_error)?;
    Ok(HttpResponse::Ok().json(operation_response(&operation_id, updated)))
}

fn operation_response(
    operation_id: &str,
    operation: db::ChannelOperationValue,
) -> server::channel_operations::Response {
    server::channel_operations::Response {
        operation_id: operation_id.to_owned(),
        expected_transaction_id: hex::encode(operation.expected_transaction_id),
        transaction_id: operation.transaction_id.map(hex::encode),
        status: if operation.status == "reserved" {
            "pending".into()
        } else {
            operation.status
        },
        depth: operation.depth,
    }
}

fn validate_remote_operation(
    request: &server::channel_operations::Request,
    response: &server::channel_operations::Response,
) -> Result<(), Error> {
    if response.operation_id != request.operation_id
        || response.expected_transaction_id != request.expected_transaction_id
    {
        return Err(Error::InvalidChannelOperation);
    }
    Ok(())
}

fn map_operation_db_error(error: db::Error) -> Error {
    match error {
        db::Error::OperationConflict => Error::OperationConflict,
        other => Error::Data(other.into()),
    }
}

fn epoch_millis() -> Result<u64, Error> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::Other)?
        .as_millis() as u64)
}

fn parse_operation_id(value: &str) -> Result<[u8; 16], Error> {
    if value.len() != 36
        || ![8, 13, 18, 23]
            .into_iter()
            .all(|index| value.as_bytes()[index] == b'-')
    {
        return Err(Error::InvalidChannelOperation);
    }
    let compact = value.replace('-', "");
    let mut result = [0; 16];
    hex::decode_to_slice(compact, &mut result).map_err(|_| Error::InvalidChannelOperation)?;
    Ok(result)
}

fn parse_hash(value: &str) -> Result<[u8; 32], Error> {
    if value.len() != 64
        || value
            .bytes()
            .any(|byte| !byte.is_ascii_digit() && !(b'a'..=b'f').contains(&byte))
    {
        return Err(Error::InvalidChannelOperation);
    }
    let mut result = [0; 32];
    hex::decode_to_slice(value, &mut result).map_err(|_| Error::InvalidChannelOperation)?;
    Ok(result)
}

fn parse_transaction(value: &str) -> Result<Vec<u8>, Error> {
    if value.len() > 512 * 1024 || value.len() % 2 != 0 {
        return Err(Error::InvalidChannelOperation);
    }
    hex::decode(value).map_err(|_| Error::InvalidChannelOperation)
}

pub async fn fx(mediation: Mediation, data: Data) -> Result<Mediate<fx_client::State>, Error> {
    Ok(Mediate(mediation.accept, data.fx().read().await.clone()))
}

pub async fn show(_data: Data) -> Result<HttpResponse, Error> {
    todo!()
    // log::info!("SHOW");
    // let keys = data.db().keys()?;
    // let results = keys
    //     .iter()
    //     .map(|x| data.db().get(x))
    //     .collect::<Result<Vec<_>, _>>()?;

    // Ok(HttpResponse::Ok().json(results))
}

/// Retrieve the latest receipt from the adaptor standpoint. This can be used by the consumer
/// to recover its own state without "fear":
///
/// - the squash is signed by their key, so necessarily originated from them.
/// - the adaptor is free to send an earlier receipt, which is only to the advantage of the
///   consumer for they will owe the adaptor *less* money. In practice, the adaptor has no
///   incentives to do that.
pub async fn receipt(
    mediation: Mediation,
    keytag: AuthKeytag,
    data: Data,
) -> Result<Mediate<Option<Receipt>>, Error> {
    Ok(Mediate(mediation.accept, data.receipt(&keytag)?))
}

pub async fn squash_proposal(
    mediation: Mediation,
    keytag: AuthKeytag,
    data: Data,
) -> Result<Mediate<SquashProposal>, Error> {
    Ok(Mediate(mediation.accept, data.squash_proposal(&keytag)?))
}

pub async fn squash_status(
    mediation: Mediation,
    keytag: AuthKeytag,
    data: Data,
) -> Result<Mediate<SquashStatus>, Error> {
    Ok(Mediate(mediation.accept, data.squash_status(&keytag)?))
}

pub async fn squash(
    mediation: Mediation,
    keytag: AuthKeytag,
    lease: LeaseToken,
    data: Data,
    body: web::Bytes,
) -> Result<Mediate<SquashStatus>, Error> {
    let _: Result<_, Error> = Ok(Mediate(
        mediation.accept,
        data.squash(
            &keytag,
            &lease.0,
            Unmediate::unmediate(mediation.content, &body)?,
        )?,
    ));
    squash_status(mediation, keytag, data).await
}

pub async fn quote(
    mediation: Mediation,
    keytag: AuthKeytag,
    data: Data,
    body: web::Bytes,
) -> Result<Mediate<Quote>, Error> {
    Ok(Mediate(
        mediation.accept,
        data.quote(&keytag, Unmediate::unmediate(mediation.content, &body)?)
            .await?,
    ))
}

// FIXME :: Remove the glue required here for historical reasons
pub async fn pay(
    mediation: Mediation,
    keytag: AuthKeytag,
    lease: LeaseToken,
    data: Data,
    body: web::Bytes,
) -> Result<Mediate<SquashStatus>, Error> {
    let b = konduit_tmp::PayBody::unmediate(mediation.content, &body)?;
    let locked = Locked::new(b.cheque_body, b.signature);
    let body = data::PayBody {
        locked,
        invoice: b.invoice,
    };
    let _ = data.pay(&keytag, &lease.0, body).await?;
    squash_status(mediation, keytag, data).await
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use konduit_data::{AssetCatalog, AssetDefinition, AssetId, Duration, Pricing, Squash};
    use konduit_tmp::Keytag;

    use super::*;

    fn fx() -> fx_client::State {
        fx_client::State::new(
            fx_client::BaseCurrency::Usd,
            0.5,
            100_000.0,
            BTreeMap::from([("custom".into(), 2.0)]),
        )
    }

    #[test]
    fn session_timestamp_allows_clock_skew_in_both_directions() {
        let now = 1_000_000;
        assert!(session_timestamp_valid(
            now - SESSION_TIMESTAMP_SKEW_MILLIS,
            now
        ));
        assert!(session_timestamp_valid(
            now + SESSION_TIMESTAMP_SKEW_MILLIS,
            now
        ));
        assert!(!session_timestamp_valid(
            now - SESSION_TIMESTAMP_SKEW_MILLIS - 1,
            now
        ));
        assert!(!session_timestamp_valid(
            now + SESSION_TIMESTAMP_SKEW_MILLIS + 1,
            now
        ));
    }

    #[test]
    fn quote_amount_uses_persisted_definition_pricing() {
        let fx = fx();
        let catalog = AssetCatalog::builtins();
        for alias in ["usdm", "usdcx", "usda"] {
            assert_eq!(
                data::quote_amount(&fx, catalog.by_alias(alias).unwrap(), 100_001_000).unwrap(),
                100_002_001
            );
        }
        let custom = AssetDefinition {
            alias: "custom".into(),
            asset: AssetId::native([1; 28], b"CUSTOM".to_vec()).unwrap(),
            decimals: 6,
            pricing: Pricing::CoinGecko {
                coin_id: "custom".into(),
            },
        };
        assert_eq!(
            data::quote_amount(&fx, &custom, 100_001_000).unwrap(),
            50_001_001
        );
        assert_eq!(
            fx.asset_units_to_msat(50_000_000, 6, data::asset_usd(&fx, &custom).unwrap(),)
                .unwrap(),
            100_000_000
        );

        let mut missing = fx;
        missing.assets.clear();
        assert!(data::quote_amount(&missing, &custom, 100_000_000).is_err());
    }

    struct PanicAdmin;

    #[async_trait::async_trait(?Send)]
    impl crate::admin::SyncApi for PanicAdmin {
        async fn sync(&self) -> anyhow::Result<()> {
            panic!("unexpected admin sync")
        }
    }

    struct PanicChannelOperations;

    #[async_trait::async_trait]
    impl crate::server::channel_operations::Api for PanicChannelOperations {
        async fn submit(
            &self,
            _request: &crate::server::channel_operations::Request,
        ) -> anyhow::Result<crate::server::channel_operations::Response> {
            panic!("unexpected channel submit")
        }

        async fn lookup(
            &self,
            _operation_id: &str,
        ) -> anyhow::Result<crate::server::channel_operations::Response> {
            panic!("unexpected channel lookup")
        }
    }

    fn handler_data(definition: AssetDefinition) -> (web::Data<server::Data>, Keytag) {
        use cardano_sdk::{Address, Credential, Hash, Network, VerificationKey};
        use konduit_data::{SigningKey, Squash, SquashBody};
        use konduit_tmp::{AdaptorInfo, ChannelParameters, TosInfo, TxHelp};

        let file = tempfile::NamedTempFile::new().unwrap();
        let db = std::sync::Arc::new(db::Db::open(file.path().to_str().unwrap()).unwrap());
        let signing = SigningKey::from_bytes([7; 32]);
        let tag = konduit_data::Tag::from(b"quote-smoke".as_slice());
        let mut channel =
            crate::channel::Channel::new(signing.verifying_key(), tag.clone(), definition);
        channel
            .apply_retainer(vec![crate::channel::Retainer {
                amount: 200_000_000,
                subbed: 0,
                useds: vec![],
            }])
            .unwrap();
        channel
            .apply_squash(Squash::make(&signing, &tag, SquashBody::zero()).into_unverified())
            .unwrap();
        let keytag = channel.keytag();
        db.insert(channel).unwrap();

        let payment = Credential::from_key(Hash::<28>::from([1; 28]));
        let host_address = Address::new(Network::Preview.into(), payment);
        let info = AdaptorInfo {
            tos: TosInfo { flat_fee: 0 },
            channel_parameters: ChannelParameters {
                adaptor_key: VerificationKey::from([2; 32]),
                close_period: Duration::from_secs(60),
                tag_length: 32,
            },
            tx_help: TxHelp {
                host_address,
                validator: konduit_tx::KONDUIT_VALIDATOR.hash,
            },
            asset_catalog_digest: None,
        };
        let data = server::Data::new(
            std::sync::Arc::new(bln_client::mock::Client::new()),
            db,
            std::sync::Arc::new(tokio::sync::RwLock::new(fx())),
            std::sync::Arc::new(info),
            std::sync::Arc::new(PanicAdmin),
            std::sync::Arc::new(PanicChannelOperations),
        );
        (web::Data::new(data), keytag)
    }

    #[actix_web::test]
    async fn squash_unknown_channel_returns_not_found_without_sync() {
        use konduit_data::{SigningKey, SquashBody, Tag};

        let (data, _) = handler_data(AssetCatalog::builtins().by_alias("usdm").unwrap().clone());
        let signing = SigningKey::from_bytes([8; 32]);
        let tag = Tag::from(b"unknown-squash".as_slice());
        let keytag = Keytag::new(
            &konduit_tmp::from_verifying_key(signing.verifying_key()),
            &tag,
        );
        let unknown_squash = Squash::make(&signing, &tag, SquashBody::zero()).into_unverified();
        let error = squash(
            Mediation {
                content: mediation::MediaType::Json,
                accept: mediation::MediaType::Json,
            },
            AuthKeytag(keytag.clone()),
            LeaseToken([0; 32]),
            data.clone(),
            web::Bytes::from(serde_json::to_vec(&unknown_squash).unwrap()),
        )
        .await
        .err()
        .unwrap();

        assert_eq!(error.status_code(), StatusCode::NOT_FOUND);
        assert!(data.db().get(&keytag).unwrap().is_none());
    }

    #[actix_web::test]
    async fn quote_handler_uses_authenticated_channel_definition() {
        for (definition, expected) in [
            (
                AssetCatalog::builtins().by_alias("usdm").unwrap().clone(),
                100_003_001,
            ),
            (
                AssetDefinition {
                    alias: "custom".into(),
                    asset: AssetId::native([1; 28], b"CUSTOM".to_vec()).unwrap(),
                    decimals: 6,
                    pricing: Pricing::CoinGecko {
                        coin_id: "custom".into(),
                    },
                },
                50_001_501,
            ),
        ] {
            let (data, keytag) = handler_data(definition);
            let body = serde_json::json!({
                "Simple": {
                    "amount_msat": 100_001_000_u64,
                    "payee": hex::encode([2_u8; 33]),
                    "route_hints": [],
                }
            });
            let response = quote(
                Mediation {
                    content: mediation::MediaType::Json,
                    accept: mediation::MediaType::Json,
                },
                AuthKeytag(keytag),
                data,
                web::Bytes::from(serde_json::to_vec(&body).unwrap()),
            )
            .await
            .unwrap();
            assert_eq!(response.1.amount, expected);
            assert_eq!(
                response.1.amount,
                response.1.payment_amount + response.1.routing_fee_amount + response.1.adaptor_fee
            );
            assert_eq!(response.1.invoice_amount_msat, 100_001_000);
            assert_eq!(response.1.invoice_hash, None);
        }
    }
}

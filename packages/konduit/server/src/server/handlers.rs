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
use std::{
    ops::Deref,
    time::{SystemTime, UNIX_EPOCH},
};

type Data = web::Data<server::Data>;

const SESSION_TIMESTAMP_SKEW_MILLIS: u64 = 5 * 60 * 1000;
const SESSION_LEASE_MILLIS: u64 = 2 * 60 * 1000;

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
    if claim.timestamp > now || now - claim.timestamp > SESSION_TIMESTAMP_SKEW_MILLIS {
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
        Err(db::LeaseClaimError::UnknownWallet) => Err(Error::Data(data::Error::NoChannel)),
        Err(db::LeaseClaimError::Database(error)) => Err(Error::Data(error.into())),
    }
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
        }
    }
}

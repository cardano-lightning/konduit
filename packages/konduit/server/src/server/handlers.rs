use crate::server::{
    self,
    auth::AuthKeytag,
    data,
    mediation::{self, Mediate, Mediation, Unmediate},
};
use actix_web::{HttpResponse, ResponseError, http::StatusCode, web};
use konduit_data::Locked;
use konduit_tmp::{AdaptorInfo, Quote, Receipt, SquashProposal, SquashStatus, TxHelp};
use std::ops::Deref;

type Data = web::Data<server::Data>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("mediation: {0}")]
    Mediation(#[from] mediation::Error),
    #[error("data: {0}")]
    Data(#[from] data::Error),
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Error::Mediation(mediation::Error::Unmediate(_)) => StatusCode::BAD_REQUEST,
            Error::Mediation(mediation::Error::Backend(_)) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Data(_) => StatusCode::BAD_REQUEST,
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
    data: Data,
    body: web::Bytes,
    // ) -> Result<Mediate<()>, Error> {
) -> Result<Mediate<SquashStatus>, Error> {
    let _: Result<_, Error> = Ok(Mediate(
        mediation.accept,
        data.squash(&keytag, Unmediate::unmediate(mediation.content, &body)?)?,
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
    data: Data,
    body: web::Bytes,
) -> Result<Mediate<SquashStatus>, Error> {
    let b = konduit_tmp::PayBody::unmediate(mediation.content, &body)?;
    let locked = Locked::new(b.cheque_body, b.signature);
    let body = data::PayBody {
        locked,
        invoice: b.invoice,
    };
    let _ = data.pay(&keytag, body).await?;
    // FIXME : The return type here has diverged!!
    squash_status(mediation, keytag, data).await
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use konduit_data::{AssetCatalog, AssetDefinition, AssetId, Pricing};

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
            50_001_501
        );
        assert_eq!(
            fx.asset_units_to_msat(
                50_000_000,
                6,
                data::asset_usd(&fx, &custom).unwrap(),
            )
                .unwrap(),
            100_000_000
        );

        let mut missing = fx;
        missing.assets.clear();
        assert!(data::quote_amount(&missing, &custom, 100_000_000).is_err());
    }

    struct NoopAdmin;

    #[async_trait::async_trait(?Send)]
    impl crate::admin::SyncApi for NoopAdmin {
        async fn sync(&self) -> anyhow::Result<()> {
            Ok(())
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
        };
        let data = server::Data::new(
            std::sync::Arc::new(bln_client::mock::Client::new()),
            db,
            std::sync::Arc::new(tokio::sync::RwLock::new(fx())),
            std::sync::Arc::new(info),
            std::sync::Arc::new(NoopAdmin),
        );
        (web::Data::new(data), keytag)
    }

    #[actix_web::test]
    async fn quote_handler_uses_authenticated_channel_definition() {
        for (definition, expected) in [
            (
                AssetCatalog::builtins().by_alias("usdm").unwrap().clone(),
                100_002_001,
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
            let req = actix_web::test::TestRequest::default().to_http_request();
            req.extensions_mut().insert(keytag);
            let body = serde_json::from_value::<QuoteBody>(serde_json::json!({
                "Simple": {
                    "amount_msat": 100_000_000,
                    "payee": "000000000000000000000000000000000000000000000000000000000000000000",
                    "route_hints": []
                }
            }))
            .unwrap();
            let response = quote(req, data, web::Json(body)).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let body = actix_web::body::to_bytes(response.into_body())
                .await
                .unwrap();
            let quote: Quote = serde_json::from_slice(&body).unwrap();
            assert_eq!(quote.amount, expected);
        }
    }
}

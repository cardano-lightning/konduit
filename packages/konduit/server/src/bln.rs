use konduit_rail::fx::{self, Fx};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Build the explicitly selected BLN backend. No selection means the rail is
/// intentionally absent from this adaptor and therefore from `/info`.
pub fn build_client() -> anyhow::Result<Option<Arc<dyn bln_client::Api>>> {
    use std::str::FromStr;

    let Some(backend) = std::env::var(crate::env::BLN_BACKEND).ok() else {
        return Ok(None);
    };
    let base_url = std::env::var(crate::env::BLN_URL).map_err(|_| {
        anyhow::anyhow!("{} is required when BLN is configured", crate::env::BLN_URL)
    })?;
    let block_time = match std::env::var(crate::env::BLN_BLOCK_TIME) {
        Ok(value) => humantime::parse_duration(&value)?,
        Err(_) => std::time::Duration::from_secs(600),
    };

    let client: Arc<dyn bln_client::Api> = match backend.to_ascii_lowercase().as_str() {
        "lnd" => {
            let macaroon = std::env::var(crate::env::BLN_MACAROON).map_err(|_| {
                anyhow::anyhow!(
                    "{} is required for the LND backend",
                    crate::env::BLN_MACAROON
                )
            })?;
            let macaroon =
                bln_client::lnd::Macaroon::from_str(&macaroon).map_err(anyhow::Error::msg)?;
            let tls_certificate = std::env::var(crate::env::BLN_TLS)
                .ok()
                .map(std::fs::read)
                .transpose()?;
            Arc::new(bln_client::lnd::Client::try_from(
                bln_client::lnd::Config::new(
                    base_url,
                    macaroon,
                    block_time,
                    84,
                    tls_certificate,
                    1000,
                ),
            )?)
        }
        "cln" => {
            let rune = std::env::var(crate::env::BLN_RUNE).map_err(|_| {
                anyhow::anyhow!("{} is required for the CLN backend", crate::env::BLN_RUNE)
            })?;
            Arc::new(bln_client::cln::Client::new(base_url, rune, block_time)?)
        }
        other => anyhow::bail!(
            "unsupported {} value {other:?}; expected `lnd` or `cln`",
            crate::env::BLN_BACKEND
        ),
    };
    Ok(Some(client))
}

/// FX service backed by the server's periodically refreshed ADA/BTC state.
pub struct StateFx {
    state: Arc<RwLock<fx_client::State>>,
}

impl StateFx {
    pub fn new(state: Arc<RwLock<fx_client::State>>) -> Self {
        Self { state }
    }
}

impl Fx for StateFx {
    fn inbound_to_outbound<'a>(
        &'a self,
        amount: u64,
    ) -> konduit_rail::BoxFuture<'a, Result<u64, fx::Error>> {
        Box::pin(async move {
            self.state
                .read()
                .await
                .lovelace_to_msat(amount)
                .map_err(conversion_error)
        })
    }

    fn outbound_to_inbound<'a>(
        &'a self,
        amount: u64,
    ) -> konduit_rail::BoxFuture<'a, Result<u64, fx::Error>> {
        Box::pin(async move {
            self.state
                .read()
                .await
                .msat_to_lovelace(amount)
                .map_err(conversion_error)
        })
    }
}

fn conversion_error(error: fx_client::Error) -> fx::Error {
    match error {
        fx_client::Error::InvalidAmount(detail) => fx::Error::Invalid(detail),
        other => fx::Error::Unavailable(other.to_string()),
    }
}

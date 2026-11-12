//! Konduit rail boundary for Bitcoin Lightning Network payments.
//!
//! BLN uses millisatoshis and Bitcoin block time, while the Konduit
//! instance uses its configured asset and Cardano time.

use bln_client::{
    Api,
    types::{Invoice, PayRequest, QuoteRequest, RevealRequest},
};
use bln_dto::{CommitInstruction, QuoteInstruction};
use bln_sdk::types::RouteHint;
use konduit_core::Lock;
use konduit_rail::fx::{self, Fx};
use konduit_rail::{Constraints, Outcome, PreProblem, QuoteProblem, Quoted, Rail, Resolution};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// BLN rail with a configured instance-asset ↔ millisatoshi converter.
pub struct Bln {
    fx: Arc<dyn Fx>,
    api: Arc<dyn Api>,
}

impl Bln {
    pub fn new(fx: Arc<dyn Fx>, api: Arc<dyn Api>) -> Self {
        Self { fx, api }
    }
}

impl Rail for Bln {
    const NAME: &'static str = "bln";
    type QuoteInstruction = QuoteInstruction;
    type CommitInstruction = CommitInstruction;

    async fn quote(
        &self,
        amount: u64,
        instruction: Self::QuoteInstruction,
    ) -> Result<Quoted, QuoteProblem> {
        let invoice = parse_invoice(&instruction.invoice)?;
        let outbound_ceiling = self
            .fx
            .inbound_to_outbound(amount)
            .await
            .map_err(fx_problem)?;
        if invoice.amount_msat > outbound_ceiling {
            return Err(QuoteProblem::Unroutable(
                "invoice amount exceeds the requested inbound amount after FX conversion".into(),
            ));
        }
        let response = self
            .api
            .quote(quote_request(&invoice))
            .await
            .map_err(api_problem)?;
        if response.amount_msat != invoice.amount_msat
            || response.payee != invoice.payee_compressed.serialize()
        {
            return Err(QuoteProblem::RailUnavailable(
                "Lightning backend returned a quote for a different invoice".into(),
            ));
        }
        let fee = self
            .fx
            .outbound_to_inbound(response.fee_msat)
            .await
            .map_err(fx_problem)?;
        // FX integer division truncates. One lovelace of headroom keeps the
        // consumer's quote from understating a non-zero converted fee.
        let fee = if response.fee_msat == 0 {
            fee
        } else {
            fee.checked_add(1).ok_or_else(|| {
                QuoteProblem::AmountOutOfRange("converted fee overflows lovelace".into())
            })?
        };
        let timeout = invoice_timeout(&invoice, response.relative_timeout)
            .ok_or_else(|| QuoteProblem::Unroutable("invoice has expired".into()))?;
        if timeout.is_zero() {
            return Err(QuoteProblem::Unroutable(
                "no payment timeout remains before invoice expiry".into(),
            ));
        }
        Ok(Quoted {
            fee,
            timeout: konduit_rail::Relative::from_millis(
                u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX),
            ),
        })
    }

    async fn commit(&self, inbound: Constraints, instruction: Self::CommitInstruction) -> Outcome {
        let invoice = match parse_invoice(&instruction.invoice) {
            Ok(invoice) => invoice,
            Err(problem) => return Outcome::FailedPre(problem.into()),
        };
        if invoice.payment_hash != inbound.lock.0 {
            return Outcome::FailedPre(PreProblem::Quote(QuoteProblem::Unroutable(
                "invoice payment hash does not match inbound lock".to_string(),
            )));
        }
        let outbound_ceiling = match self.fx.inbound_to_outbound(inbound.amount).await {
            Ok(amount) => amount,
            Err(error) => return Outcome::FailedPre(PreProblem::Quote(fx_problem(error))),
        };
        if invoice.amount_msat > outbound_ceiling {
            return Outcome::FailedPre(PreProblem::Quote(QuoteProblem::Unroutable(
                "invoice amount exceeds the authorized inbound amount after FX conversion".into(),
            )));
        }
        let available_fee_msat = outbound_ceiling - invoice.amount_msat;
        let route_quote = match self.api.quote(quote_request(&invoice)).await {
            Ok(quote) => quote,
            // A quote is read-only: failure is certain to precede payment dispatch.
            Err(error) => return Outcome::FailedPre(PreProblem::Quote(api_problem(error))),
        };
        let remaining_timeout = match invoice_timeout(&invoice, route_quote.relative_timeout) {
            Some(timeout) => timeout,
            None => {
                return Outcome::FailedPre(PreProblem::Quote(QuoteProblem::Unroutable(
                    "invoice is expired or has no remaining payment timeout".into(),
                )));
            }
        };
        if remaining_timeout > *inbound.timeout.0 {
            return Outcome::FailedPre(PreProblem::Quote(QuoteProblem::Unroutable(
                "payment route timeout exceeds the inbound cheque timeout".into(),
            )));
        }
        if route_quote.fee_msat > available_fee_msat {
            return Outcome::FailedPre(PreProblem::Quote(QuoteProblem::Unroutable(
                "current Lightning fee exceeds the authorized maximum".into(),
            )));
        }
        // Reuse the current route quote but replace its fee with the maximum
        // authorized by the cheque. The backend enforces this as an absolute cap.
        let mut capped_quote = route_quote;
        capped_quote.fee_msat = available_fee_msat;
        capped_quote.relative_timeout = remaining_timeout;
        match self
            .api
            .commit(PayRequest {
                quote: capped_quote,
                invoice: invoice.clone(),
            })
            .await
        {
            Ok(response) => match response.secret {
                Some(secret) if payment_hash(&secret) == invoice.payment_hash => {
                    Outcome::Success(konduit_core::Secret(secret))
                }
                Some(_) => Outcome::Pending,
                None => Outcome::Pending,
            },
            // A REST failure can happen after dispatch reached the node.
            // Let reveal recover the result rather than release the cheque.
            Err(_) => Outcome::Pending,
        }
    }

    async fn reveal(&self, lock: &Lock) -> Resolution {
        match self.api.reveal(RevealRequest { lock: lock.0 }).await {
            Ok(response) => match response.secret {
                Some(secret) if payment_hash(&secret) == lock.0 => {
                    Resolution::Success(konduit_core::Secret(secret))
                }
                Some(_) => Resolution::Failed(konduit_rail::PostProblem::OutboundRejected(
                    "backend returned a preimage that does not match the payment hash".into(),
                )),
                None => Resolution::Unknown,
            },
            Err(_) => Resolution::Pending,
        }
    }
}

fn fx_problem(error: fx::Error) -> QuoteProblem {
    QuoteProblem::FxUnavailable(error.to_string())
}

fn parse_invoice(invoice: &str) -> Result<Invoice, QuoteProblem> {
    Invoice::from_str(invoice).map_err(|error| QuoteProblem::Unroutable(error.to_string()))
}

fn quote_request(invoice: &Invoice) -> QuoteRequest {
    QuoteRequest {
        amount_msat: invoice.amount_msat,
        payee: invoice.payee_compressed.serialize(),
        route_hints: invoice
            .private_route
            .iter()
            .cloned()
            .map(|route| RouteHint::from(route.into_inner()))
            .collect(),
    }
}

fn invoice_timeout(invoice: &Invoice, route_timeout: Duration) -> Option<Duration> {
    // BOLT11's omitted expiry field has the protocol default of one hour.
    let expiry = invoice.expiry_time.unwrap_or(Duration::from_secs(3600));
    let (raw, _, _) = invoice.__raw.clone().into_parts();
    let created = raw.data.timestamp.as_duration_since_epoch();
    let expires_at = created.checked_add(expiry)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
    let invoice_remaining = expires_at.checked_sub(now)?;
    Some(route_timeout.min(invoice_remaining))
}

fn payment_hash(secret: &[u8; 32]) -> [u8; 32] {
    use bitcoin::hashes::Hash;
    bitcoin::hashes::sha256::Hash::hash(secret).to_byte_array()
}

fn api_problem(error: bln_client::Error) -> QuoteProblem {
    match error {
        bln_client::Error::ApiError {
            status: 404,
            message,
        } => QuoteProblem::Unroutable(message),
        other => QuoteProblem::RailUnavailable(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::{
        hashes::Hash as _,
        secp256k1::{Secp256k1, SecretKey},
    };
    use bln_client::{
        CreateInvoiceRequest, CreatedInvoice, Health,
        types::{PayResponse, QuoteResponse, RevealResponse},
    };
    use std::sync::Mutex;

    struct IdentityFx;

    impl Fx for IdentityFx {
        fn inbound_to_outbound<'a>(
            &'a self,
            amount: u64,
        ) -> konduit_rail::BoxFuture<'a, Result<u64, fx::Error>> {
            Box::pin(async move { Ok(amount) })
        }

        fn outbound_to_inbound<'a>(
            &'a self,
            amount: u64,
        ) -> konduit_rail::BoxFuture<'a, Result<u64, fx::Error>> {
            Box::pin(async move { Ok(amount) })
        }
    }

    struct FakeApi {
        secret: [u8; 32],
        lock: [u8; 32],
        committed_fee_cap: Mutex<Option<u64>>,
    }

    #[async_trait::async_trait]
    impl Api for FakeApi {
        async fn health(&self) -> bln_client::Result<Health> {
            unreachable!("health is not part of the rail flow")
        }

        async fn create_invoice(
            &self,
            _request: CreateInvoiceRequest,
        ) -> bln_client::Result<CreatedInvoice> {
            unreachable!("invoice creation is not part of the rail flow")
        }

        async fn quote(&self, request: QuoteRequest) -> bln_client::Result<QuoteResponse> {
            Ok(QuoteResponse {
                backend: "fake".into(),
                amount_msat: request.amount_msat,
                payee: request.payee,
                relative_timeout: Duration::from_secs(30),
                fee_msat: 10,
            })
        }

        async fn commit(&self, request: PayRequest) -> bln_client::Result<PayResponse> {
            *self.committed_fee_cap.lock().unwrap() = Some(request.quote.fee_msat);
            Ok(PayResponse {
                secret: Some(self.secret),
            })
        }

        async fn reveal(&self, request: RevealRequest) -> bln_client::Result<RevealResponse> {
            Ok(RevealResponse {
                secret: (request.lock == self.lock).then_some(self.secret),
            })
        }
    }

    fn signed_invoice() -> (String, [u8; 32], [u8; 32]) {
        use lightning_invoice::{Currency, InvoiceBuilder, PaymentSecret};
        use std::time::SystemTime;

        let secret = [0x42; 32];
        let payment_hash = bitcoin::hashes::sha256::Hash::hash(&secret).to_byte_array();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is after Unix epoch");
        let key = SecretKey::from_slice(&[7; 32]).unwrap();
        let signed = InvoiceBuilder::new(Currency::Bitcoin)
            .amount_milli_satoshis(100_000)
            .duration_since_epoch(now)
            .payment_secret(PaymentSecret([0x24; 32]))
            .payment_hash(bitcoin::hashes::sha256::Hash::from_byte_array(payment_hash))
            .description("konduit test payment".to_owned())
            .expiry_time(Duration::from_secs(3600))
            .build_raw()
            .unwrap()
            .sign(|message| {
                Ok::<_, std::convert::Infallible>(
                    Secp256k1::new().sign_ecdsa_recoverable(message, &key),
                )
            })
            .unwrap();
        (signed.to_string(), secret, payment_hash)
    }

    fn test_rail(secret: [u8; 32], lock: [u8; 32]) -> (Bln, Arc<FakeApi>) {
        let api = Arc::new(FakeApi {
            secret,
            lock,
            committed_fee_cap: Mutex::new(None),
        });
        (Bln::new(Arc::new(IdentityFx), api.clone()), api)
    }

    #[test]
    fn dto_instructions_round_trip_through_json_and_cbor() {
        let quote = QuoteInstruction {
            invoice: "lnbc1example".to_string(),
        };
        let json = serde_json::to_vec(&quote).unwrap();
        assert_eq!(
            serde_json::from_slice::<QuoteInstruction>(&json).unwrap(),
            quote
        );

        let cbor = minicbor::to_vec(&quote).unwrap();
        assert_eq!(minicbor::decode::<QuoteInstruction>(&cbor).unwrap(), quote);

        let commit = CommitInstruction {
            invoice: "lnbc1example".to_string(),
        };
        let cbor = minicbor::to_vec(&commit).unwrap();
        assert_eq!(
            minicbor::decode::<CommitInstruction>(&cbor).unwrap(),
            commit
        );
    }

    #[test]
    fn rail_name_is_the_feature_boundary_name() {
        assert_eq!(Bln::NAME, "bln");
    }

    #[test]
    fn converter_failure_becomes_the_rail_fx_problem() {
        let problem = fx_problem(fx::Error::Unavailable("no rate data".to_string()));
        assert_eq!(problem.slug(), "fx-unavailable");
        assert_eq!(problem.status(), 503);
    }

    #[tokio::test]
    async fn quote_allows_invoice_principal_at_or_below_inbound_ceiling() {
        let (invoice, secret, lock) = signed_invoice();
        let (rail, _) = test_rail(secret, lock);
        let quoted = rail
            .quote(
                100_000,
                QuoteInstruction {
                    invoice: invoice.clone(),
                },
            )
            .await
            .unwrap();
        assert_eq!(quoted.fee, 11, "one unit covers integer FX truncation");

        let too_small = rail
            .quote(99_999, QuoteInstruction { invoice })
            .await
            .unwrap_err();
        assert_eq!(too_small.slug(), "unroutable");
    }

    #[tokio::test]
    async fn commit_caps_backend_fee_and_validates_the_returned_preimage() {
        let (invoice, secret, lock) = signed_invoice();
        let (rail, api) = test_rail(secret, lock);
        let outcome = rail
            .commit(
                Constraints {
                    amount: 105_000,
                    timeout: konduit_rail::Relative::from_secs(60),
                    lock: Lock(lock),
                },
                CommitInstruction { invoice },
            )
            .await;
        assert_eq!(outcome, Outcome::Success(konduit_core::Secret(secret)));
        assert_eq!(*api.committed_fee_cap.lock().unwrap(), Some(5_000));
    }

    #[tokio::test]
    async fn commit_refuses_lock_mismatch_before_dispatch() {
        let (invoice, secret, lock) = signed_invoice();
        let (rail, api) = test_rail(secret, lock);
        let outcome = rail
            .commit(
                Constraints {
                    amount: 105_000,
                    timeout: konduit_rail::Relative::from_secs(60),
                    lock: Lock([0; 32]),
                },
                CommitInstruction { invoice },
            )
            .await;
        assert!(matches!(outcome, Outcome::FailedPre(_)));
        assert_eq!(*api.committed_fee_cap.lock().unwrap(), None);
    }

    #[tokio::test]
    async fn reveal_verifies_the_preimage_hash() {
        let (invoice, _valid_secret, lock) = signed_invoice();
        let parsed = parse_invoice(&invoice).unwrap();
        let (rail, _) = test_rail([0x99; 32], parsed.payment_hash);
        assert!(matches!(
            rail.reveal(&Lock(lock)).await,
            Resolution::Failed(_)
        ));
    }
}

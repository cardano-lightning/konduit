use crate::{
    Adaptor,
    core::{
        AdaptorInfo, ChequeBody, Duration, Invoice, Lock, Locked, Quote, Receipt, SigningKey,
        Squash, SquashBody, SquashStatus, Tag, VerificationKey,
    },
};
use anyhow::{Context, anyhow};
use http_client::HttpClient;
use std::ops::Deref;
use web_time::{SystemTime, UNIX_EPOCH};

pub struct Client<Http: HttpClient> {
    adaptor: Adaptor<Http>,
    signing_key: SigningKey,
    tag: Tag,
}

impl<Http: HttpClient> Client<Http>
where
    Http::Error: Into<anyhow::Error>,
{
    pub fn new(adaptor: Adaptor<Http>, signing_key: SigningKey, tag: Tag) -> Self {
        Self {
            adaptor,
            signing_key,
            tag,
        }
    }

    pub fn info(&self) -> &AdaptorInfo {
        self.adaptor.info()
    }

    pub async fn quote(&self, invoice: &str) -> anyhow::Result<Quote> {
        self.adaptor
            .quote(invoice.parse().context("failed to parse bolt11 invoice")?)
            .await
    }

    pub async fn receipt(&self) -> anyhow::Result<Option<Receipt>> {
        self.adaptor.receipt().await
    }

    pub async fn pay(
        &self,
        invoice: &str,
        quote: impl Deref<Target = Quote>,
    ) -> anyhow::Result<SquashStatus> {
        let payment_hash = invoice
            .parse::<Invoice>()
            .context("failed to parse bolt11 invoice")?
            .payment_hash;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("failed calculate duration since UNIX epoch ?!")
            .as_millis() as u64;

        let timeout = Duration::from_millis(now + quote.relative_timeout);

        let body = ChequeBody::new(quote.index, quote.amount, timeout, Lock(payment_hash));

        let locked = Locked::make(&self.signing_key, &self.tag, body);

        self.adaptor.pay(invoice, locked).await
    }

    pub async fn squash(&self, squash_body: SquashBody) -> anyhow::Result<SquashStatus> {
        let squash = Squash::make(&self.signing_key, &self.tag, squash_body);
        self.adaptor.squash(squash).await
    }

    pub async fn sync(&self, squash: SquashStatus, and_confirm: bool) -> anyhow::Result<()> {
        match squash {
            SquashStatus::Complete => {
                log::info!("nothing to squash");
                Ok(())
            }
            SquashStatus::Stale(_) => {
                log::info!("squash stale");
                Ok(())
            }
            SquashStatus::Incomplete(_) if !and_confirm => {
                log::info!("squash incomplete but aborted by user");
                Ok(())
            }
            SquashStatus::Incomplete(st) => {
                log::info!("squash incomplete; verifying...");

                let verification_key = VerificationKey::from(&self.signing_key);

                // 1. Verify the current squash
                if !st.current.verify(&verification_key, &self.tag) {
                    return Err(anyhow!("current squash does not verify"));
                }
                log::info!("currently squashed = {}", st.current.amount());

                // 2. Sum-verify all the unlockeds
                let unlocked_value = st.unlockeds.iter().try_fold(0, |value, unlocked| {
                    // NOTE: Handles timeout when verifying unlocked (or not?)
                    //
                    // Although... unclear how the client can 'reliably' keep track of timeout.
                    // In the current approach, the client rely heavily on the adaptor for
                    // recovering its state; this means that an adaptor could be attempting to
                    // make the client squash a timed out unlock... This isn't as bad as it
                    // seems since:
                    //
                    // - the adaptor is still capable of providing the secret, which means that
                    // we can reasonably assume that the other end of the payment got its due
                    // and released it.
                    // - the locked cheque was still emitted (signed) by the consumer, so they
                    // definitely intented to make that payment.
                    if !unlocked.verify_no_time(&verification_key, &self.tag) {
                        return Err(anyhow!("current squash does not verify"));
                    }

                    Ok(value + unlocked.amount())
                })?;

                log::info!("total unlocked = {}", unlocked_value);

                if st.proposal.amount > st.current.amount() + unlocked_value {
                    return Err(anyhow!(
                        "adaptor requesting to squash more than provably owed"
                    ));
                }

                log::info!("proposal = {:?}", &st.proposal);

                let res = self.squash(st.proposal).await?;

                Box::pin(self.sync(res, and_confirm)).await
            }
        }
    }
}

#[cfg(feature = "wasm")]
pub mod wasm {
    use crate::{
        core::wasm::{AdaptorInfo, SigningKey, Tag},
        wasm::Adaptor,
        wasm_proxy,
    };
    use http_client_wasm::HttpClient;
    use std::ops::Deref;
    use wasm_bindgen::prelude::*;

    wasm_proxy! {
        #[doc = "An L2 client for Konduit, bespoke a single consumer key-tag and adaptor."]
        Client => super::Client<HttpClient>
    }

    #[wasm_bindgen]
    impl Client {
        #[wasm_bindgen(constructor)]
        pub fn _wasm_new(adaptor: &Adaptor, signing_key: &SigningKey, tag: &Tag) -> Self {
            let signing_key = signing_key.clone().into();
            let tag = tag.deref().clone();
            Self::from(super::Client::new(adaptor.clone().into(), signing_key, tag))
        }

        #[wasm_bindgen(getter, js_name = "info")]
        pub fn _wasm_info(&self) -> AdaptorInfo {
            self.adaptor.info().clone().into()
        }
    }
}

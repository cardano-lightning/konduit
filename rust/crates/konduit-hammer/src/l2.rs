use crate::{
    Adaptor,
    core::{
        AdaptorInfo, ChequeBody, Duration, Invoice, Lock, Locked, Quote, Receipt, SigningKey,
        Squash, SquashBody, SquashStatus,
    },
};
use anyhow::anyhow;
use http_client::HttpClient;
use web_time::{SystemTime, UNIX_EPOCH};

pub struct Client<'a, Http: HttpClient> {
    adaptor: &'a Adaptor<Http>,
    signing_key: &'a SigningKey,
}

impl<'a, Http> Client<'a, Http>
where
    Http: HttpClient,
    Http::Error: Into<anyhow::Error>,
{
    pub fn new(adaptor: &'a Adaptor<Http>, signing_key: &'a SigningKey) -> Self {
        Self {
            adaptor,
            signing_key,
        }
    }

    // pub fn info(&self) -> &AdaptorInfo<()> {
    //     self.adaptor.info()
    // }

    // pub async fn quote(&self, invoice: &Invoice) -> anyhow::Result<Quote> {
    //     self.adaptor.quote(invoice).await
    // }

    // pub async fn receipt(&self) -> anyhow::Result<Option<Receipt>> {
    //     self.adaptor.receipt().await
    // }

    // pub async fn pay(&self, invoice: &Invoice, quote: &Quote) -> anyhow::Result<SquashStatus> {
    //     let now = SystemTime::now()
    //         .duration_since(UNIX_EPOCH)
    //         .expect("failed calculate duration since UNIX epoch ?!")
    //         .as_millis() as u64;

    //     let timeout = Duration::from_millis(now + quote.relative_timeout);

    //     let body = ChequeBody::new(
    //         quote.index,
    //         quote.amount,
    //         timeout,
    //         Lock(invoice.payment_hash),
    //     );

    //     let tag = self.adaptor.tag().ok_or(anyhow!("no tag set on adaptor"))?;

    //     let locked = Locked::make(self.signing_key, tag, body);

    //     self.adaptor.pay(invoice, locked).await
    // }

    // pub async fn squash(&self, squash_body: SquashBody) -> anyhow::Result<SquashStatus> {
    //     let tag = self.adaptor.tag().ok_or(anyhow!("no tag set on adaptor"))?;
    //     let squash = Squash::make(self.signing_key, tag, squash_body);
    //     self.adaptor.squash(squash).await
    // }

    // /// Synchronize with an adaptor. 'expected_unlockeds' can be used by the client to ensure
    // /// that the adaptor isn't trying to squash a very old cheque that should be considered
    // /// expired.
    // ///
    // /// Returns unlocked cheques that have been squashed, if any.
    // pub async fn sync(
    //     &self,
    //     squash: SquashStatus,
    //     and_confirm: bool,
    //     known_lock: impl Fn(Lock) -> bool,
    // ) -> anyhow::Result<Vec<Lock>> {
    //     let tag = self.adaptor.tag().ok_or(anyhow!("no tag set on adaptor"))?;
    //     match squash {
    //         SquashStatus::Complete => {
    //             log::info!("nothing to squash");
    //             Ok(vec![])
    //         }
    //         SquashStatus::Incomplete(st) if and_confirm => {
    //             log::info!("squash incomplete; verifying...");

    //             let verification_key = self.signing_key.to_verification_key();

    //             // 1. Verify the current squash
    //             if !st.current.verify(&verification_key, tag) {
    //                 return Err(anyhow!("current squash does not verify"));
    //             }
    //             log::info!("currently squashed = {}", st.current.amount());

    //             let current_squash_index = st.current.index();

    //             // 2. Sum-verify all the unlockeds
    //             //
    //             // NOTE: Handling timeouts when verifying unlocked
    //             //
    //             // There's an ambiguity when payments aren't resolved immediately. From a consumer
    //             // standpoint, their 'authorisation' is out there in the wild and the payment
    //             // may be valid up until its timeout.
    //             //
    //             // However, consumers may not be online at the moment adaptors get to know the
    //             // secret, and thus may be unable to squash. This is a scenario where an adaptor
    //             // may need to sub directly from the L1, without which it has no guarantee to get
    //             // its due back.
    //             //
    //             // Yet, the adaptor may still attempt to squash the unlocked cheque since there's
    //             // only a limited number of cheques that can be subbed but not squashed on-chain
    //             // (limitation imposed by the smart contract layer).
    //             //
    //             // When a client comes back online, an Adaptor may thus attempt to squash cheques
    //             // that are technically expired. In principle, this is not an issue because:
    //             //
    //             // - the adaptor is capable of providing the secret, which means that
    //             // we can reasonably assume that the other end of the payment got its due
    //             // and released it.
    //             //
    //             // - the locked cheque was still emitted (signed) by the consumer, so they
    //             // definitely intented to make that payment.
    //             //
    //             // So, it is only truly an issue for the consumer when it comes to determining its
    //             // currently available balance: a consumer needs to be able to rely on the cheque
    //             // to timeout eventually so that it can update its own reported state.
    //             let mut squashed_unlockeds = vec![];
    //             let unlocked_value = st.unlockeds.into_iter().try_fold(0, |value, unlocked| {
    //                 if unlocked.index() <= current_squash_index {
    //                     log::warn!(
    //                         "adaptor trying to replay an old squash with index={}, but current squash is at index={}",
    //                         unlocked.index(),
    //                         current_squash_index,
    //                     );
    //                     return Ok(value);

    //                 }

    //                 if !known_lock(*unlocked.lock()) {
    //                     // NOTE: No error raised here, because it'll be raised below as the squash
    //                     // amount wouldn't match.
    //                     log::warn!(
    //                         "adaptor reported an unexpected, likely expired, unlocked: {unlocked:?}"
    //                     );
    //                     return Ok(value);
    //                 }

    //                 if !unlocked.verify_no_time(&verification_key, tag) {
    //                     return Err(anyhow!("current squash does not verify"));
    //                 }

    //                 squashed_unlockeds.push(*unlocked.lock());

    //                 Ok(value + unlocked.amount())
    //             })?;

    //             log::info!("total unlocked = {}", unlocked_value);

    //             if st.proposal.amount > st.current.amount() + unlocked_value {
    //                 return Err(anyhow!(
    //                     "adaptor requesting to squash more than provably owed"
    //                 ));
    //             }

    //             log::info!("proposal = {:?}", &st.proposal);

    //             let res = self.squash(st.proposal).await?;

    //             // Synchronize again with the adaptor, in case there are now  more squashes to do.
    //             // In most cases, this should simply hit the Complete case.
    //             let extra_squashes = Box::pin(self.sync(res, and_confirm, known_lock)).await?;

    //             Ok(squashed_unlockeds
    //                 .into_iter()
    //                 .chain(extra_squashes)
    //                 .collect())
    //         }
    //         SquashStatus::Stale(_) | SquashStatus::Incomplete(_) => {
    //             log::info!("squash stale or incomplete");
    //             Ok(vec![])
    //         }
    //     }
    // }
}

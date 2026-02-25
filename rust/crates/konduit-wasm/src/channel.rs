use crate::{
    Adaptor, Wallet,
    adaptor::{QuoteResponse, SquashResponse},
};
use anyhow::anyhow;
use bln_sdk::types as bln;
use cardano_connector_client::{
    CardanoConnector,
    wasm::{self, StrError},
};
use cardano_sdk::{
    Credential, Input, Output, VerificationKey, address::ShelleyAddress, hash::Hash32,
};
use konduit_data::{ChequeBody, Duration, Lock, Locked, Squash, SquashBody, Stage, Tag};
use konduit_tx::{
    Bounds, ChannelOutput, NetworkParameters,
    consumer::{Intent, OpenIntent},
    filter_channels,
};
use log::{debug, error};
use std::{collections::BTreeMap, ops::Deref, str::FromStr};
use wasm_bindgen::prelude::*;
use web_time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
#[wasm_bindgen]
pub struct Channel(ChannelOutput);

#[wasm_bindgen]
impl Channel {
    #[wasm_bindgen(getter, js_name = "tag")]
    pub fn tag(&self) -> Tag {
        self.0.constants.tag.clone()
    }

    /// Return the initial amount deposited in the channel. We track the remainder using receipts.
    #[wasm_bindgen(getter, js_name = "amount")]
    pub fn amount(&self) -> u64 {
        let subbed = match self.0.stage {
            Stage::Opened(subbed, _)
            | Stage::Closed(subbed, _, _)
            | Stage::Responded(subbed, _) => subbed,
        };

        self.0.amount + subbed + 2_000_000
    }

    #[wasm_bindgen(js_name = "receipt")]
    pub async fn receipt(&self, consumer: &Wallet, adaptor: &Adaptor) -> wasm::Result<u64> {
        let vk = consumer.verification_key();
        let tag = self.tag();

        let receipt_opt = adaptor
            .receipt(&consumer.verification_key(), &self.tag())
            .await?;

        // 1. Inspect the receipt to collect the amount we owe the adaptor, but only trust squash
        //    and cheques that verify.
        let unavailable = if let Some(receipt) = receipt_opt.deref() {
            let squash_amount = if receipt.squash.verify(&vk, &tag) {
                receipt.squash.amount()
            } else {
                error!("squash in receipt does not verify; ignoring");
                0
            };

            let cheques_amount = receipt.cheques.iter().fold(0, |total, cheque| {
                total
                    + if cheque.verify(&vk, &tag) {
                        cheque.amount()
                    } else {
                        error!("cheque in receipt does not verify; ignoring");
                        0
                    }
            });

            squash_amount + cheques_amount
        } else {
            0
        };

        // 2. Regardless, always send a null squash to check if there's any unresolved state to
        //    settle with the server.
        let null_squash = Squash::new(
            SquashBody::default(),
            consumer.sign(SquashBody::default().tagged_bytes(&tag)).1,
        );
        let squash_response = adaptor.squash(null_squash, &vk, &tag).await?;
        self.handle_squash_reponse(squash_response, consumer, adaptor)
            .await?;

        Ok(unavailable)
    }

    pub async fn get_quote(
        &self,
        adaptor: &Adaptor,
        consumer: &Wallet,
        invoice: &str,
    ) -> wasm::Result<QuoteResponse> {
        adaptor
            .quote(invoice, &consumer.verification_key(), &self.tag())
            .await
    }

    pub async fn pay(
        &self,
        adaptor: &Adaptor,
        consumer: &Wallet,
        invoice: &str,
        quote: &QuoteResponse,
    ) -> wasm::Result<()> {
        debug!("quote = {quote:?}");

        let payment_hash = bln::Invoice::from_str(invoice)
            .map_err(|e| StrError::from(anyhow!(e)))?
            .payment_hash;

        debug!("payment_hash = {:?}", hex::encode(payment_hash));

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("failed calculate duration since UNIX epoch ?!")
            .as_millis() as u64;
        let timeout = Duration::from_millis(now + quote.relative_timeout);

        let body = ChequeBody::new(
            quote.index,
            quote.amount + adaptor.info().fee,
            timeout,
            Lock(payment_hash),
        );
        let signature = consumer.sign(body.tagged_bytes(&self.tag())).1;
        let locked = Locked::new(body, signature);

        debug!("locked = {locked:?}");

        let ask = adaptor
            .pay(invoice, locked, &consumer.verification_key(), &self.tag())
            .await?;

        debug!("squash response = {ask:?}");

        self.handle_squash_reponse(ask, consumer, adaptor).await
    }

    /// Find currently opened channels.
    #[wasm_bindgen(js_name = "opened")]
    pub async fn opened(
        connector: &wasm::Connector,
        consumer: &Wallet,
        konduit_validator: &Credential,
    ) -> wasm::Result<Vec<Channel>> {
        let consumer_key = consumer.verification_key();

        let utxos_konduit = utxos_at_address(
            connector,
            konduit_validator,
            consumer.stake_credential().as_ref(),
        )
        .await?;

        Ok(filter_channels(&utxos_konduit.collect(), |channel| {
            channel.constants.add_vkey == consumer_key
        })
        .into_iter()
        .filter_map(|(_, channel)| match channel.stage {
            Stage::Opened { .. } => Some(Self(channel)),
            Stage::Closed { .. } | Stage::Responded { .. } => None,
        })
        .collect())
    }

    #[wasm_bindgen(js_name = "open")]
    pub async fn open(
        connector: &wasm::Connector,
        consumer: &Wallet,
        script_deployment_address: &ShelleyAddress,
        tag: &Tag,
        adaptor_key: &VerificationKey,
        close_period_secs: u64,
        amount: u64,
    ) -> wasm::Result<Hash32> {
        let network_parameters = NetworkParameters {
            network_id: connector.network_id(),
            protocol_parameters: connector.protocol_parameters().await?,
        };

        let consumer_key = consumer.verification_key();

        let opens = vec![OpenIntent {
            tag: tag.clone(),
            sub_vkey: *adaptor_key,
            close_period: Duration::from_secs(close_period_secs),
            amount,
        }];

        let intents = BTreeMap::new();

        let utxos_consumer = utxos_at_address(
            connector,
            &consumer.payment_credential(),
            consumer.stake_credential().as_ref(),
        )
        .await?;

        let utxos_script_ref = connector
            .utxos_at(
                &script_deployment_address.payment(),
                script_deployment_address.delegation().as_ref(),
            )
            .await?;

        let mut tx = konduit_tx::consumer::tx(
            &network_parameters,
            &consumer_key,
            opens,
            intents,
            &utxos_consumer.chain(utxos_script_ref).collect(),
            Bounds::twenty_mins(),
        )?;

        tx.sign_with(|msg| consumer.sign(msg));

        connector.submit(&tx).await?;

        Ok(tx.id().into())
    }

    #[wasm_bindgen(js_name = "close")]
    pub async fn close(
        connector: &wasm::Connector,
        consumer: &Wallet,
        konduit_validator: &Credential,
        script_deployment_address: &ShelleyAddress,
        tag: &Tag,
    ) -> wasm::Result<Hash32> {
        let network_parameters = NetworkParameters {
            network_id: connector.network_id(),
            protocol_parameters: connector.protocol_parameters().await?,
        };

        let consumer_key = consumer.verification_key();

        let opens = vec![];

        let intents = BTreeMap::from([(tag.clone(), Intent::Close)]);

        let stake_credential = consumer.stake_credential();

        let utxos_konduit =
            utxos_at_address(connector, konduit_validator, stake_credential.as_ref()).await?;

        let utxos_consumer = utxos_at_address(
            connector,
            &Credential::from(&consumer.verification_key()),
            stake_credential.as_ref(),
        )
        .await?;

        let utxos_script_ref = connector
            .utxos_at(
                &script_deployment_address.payment(),
                script_deployment_address.delegation().as_ref(),
            )
            .await?;

        let mut tx = konduit_tx::consumer::tx(
            &network_parameters,
            &consumer_key,
            opens,
            intents,
            &utxos_konduit
                .chain(utxos_consumer)
                .chain(utxos_script_ref)
                .collect(),
            Bounds::twenty_mins(),
        )?;

        tx.sign_with(|msg| consumer.sign(msg));

        connector.submit(&tx).await?;

        Ok(tx.id().into())
    }
}

impl Channel {
    async fn handle_squash_reponse(
        &self,
        squash_response: SquashResponse,
        consumer: &Wallet,
        adaptor: &Adaptor,
    ) -> wasm::Result<()> {
        match squash_response {
            SquashResponse::Complete => {
                debug!("nothing to squash");
                Ok(())
            }
            SquashResponse::Incomplete(st) | SquashResponse::Stale(st) => {
                // 1. Verify the current squash
                if !st.current.verify(&consumer.verification_key(), &self.tag()) {
                    return Err(anyhow!("current squash does not verify").into());
                }
                debug!("currently squashed = {}", st.current.amount());

                // 2. Sum-verify all the unlockeds
                let unlocked_value = st.unlockeds.iter().try_fold(0, |value, unlocked| {
                    if !unlocked.verify_no_time(&consumer.verification_key(), &self.tag()) {
                        // TODO: Handles timeout when verifying unlocked (or not?)
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
                        return Err(anyhow!("current squash does not verify"));
                    }

                    Ok(value + unlocked.amount())
                })?;

                debug!("unlocked value = {}", unlocked_value);

                if st.proposal.amount > st.current.amount() + unlocked_value {
                    return Err(
                        anyhow!("adaptor requesting to squash more than provably owed").into(),
                    );
                }

                // 3. Create the requested squash
                let signature = consumer.sign(st.proposal.tagged_bytes(&self.tag())).1;
                let squash = Squash::new(st.proposal.clone(), signature);

                debug!("squash = {:?}", squash);

                // 3. Submit the squash
                adaptor
                    .squash(squash, &consumer.verification_key(), &self.tag())
                    .await
                    .inspect_err(|e| error!("adaptor failed to squash: {e}"))?;

                Ok(())
            }
        }
    }
}

async fn utxos_at_address(
    connector: &wasm::Connector,
    payment_credential: &Credential,
    stake_credential_opt: Option<&Credential>,
) -> wasm::Result<impl Iterator<Item = (Input, Output)> + use<>> {
    Ok(connector
        .utxos_at(payment_credential, None)
        .await?
        .into_iter()
        .chain(if let Some(stake_credential) = stake_credential_opt {
            connector
                .utxos_at(payment_credential, Some(stake_credential))
                .await?
        } else {
            BTreeMap::new()
        }))
}

use crate::{
    Connector, Wallet,
    core::{
        ChannelOutput, Duration, MIN_ADA_BUFFER, SquashBody, Stage,
        consumer::{Intent, OpenIntent},
        wasm,
    },
    l1, l2,
};
use std::ops::Deref;
use wasm_bindgen::prelude::*;

#[derive(Debug)]
#[wasm_bindgen]
pub struct Channel(ChannelOutput);

#[wasm_bindgen]
impl Channel {
    #[wasm_bindgen(getter, js_name = "tag")]
    pub fn tag(&self) -> wasm::Tag {
        self.0.constants.tag.clone().into()
    }

    /// Return the initial amount deposited in the channel. We track the remainder using receipts.
    #[wasm_bindgen(getter, js_name = "amount")]
    pub fn amount(&self) -> u64 {
        let subbed = match self.0.stage {
            Stage::Opened(subbed, _)
            | Stage::Closed(subbed, _, _)
            | Stage::Responded(subbed, _) => subbed,
        };

        self.0.amount + subbed + MIN_ADA_BUFFER
    }

    #[wasm_bindgen(js_name = "receipt")]
    pub async fn receipt(&self, consumer: &Wallet, client: &l2::Client) -> crate::Result<u64> {
        // 1. Inspect the receipt to collect the amount we owe the adaptor, but only trust squash
        //    and cheques that verify.
        let owed = if let Some(receipt) = client.receipt().await? {
            receipt.provably_owed(&consumer.verification_key(), &self.tag().into())
        } else {
            0
        };

        // 2. Regardless, always send a null squash to check if there's any unresolved state to
        //    settle with the server.
        let squash_response = client.squash(SquashBody::default()).await?;
        client.sync(squash_response, true).await?;

        Ok(owed)
    }

    pub async fn get_quote(
        &self,
        client: &l2::Client,
        invoice: &str,
    ) -> crate::Result<wasm::Quote> {
        Ok(client.quote(invoice).await?.into())
    }

    pub async fn pay(
        &self,
        client: &l2::Client,
        invoice: &str,
        quote: &wasm::Quote,
    ) -> crate::Result<()> {
        let squash_status = client.pay(invoice, quote.deref()).await?;
        client.sync(squash_status, true).await?;
        Ok(())
    }

    /// Find currently opened channels.
    #[wasm_bindgen(js_name = "opened")]
    pub async fn opened(connector: &Connector, consumer: &Wallet) -> crate::Result<Vec<Self>> {
        Ok(
            l1::Client::new(connector.deref(), consumer.signing_key().into())
                .opened_channels(consumer.stake_credential().as_deref())
                .await?
                .map(Self)
                .collect(),
        )
    }

    #[wasm_bindgen(js_name = "open")]
    pub async fn open(
        connector: &Connector,
        consumer: &Wallet,
        script_deployment_address: &wasm::ShelleyAddress,
        tag: &wasm::Tag,
        adaptor_key: &wasm::VerificationKey,
        close_period_secs: u64,
        amount: u64,
    ) -> crate::Result<wasm::Hash32> {
        let opens = vec![OpenIntent {
            tag: tag.clone().into(),
            sub_vkey: (*adaptor_key).into(),
            close_period: Duration::from_secs(close_period_secs),
            amount,
        }];

        Ok(
            l1::Client::new(connector.deref(), consumer.signing_key().into())
                .execute(
                    &consumer.signing_key(),
                    consumer.stake_credential().as_deref(),
                    opens,
                    Default::default(),
                    &script_deployment_address.clone().into(),
                )
                .await?
                .into(),
        )
    }

    #[wasm_bindgen(js_name = "close")]
    pub async fn close(
        connector: &Connector,
        consumer: &Wallet,
        script_deployment_address: &wasm::ShelleyAddress,
        tag: &wasm::Tag,
    ) -> crate::Result<wasm::Hash32> {
        Ok(
            l1::Client::new(connector.deref(), consumer.signing_key().into())
                .execute(
                    &consumer.signing_key(),
                    consumer.stake_credential().as_deref(),
                    vec![],
                    From::from([(tag.clone().into(), Intent::Close)]),
                    &script_deployment_address.clone().into(),
                )
                .await?
                .into(),
        )
    }
}

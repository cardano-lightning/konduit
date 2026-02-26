use crate::{
    CardanoConnector as _, Client, Connector, Wallet,
    core::{
        Bounds, ChannelOutput, Credential, Duration, Input, NetworkParameters, Output, SquashBody,
        Stage, VerificationKey,
        consumer::{Intent, OpenIntent},
        filter_channels, wasm,
    },
};
use std::{borrow::Borrow, collections::BTreeMap, ops::Deref};
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

        self.0.amount + subbed + 2_000_000
    }

    #[wasm_bindgen(js_name = "receipt")]
    pub async fn receipt(&self, consumer: &Wallet, client: &Client) -> crate::Result<u64> {
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

    pub async fn get_quote(&self, client: &Client, invoice: &str) -> crate::Result<wasm::Quote> {
        Ok(client.quote(invoice).await?.into())
    }

    pub async fn pay(
        &self,
        client: &Client,
        invoice: &str,
        quote: &wasm::Quote,
    ) -> crate::Result<()> {
        let squash_status = client.pay(invoice, quote.deref()).await?;
        client.sync(squash_status, true).await?;
        Ok(())
    }

    /// Find currently opened channels.
    #[wasm_bindgen(js_name = "opened")]
    pub async fn opened(
        connector: &Connector,
        consumer: &Wallet,
        konduit_validator: &wasm::Credential,
    ) -> crate::Result<Vec<Self>> {
        let consumer_key = consumer.verification_key();
        let stake_credential = consumer.stake_credential();

        let utxos_konduit =
            utxos_at_address(connector, konduit_validator, stake_credential.as_deref()).await?;

        Ok(filter_channels(&utxos_konduit.collect(), |channel| {
            &channel.constants.add_vkey == consumer_key.borrow()
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
        connector: &Connector,
        consumer: &Wallet,
        script_deployment_address: &wasm::ShelleyAddress,
        tag: &wasm::Tag,
        adaptor_key: &wasm::VerificationKey,
        close_period_secs: u64,
        amount: u64,
    ) -> crate::Result<wasm::Hash32> {
        let network_parameters = NetworkParameters {
            network_id: connector.network_id().into(),
            protocol_parameters: connector.protocol_parameters().await?,
        };

        let consumer_key = consumer.verification_key().into();

        let opens = vec![OpenIntent {
            tag: tag.clone().into(),
            sub_vkey: (*adaptor_key).into(),
            close_period: Duration::from_secs(close_period_secs),
            amount,
        }];

        let intents = BTreeMap::new();

        let utxos_consumer = utxos_at_address(
            connector,
            &consumer.payment_credential(),
            consumer.stake_credential().as_deref(),
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
        connector: &Connector,
        consumer: &Wallet,
        konduit_validator: &wasm::Credential,
        script_deployment_address: &wasm::ShelleyAddress,
        tag: &wasm::Tag,
    ) -> crate::Result<wasm::Hash32> {
        let network_parameters = NetworkParameters {
            network_id: connector.network_id().into(),
            protocol_parameters: connector.protocol_parameters().await?,
        };

        let consumer_key: VerificationKey = consumer.verification_key().into();

        let opens = vec![];

        let intents = BTreeMap::from([(tag.clone().into(), Intent::Close)]);

        let stake_credential = consumer.stake_credential();

        let utxos_konduit =
            utxos_at_address(connector, konduit_validator, stake_credential.as_deref()).await?;

        let utxos_consumer = utxos_at_address(
            connector,
            &Credential::from(consumer.verification_key().deref()),
            stake_credential.as_deref(),
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

async fn utxos_at_address(
    connector: &Connector,
    payment_credential: &Credential,
    stake_credential_opt: Option<&Credential>,
) -> crate::Result<impl Iterator<Item = (Input, Output)> + use<>> {
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

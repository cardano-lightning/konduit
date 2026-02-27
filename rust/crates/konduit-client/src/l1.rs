use crate::core::{
    Address, Bounds, ChannelOutput, Credential, Hash, Input, KONDUIT_VALIDATOR, NetworkId,
    NetworkParameters, Output, SigningKey, Stage, Tag,
    address::kind,
    consumer::{self, Intent, OpenIntent},
    filter_channels,
};
use cardano_connector::CardanoConnector;
use std::collections::BTreeMap;

pub struct Client<'connector, Connector: CardanoConnector> {
    connector: &'connector Connector,
    consumer: SigningKey,
}

impl<'connector, Connector: CardanoConnector> Client<'connector, Connector> {
    pub fn new(connector: &'connector Connector, consumer: SigningKey) -> Self {
        Client {
            connector,
            consumer,
        }
    }

    /// Check the chain for opened channels that match the client's credentials, regardless of the
    /// adaptor they're configured with.
    pub async fn opened_channels(
        &self,
        stake_credential: Option<&Credential>,
    ) -> anyhow::Result<impl Iterator<Item = ChannelOutput>> {
        let consumer = self.consumer.to_verification_key();

        let utxos_konduit = all_utxos_at(
            self.connector,
            &KONDUIT_VALIDATOR.to_credential(),
            stake_credential,
        )
        .await?;

        Ok(filter_channels(&utxos_konduit.collect(), |channel| {
            channel.constants.add_vkey == consumer
        })
        .into_iter()
        .filter_map(|(_, channel)| match channel.stage {
            Stage::Opened { .. } => Some(channel),
            Stage::Closed { .. } | Stage::Responded { .. } => None,
        }))
    }

    /// Execute the given intents on any compatible channel owned by the client's credentials.
    pub async fn execute(
        &self,
        wallet_sk: &SigningKey,
        stake_credential: Option<&Credential>,
        opens: Vec<OpenIntent>,
        intents: BTreeMap<Tag, Intent>,
        script_deployment_address: &Address<kind::Shelley>,
    ) -> anyhow::Result<Hash<32>> {
        let network_parameters = NetworkParameters {
            network_id: NetworkId::from(self.connector.network()),
            protocol_parameters: self.connector.protocol_parameters().await?,
        };

        let consumer_sk = &self.consumer;
        let consumer_vk = self.consumer.to_verification_key();

        let wallet_vk = wallet_sk.to_verification_key();

        let utxos_script_ref = self
            .connector
            .utxos_at(
                &script_deployment_address.payment(),
                script_deployment_address.delegation().as_ref(),
            )
            .await?;

        let utxos_konduit = if !intents.is_empty() {
            Box::new(
                all_utxos_at(
                    self.connector,
                    &KONDUIT_VALIDATOR.to_credential(),
                    stake_credential,
                )
                .await?,
            )
        } else {
            Box::new(std::iter::empty()) as Box<dyn Iterator<Item = (Input, Output)>>
        };

        let utxos_consumer = all_utxos_at(
            self.connector,
            &Credential::from(&consumer_vk),
            stake_credential,
        )
        .await?;

        let utxos_wallet = if wallet_vk != consumer_vk {
            Box::new(
                all_utxos_at(
                    self.connector,
                    &Credential::from(&wallet_vk),
                    stake_credential,
                )
                .await?,
            )
        } else {
            Box::new(std::iter::empty()) as Box<dyn Iterator<Item = (Input, Output)>>
        };

        let mut tx = consumer::tx(
            &network_parameters,
            &wallet_vk,
            opens,
            intents,
            &std::iter::empty()
                .chain(utxos_script_ref)
                .chain(utxos_konduit)
                .chain(utxos_consumer)
                .chain(utxos_wallet)
                .collect(),
            Bounds::twenty_mins(),
        )?;

        tx.sign_with(|msg| (consumer_vk, consumer_sk.sign(msg)));

        if wallet_vk != consumer_vk {
            tx.sign_with(|msg| (wallet_vk, wallet_sk.sign(msg)));
        }

        self.connector.submit(&tx).await?;

        Ok(tx.id())
    }
}

/// A version of 'utxos_at' that fetches utxos at the payment credential, without delegation
/// credentials and with delegation credentials if set.
async fn all_utxos_at<Connector: CardanoConnector>(
    connector: &Connector,
    payment_credential: &Credential,
    stake_credential_opt: Option<&Credential>,
) -> anyhow::Result<impl Iterator<Item = (Input, Output)> + use<Connector>> {
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

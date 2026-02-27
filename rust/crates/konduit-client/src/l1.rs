use crate::{
    CardanoConnector,
    core::{
        Address, Bounds, ChannelOutput, Credential, Hash, Input, KONDUIT_VALIDATOR, NetworkId,
        NetworkParameters, Output, SigningKey, Stage, Tag,
        address::kind,
        consumer::{self, Intent, OpenIntent},
        filter_channels,
    },
};
use std::collections::BTreeMap;

pub struct Client<Connector: CardanoConnector> {
    connector: Connector,
    consumer: SigningKey,
    script_deployment_address: Address<kind::Shelley>,
    stake_credential: Option<Credential>,
}

impl<Connector: CardanoConnector> Client<Connector> {
    pub fn new(
        connector: Connector,
        consumer: SigningKey,
        script_deployment_address: Address<kind::Shelley>,
        stake_credential: Option<Credential>,
    ) -> Self {
        Client {
            connector,
            consumer,
            script_deployment_address,
            stake_credential,
        }
    }

    /// Set or reset the stake credential associated with the consumer's signing key.
    pub fn set_stake_credential(&mut self, stake_credential: Option<Credential>) {
        self.stake_credential = stake_credential;
    }

    /// Check the chain for opened channels that match the client's credentials, regardless of the
    /// adaptor they're configured with.
    pub async fn opened_channels(&self) -> anyhow::Result<Vec<ChannelOutput>> {
        let consumer = self.consumer.to_verification_key();

        let utxos_konduit = all_utxos_at(
            &self.connector,
            &KONDUIT_VALIDATOR.to_credential(),
            self.stake_credential.as_ref(),
        )
        .await?;

        Ok(filter_channels(&utxos_konduit.collect(), |channel| {
            channel.constants.add_vkey == consumer
        })
        .into_iter()
        .filter_map(|(_, channel)| match channel.stage {
            Stage::Opened { .. } => Some(channel),
            Stage::Closed { .. } | Stage::Responded { .. } => None,
        })
        .collect())
    }

    /// Execute the given intents on any compatible channel owned by the client's credentials.
    pub async fn execute(
        &self,
        wallet_sk: &SigningKey,
        opens: Vec<OpenIntent>,
        intents: BTreeMap<Tag, Intent>,
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
                &self.script_deployment_address.payment(),
                self.script_deployment_address.delegation().as_ref(),
            )
            .await?;

        let utxos_konduit = if !intents.is_empty() {
            Box::new(
                all_utxos_at(
                    &self.connector,
                    &KONDUIT_VALIDATOR.to_credential(),
                    self.stake_credential.as_ref(),
                )
                .await?,
            )
        } else {
            Box::new(std::iter::empty()) as Box<dyn Iterator<Item = (Input, Output)>>
        };

        let utxos_consumer = all_utxos_at(
            &self.connector,
            &Credential::from(&consumer_vk),
            self.stake_credential.as_ref(),
        )
        .await?;

        let utxos_wallet = if wallet_vk != consumer_vk {
            Box::new(
                all_utxos_at(
                    &self.connector,
                    &Credential::from(&wallet_vk),
                    self.stake_credential.as_ref(),
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

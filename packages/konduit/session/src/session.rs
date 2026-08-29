use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

use cardano_connector::CardanoConnector;
use cardano_sdk::{
    Credential, Hash, Input, Output, Transaction, transaction::state::ReadyForSigning,
};
use cardano_wallet::Wallet;
use konduit_tx2::{Channel, Interval, KONDUIT_VALIDATOR, StagedTx, konduit_address};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("session: {0}")]
    Cardano(#[from] cardano_session::session::Error),
}

pub struct Session<C, W> {
    cardano: cardano_session::Session<C, W>,
}

impl<C: CardanoConnector, W: Wallet> Deref for Session<C, W> {
    type Target = cardano_session::Session<C, W>;
    fn deref(&self) -> &Self::Target {
        &self.cardano
    }
}

impl<C: CardanoConnector, W: Wallet> DerefMut for Session<C, W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cardano
    }
}

impl<C: CardanoConnector, W: Wallet> Session<C, W> {
    pub fn new(cardano: cardano_session::Session<C, W>) -> Result<Self, Error> {
        let mut session = Self { cardano };
        session.track(None)?;
        Ok(session)
    }

    /// Tracks the konduit script address for `delegation` (`None` for the
    /// delegation-free address), under a label derived from it - there's
    /// only one meaningful address per delegation, so there's nothing for
    /// a caller to usefully name it themselves.
    pub fn track(&mut self, delegation: Option<&Credential>) -> Result<(), Error> {
        let address = konduit_address(self.cardano.network_id(), delegation);
        let label = match delegation {
            Some(credential) => format!("konduit-{credential}"),
            None => "konduit".to_string(),
        };
        Ok(self.cardano.track(label, address)?)
    }

    /// Stops tracking the konduit script address for `delegation`.
    pub fn untrack(&mut self, delegation: Option<&Credential>) -> Result<(), Error> {
        let address = konduit_address(self.cardano.network_id(), delegation);
        Ok(self.cardano.forget(&address)?)
    }

    /// Uploads the konduit validator script and tracks its
    /// (delegation-free) script address.
    pub async fn upload(&mut self) -> Result<Hash<32>, Error> {
        let id = self
            .cardano
            .upload(KONDUIT_VALIDATOR.script.clone())
            .await?;
        self.track(None)?;
        Ok(id)
    }

    pub async fn teardown(&mut self) -> Result<Hash<32>, Error> {
        Ok(self.cardano.teardown(&KONDUIT_VALIDATOR.hash).await?)
    }

    pub fn network_parameters(&self) -> konduit_tx2::NetworkParameters {
        let network_id = self.cardano.network_id();
        let protocol_parameters = self.cardano.protocol_parameters().clone();
        konduit_tx2::NetworkParameters {
            network_id,
            protocol_parameters,
        }
    }

    /// FIXME :: upstream/ fix. tx builder needs a single BTreeMap to borrow.
    pub fn utxos(&self) -> BTreeMap<Input, Output> {
        self.cardano
            .tip()
            .addresses()
            .filter_map(|address| self.cardano.utxos_at(address))
            .flatten()
            .map(|(input, output)| (input.clone(), output.clone()))
            .collect()
    }

    pub fn channels(&self) -> BTreeMap<Input, Channel> {
        let konduit_credential = Credential::from_script(KONDUIT_VALIDATOR.hash);
        self.cardano
            .tip()
            .addresses()
            .filter(|address| address.payment() == konduit_credential)
            .filter_map(|address| self.cardano.utxos_at(address))
            .flat_map(|utxos| {
                utxos
                    .iter()
                    .filter_map(|(input, output)| {
                        Channel::try_from(output)
                            .ok()
                            .map(|channel| (input.clone(), channel))
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    pub fn stage_tx(&self, window: Interval) -> StagedTx {
        let network_id = self.cardano.network_id();
        StagedTx::new(network_id, window, self.channels())
    }

    pub fn reference_utxo(&self) -> Option<(Input, Output)> {
        self.cardano.ref_script(&KONDUIT_VALIDATOR.hash)
    }

    pub fn build(
        &self,
        mut staged_tx: StagedTx,
    ) -> Result<Transaction<ReadyForSigning>, konduit_tx2::staged_tx::BuildError> {
        staged_tx.build(
            &self.utxos(),
            &self.network_parameters(),
            self.reference_utxo().as_ref(),
            self.change_address().into(),
            &self.fuel(),
        )
    }
}

use crate::{
    Cardano, Signer,
    core::{Address, Credential, Hash, KONDUIT_VALIDATOR, NetworkParameters, address::kind},
};
use cardano_connector::CardanoConnector;
use konduit_tx::{ChannelUtxo, Open, SteppedUtxos, Utxo, Utxos, find_reference_script};
use std::collections::BTreeMap;

pub struct L1 {
    connector: Cardano,
    signer: Signer,
    fuel_address: Address<kind::Shelley>,
    network_parameters: NetworkParameters,
    reference_script: Utxo,
    fuel: Utxos,
    channels: Vec<ChannelUtxo>,
    stepped: SteppedUtxos,
}

impl L1 {
    pub async fn new(
        connector: Cardano,
        signer: Signer,
        fuel_address: Address<kind::Shelley>,
        host_address: Address<kind::Shelley>,
    ) -> anyhow::Result<Self> {
        let network_id = connector.network().into();
        let protocol_parameters = connector.protocol_parameters().await?;
        let network_parameters = NetworkParameters {
            network_id,
            protocol_parameters,
        };
        println!("{}", fuel_address);
        println!("{}", host_address);
        let reference_script = find_reference_script(
            &connector
                .utxos_at(&host_address.payment(), host_address.delegation().as_ref())
                .await?,
        )
        .expect("Reference script required, but none found");
        Ok(Self {
            connector,
            signer,
            fuel_address,
            network_parameters,
            reference_script,
            fuel: BTreeMap::new(),
            channels: Vec::new(),
            stepped: SteppedUtxos::from(vec![]),
        })
    }

    /// Returns true if anything updated.
    pub async fn sync_wallet(&mut self) -> anyhow::Result<bool> {
        let credential = self.fuel_address.payment();
        let fuel = self.connector.utxos_at(&credential, None).await?;
        if fuel == self.fuel {
            Ok(false)
        } else {
            self.fuel = fuel;
            Ok(true)
        }
    }

    /// Returns true if anything updated.
    pub async fn sync_channels(&mut self) -> anyhow::Result<bool> {
        let credential = Credential::from_script(KONDUIT_VALIDATOR.hash);
        let channels = self
            .connector
            .utxos_at(&credential, None)
            .await?
            .into_iter()
            .filter_map(|u| ChannelUtxo::try_from(u).ok())
            .collect();
        if channels == self.channels {
            Ok(false)
        } else {
            self.channels = channels;
            Ok(true)
        }
    }

    pub fn info(&self) -> String {
        format!("CHANNELS : {}, FUEL : {}", self.channels.len(), self.fuel.iter().map(|x| x.1.value().lovelace()).sum::<u64>())
    }

    /// Returns true if anything updated.
    pub async fn sync(&mut self) -> anyhow::Result<bool> {
        Ok(self.sync_wallet().await? || self.sync_channels().await?)
    }

    pub async fn tx(&mut self, opens: Vec<Open>) -> anyhow::Result<Hash<32>> {
        let steppeds = self.stepped.to_owned();
        self.stepped = SteppedUtxos::from(vec![]);
        let signers = steppeds.signers();
        let mut tx = konduit_tx::tx::tx(
            &self.network_parameters,
            Some(&self.reference_script),
            self.fuel_address.clone().into(),
            steppeds,
            opens,
            &self.fuel,
        )
        .inspect_err(|err| {
            eprintln!("{}", err);
            todo!("Handle failure gracefully")
        })?;

        let fuel_vkh = self
            .fuel_address
            .payment()
            .as_key()
            .expect("Fuel address must by a payment key");
        tx.sign_with(self.signer.get(&fuel_vkh).expect("Fuel key required"));
        for signer in signers {
            tx.sign_with(
                self.signer
                    .get(&Hash::<28>::new(signer))
                    .expect(format!("Signer key required {}", signer).as_str()),
            );
        }
        self.connector.submit(&tx).await?;
        Ok(tx.id())
    }
}

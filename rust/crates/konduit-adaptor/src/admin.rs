use crate::models::{L1Channel, TipBody};
use cardano_connect::CardanoConnect;
use cardano_connect_blockfrost::Blockfrost;
use cardano_tx_builder::{Credential, Datum, Hash, Input, Output, SigningKey, VerificationKey};
use konduit_data::{Duration, Keytag};
use std::{collections::BTreeMap, sync::Arc};

use crate::{db, info::Info};

pub const MIN_ADA_BUFFER: u64 = 2_000_000;

#[derive(Clone)]
pub struct Admin {
    close_period: Duration,
    connector: Arc<Blockfrost>,
    db: Arc<dyn db::DbInterface + Send + Sync + 'static>,
    max_tag_length: usize,
    #[allow(dead_code)]
    script_utxo: (Input, Output),
    script_hash: Hash<28>,
    skey: SigningKey,
}

fn guard(cond: bool) -> Option<()> {
    if cond { Some(()) } else { None }
}

async fn fetch_script_utxo(
    connector: Arc<Blockfrost>,
    deployer_vkey: VerificationKey,
    script_hash: Hash<28>,
) -> anyhow::Result<(Input, Output)> {
    let deployed_by = Credential::from_key(Hash::<28>::new(deployer_vkey));
    let deployer_utxos = connector.utxos_at(&deployed_by, None).await?;
    deployer_utxos
        .into_iter()
        .find(|(_input, output)| {
            let output_script_hash = output.script().map(Hash::<28>::from);
            output_script_hash == Some(script_hash)
        })
        .ok_or_else(|| anyhow::anyhow!("could not find konduit script UTXO"))
}

impl Admin {
    pub async fn new(
        connector: Arc<Blockfrost>,
        db: Arc<dyn db::DbInterface + Send + Sync + 'static>,
        info: Arc<Info>,
        skey: SigningKey,
    ) -> anyhow::Result<Admin> {
        let deployer_vkey = info.deployer_vkey;
        let script_hash = info.script_hash;
        let script_utxo = fetch_script_utxo(connector.clone(), deployer_vkey, script_hash).await?;
        Ok(Self {
            close_period: info.close_period,
            connector,
            db,
            max_tag_length: info.max_tag_length,
            script_utxo,
            script_hash: info.script_hash,
            skey,
        })
    }

    fn parse_channel_output(&self, output: Output) -> Option<(Keytag, L1Channel)> {
        let datum = output.datum()?;
        match datum {
            Datum::Inline(plutus_data) => {
                let datum = konduit_data::Datum::try_from(plutus_data).ok()?;
                guard(datum.own_hash == self.script_hash)?;
                guard(output.value().assets().is_empty())?;
                let adaptor_verification_key = VerificationKey::from(&self.skey);
                guard(datum.constants.sub_vkey == adaptor_verification_key)?;
                guard(datum.constants.close_period >= self.close_period)?;
                guard({
                    let bytes = <Vec<u8>>::from(&datum.constants.tag);
                    bytes.len() <= self.max_tag_length
                })?;
                let keytag = Keytag::new(datum.constants.add_vkey, datum.constants.tag);
                let res = (
                    keytag,
                    L1Channel {
                        stage: datum.stage,
                        amount: output
                            .value()
                            .lovelace()
                            .checked_sub(MIN_ADA_BUFFER)
                            .unwrap_or(0),
                    },
                );
                Some(res)
            }
            Datum::Hash(_) => None,
        }
    }

    async fn fetch_tip(&self) -> anyhow::Result<TipBody> {
        let script_credential = Credential::from_script(self.script_hash);
        let channel_utxos = self.connector.utxos_at(&script_credential, None).await?;
        let channels = channel_utxos
            .into_iter()
            .filter_map(|(_, output)| self.parse_channel_output(output));

        let mut tip_body = BTreeMap::new();
        for (keytag, l1_channel) in channels {
            tip_body
                .entry(keytag)
                .or_insert_with(Vec::new)
                .push(l1_channel);
        }
        Ok(tip_body)
    }

    pub async fn sync(&self) -> Result<(), anyhow::Error> {
        let tip = self.fetch_tip().await?;
        self.db.update_l1s(tip).await?;
        // sub(s)...
        Ok(())
    }
}

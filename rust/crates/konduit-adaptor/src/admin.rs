use crate::models::{L1Channel, TipBody};
use cardano_connect::CardanoConnect;
use cardano_connect_blockfrost::Blockfrost;
use cardano_tx_builder::{Credential, Datum, Hash, Input, Output, SigningKey, VerificationKey};
use konduit_data::{Duration, Keytag};
use std::{collections::BTreeMap, sync::Arc};

use crate::{db, info::Info};

#[derive(Clone)]
pub struct Admin {
    close_period: Duration,
    connector: Arc<Blockfrost>,
    db: Arc<dyn db::DbInterface + Send + Sync + 'static>,
    max_tag_length: usize,
    published_by: Credential,
    script_hash: Hash<28>,
    skey: SigningKey,
}

fn guard(cond: bool) -> Option<()> {
    if cond { Some(()) } else { None }
}

impl Admin {
    pub fn new(
        connector: Arc<Blockfrost>,
        db: Arc<dyn db::DbInterface + Send + Sync + 'static>,
        info: Arc<Info>,
        skey: SigningKey,
    ) -> Self {
        let publisher_vkey = info.publisher_vkey.clone();
        let published_by = Credential::from_key(Hash::<28>::new(publisher_vkey));

        Self {
            close_period: info.close_period.clone(),
            connector,
            db,
            max_tag_length: info.max_tag_length,
            published_by,
            script_hash: info.script_hash.clone(),
            skey,
        }
    }

    // I can not pre cache that because Output contains Rc which I had problem moving into async
    // closure which we want to do.
    async fn fetch_script_utxo(&self) -> anyhow::Result<(Input, Output)> {
        let publisher_utxos = self.connector.utxos_at(&self.published_by, None).await?;
        publisher_utxos
            .into_iter()
            .find(|(_input, output)| {
                let script_hash = output.script().map(|script| Hash::<28>::from(script));
                script_hash == Some(self.script_hash)
            })
            .ok_or_else(|| anyhow::anyhow!("could not find konduit script UTXO"))
    }

    fn parse_channel_output(&self, output: Output) -> Option<(Keytag, L1Channel)> {
        let datum = output.datum()?;
        match datum {
            Datum::Inline(plutus_data) => {
                let datum = konduit_data::Datum::try_from(plutus_data).ok()?;
                guard(datum.own_hash == self.script_hash)?;
                guard(output.value().assets().is_empty())?;
                let adaptor_verification_key = VerificationKey::from(&self.skey);
                guard(datum.constants.sub_vkey == adaptor_verification_key.into())?;
                guard(datum.constants.close_period <= self.close_period)?;
                guard({
                    let bytes = <Vec<u8>>::from(&datum.constants.tag);
                    bytes.len() <= self.max_tag_length
                })?;
                let keytag = Keytag::new(adaptor_verification_key, datum.constants.tag);
                let res = (
                    keytag,
                    L1Channel {
                        stage: datum.stage,
                        amount: output.value().lovelace(),
                    },
                );
                Some(res)
            }
            Datum::Hash(_) => None,
        }
    }

    async fn fetch_tip(&self) -> anyhow::Result<TipBody> {
        let script_credential = Credential::from_script(self.script_hash.into());
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

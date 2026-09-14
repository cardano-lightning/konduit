//! Component responsible for setting, updating the `backing` of a channel.
//! The initial version:
//! - is naive in that it follows tip.
//! - ignores `focus` that should handle mimics.

use std::{collections::BTreeMap, sync::Arc};

use crate::{
    channel::{self, Backing},
    db,
};
use cardano_connector::CardanoConnector;
use cardano_sdk::{Credential, Input, Output};
use itertools::{EitherOrBoth::*, Itertools};
use konduit_tmp::{ChannelParameters, Keytag};
use konduit_tx::{ChannelUtxo, KONDUIT_VALIDATOR, to_verifying_key};

#[derive(Clone)]
pub struct Index<Connector: CardanoConnector + Send + Sync + 'static> {
    cardano: Arc<Connector>,
    db: Arc<db::Db>,
    channel_parameters: ChannelParameters,
    // FIXME :: should be used to select lineage.
    #[allow(dead_code)]
    focuses: BTreeMap<Keytag, Input>,
}

impl<Connector: CardanoConnector + Send + Sync + 'static> Index<Connector> {
    pub fn new(
        cardano: Arc<Connector>,
        db: Arc<db::Db>,
        channel_parameters: ChannelParameters,
    ) -> Self {
        Self {
            cardano,
            db,
            channel_parameters,
            focuses: Default::default(),
        }
    }

    fn backings(&self, utxos: &BTreeMap<Input, Output>) -> BTreeMap<Keytag, Vec<Backing>> {
        let close_period = self.channel_parameters.close_period;
        let tag_length = self.channel_parameters.tag_length;
        let own_vkey = to_verifying_key(self.channel_parameters.adaptor_key);

        let candidates = utxos.iter().filter_map(|u| {
            let u = ChannelUtxo::try_from(u).ok()?;
            let channel = u.data();
            let c = channel.constants();
            let opened = c.sub_vkey == own_vkey
                && c.close_period >= close_period
                && c.tag.len() <= tag_length
                && channel.stage().is_opened();
            opened.then(|| {
                Backing::try_from(channel)
                    .ok()
                    .map(|b| (channel.keytag(), b))
            })?
        });

        candidates.fold(BTreeMap::new(), |mut acc, (k, v)| {
            acc.entry(k).or_default().push(v);
            acc
        })
    }

    /// These should be considered confirmed utxos,
    /// acceptable to be treated as backings.
    async fn snapshot(&self) -> anyhow::Result<BTreeMap<Input, Output>> {
        let credential = Credential::from_script(KONDUIT_VALIDATOR.hash);
        let utxos = self.cardano.utxos_at(&credential, None).await?;
        Ok(utxos)
    }

    pub async fn sync(&self) -> Result<(), anyhow::Error> {
        let snapshot = self.snapshot().await?;
        let left = self.db.keys()?.into_iter();
        let right = self.backings(&snapshot).into_iter();

        for item in left.merge_join_by(right, |l, r| l.cmp(&r.0)) {
            let (k, v) = match item {
                Left(k) => (k, Vec::new()),
                Right((k, v)) => (k, v),
                Both(k, (_, v)) => (k, v),
            };
            self.db.upsert(&k, channel::upsert_retainers(v))?;
        }
        Ok(())
    }
}

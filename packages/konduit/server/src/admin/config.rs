use crate::common;
use cardano_sdk::{Address, SigningKey, VerificationKey, address::kind};
use konduit_data::{AssetCatalog, AssetId};
use konduit_tmp::ChannelParameters;
use konduit_tx::adaptor::AdaptorPreferences;

pub struct Config {
    pub wallet: SigningKey,
    pub channel_parameters: ChannelParameters,
    pub tx_preferences: AdaptorPreferences,
    pub host_address: Address<kind::Shelley>,
}

impl Config {
    pub fn from_args(
        common: common::Args,
        admin: super::Args,
        assets: &AssetCatalog,
    ) -> anyhow::Result<Self> {
        let common::Args {
            signing_key: wallet,
            close_period,
            tag_length,
            host_address,
            ..
        } = common;
        let adaptor_key = VerificationKey::from(&wallet);
        let channel_parameters = ChannelParameters {
            adaptor_key,
            close_period,
            tag_length,
        };
        let mut asset_minimums = std::collections::BTreeMap::<AssetId, (u64, u64)>::new();
        for value in admin.asset_minimum {
            let mut parts = value.split(':');
            let alias = parts.next().unwrap_or_default();
            let single = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("invalid asset minimum '{value}'"))?
                .parse()?;
            let total = parts
                .next()
                .ok_or_else(|| anyhow::anyhow!("invalid asset minimum '{value}'"))?
                .parse()?;
            if parts.next().is_some() {
                anyhow::bail!("invalid asset minimum '{value}'");
            }
            let definition = assets
                .by_alias(alias)
                .ok_or_else(|| anyhow::anyhow!("unknown asset alias '{alias}'"))?;
            if definition.asset == AssetId::Ada {
                anyhow::bail!("use --min-single/--min-total for Ada");
            }
            asset_minimums.insert(definition.asset.clone(), (single, total));
        }
        let tx_preferences = AdaptorPreferences {
            min_single: admin.min_single,
            min_total: admin.min_total,
            asset_minimums,
        };
        Ok(Self {
            wallet,
            channel_parameters,
            tx_preferences,
            host_address,
        })
    }
}

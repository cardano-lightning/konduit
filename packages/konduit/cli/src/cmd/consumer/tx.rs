use crate::config::consumer::Config;
use cardano_sdk::VerificationKey;
use konduit_client::l1;
use konduit_data::{AssetCatalog, AssetId, Duration, Tag};
use konduit_tx::consumer::{Intent, OpenIntent};
use std::{collections::BTreeMap, str};

/// Consumer tx. Can open, add, close, expire, elapse, and end.
/// Only open add and close need to be declared, the other steps are inferred from the context.
#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    /// Open channel: TAG,ADAPTOR_KEY,CLOSE_PERIOD,AMOUNT[,ASSET_ALIAS].
    /// Amount is an integer in displayed asset units; omitted alias means Ada.
    #[arg(
        long,
        value_names = ["TAG,ADAPTOR_KEY,CLOSE_PERIOD,AMOUNT[,ASSET_ALIAS]"]
    )]
    open: Vec<OpenArgs>,

    /// Add displayed whole asset units to a channel
    #[arg(long, value_names = ["TAG,AMOUNT"])]
    add: Vec<TagAmount>,

    /// Close channel
    #[arg(long, value_names = ["TAG"])]
    close: Vec<Tag>,
}

#[derive(Debug, Clone)]
pub struct OpenArgs {
    tag: Tag,
    sub_vkey: VerificationKey,
    close_period: Duration,
    amount: u64,
    asset_alias: String,
}

impl str::FromStr for OpenArgs {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.split(',').collect::<Vec<_>>();
        let (a, b, c, d, asset_alias) = match parts.as_slice() {
            [a, b, c, d] => (*a, *b, *c, *d, "ada"),
            [a, b, c, d, alias] => (*a, *b, *c, *d, *alias),
            _ => return Err(anyhow::anyhow!("Expected 4 or 5 args")),
        };
        Ok(Self {
            tag: a.parse()?,
            sub_vkey: b.parse()?,
            close_period: c.parse()?,
            amount: d.parse()?,
            asset_alias: asset_alias.to_owned(),
        })
    }
}

#[derive(Debug, Clone)]
struct TagAmount {
    pub tag: Tag,
    pub amount: u64,
}

impl str::FromStr for TagAmount {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let [a, b] = <[&str; 2]>::try_from(s.split(",").collect::<Vec<&str>>())
            .map_err(|_err| anyhow::anyhow!("Expected 2 args"))?;
        Ok(Self {
            tag: a.parse()?,
            amount: b.parse::<u64>()?,
        })
    }
}

impl OpenArgs {
    fn resolve(self, catalog: &AssetCatalog) -> anyhow::Result<OpenIntent> {
        let definition = catalog
            .by_alias(&self.asset_alias)
            .ok_or_else(|| anyhow::anyhow!("unknown asset alias '{}'", self.asset_alias))?;
        Ok(OpenIntent {
            tag: self.tag,
            sub_vkey: self.sub_vkey,
            close_period: self.close_period,
            amount: scale(self.amount, definition.decimals)?,
            asset: definition.asset.clone(),
        })
    }
}

fn scale(amount: u64, decimals: u8) -> anyhow::Result<u64> {
    let factor = 10_u64
        .checked_pow(decimals.into())
        .ok_or_else(|| anyhow::anyhow!("amount exceeds asset precision/range"))?;
    amount
        .checked_mul(factor)
        .ok_or_else(|| anyhow::anyhow!("amount exceeds asset precision/range"))
}

fn record_asset(
    assets: &mut BTreeMap<Tag, AssetId>,
    tag: Tag,
    asset: AssetId,
) -> anyhow::Result<()> {
    match assets.entry(tag) {
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(asset);
            Ok(())
        }
        std::collections::btree_map::Entry::Occupied(entry) if entry.get() == &asset => Ok(()),
        std::collections::btree_map::Entry::Occupied(entry) => {
            Err(anyhow::anyhow!("mixed assets for tag: {:?}", entry.key()))
        }
    }
}

impl Cmd {
    pub async fn run(self, config: &Config) -> anyhow::Result<()> {
        let catalog = AssetCatalog::load(config.asset_config.as_deref())?;
        let connector = config.connector.connector().await?;
        let client = l1::Client::new(&connector, &config.wallet);
        let Cmd { open, add, close } = self;

        let opens = open
            .into_iter()
            .map(|args| args.resolve(&catalog))
            .collect::<anyhow::Result<Vec<_>>>()?;

        let mut open_assets = BTreeMap::new();
        for open in &opens {
            record_asset(&mut open_assets, open.tag.clone(), open.asset.clone())?;
        }

        let channel_assets = if opens.is_empty() && add.is_empty() {
            BTreeMap::new()
        } else {
            client
                .channels(None)
                .await?
                .try_fold(BTreeMap::new(), |mut assets, channel| {
                    record_asset(
                        &mut assets,
                        channel.tag().clone(),
                        channel.constants().asset.clone(),
                    )?;
                    Ok::<_, anyhow::Error>(assets)
                })?
        };

        for (tag, asset) in &channel_assets {
            record_asset(&mut open_assets, tag.clone(), asset.clone())?;
        }
        let adds = add
            .into_iter()
            .map(|args| {
                let asset = channel_assets
                    .get(&args.tag)
                    .ok_or_else(|| anyhow::anyhow!("channel not found for add: {:?}", args.tag))?;
                let definition = catalog
                    .by_asset(asset)
                    .ok_or_else(|| anyhow::anyhow!("channel asset is not configured"))?;
                Ok((
                    args.tag,
                    Intent::Add {
                        amount: scale(args.amount, definition.decimals)?,
                        asset: asset.clone(),
                    },
                ))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let intents = adds
            .into_iter()
            .chain(close.into_iter().map(|tag| (tag, Intent::Close)))
            .collect::<BTreeMap<_, _>>();

        let id = client
            .execute(&config.wallet, None, opens, intents, &config.host_address)
            .await?;

        println!("\"{id}\"");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displayed_amount_scaling_is_checked() {
        assert_eq!(scale(100, 6).unwrap(), 100_000_000);
        assert_eq!(
            scale(u64::MAX, 6).unwrap_err().to_string(),
            "amount exceeds asset precision/range"
        );
        assert!(scale(1, 20).is_err());
    }

    #[test]
    fn open_rejects_unsupported_arity_before_field_parsing() {
        assert_eq!(
            "tag,key,period"
                .parse::<OpenArgs>()
                .unwrap_err()
                .to_string(),
            "Expected 4 or 5 args"
        );
        assert_eq!(
            "a,b,c,d,e,f".parse::<OpenArgs>().unwrap_err().to_string(),
            "Expected 4 or 5 args"
        );
    }

    #[test]
    fn mixed_assets_for_one_tag_are_rejected() {
        let tag = Tag::from(vec![1]);
        let mut assets = BTreeMap::new();
        record_asset(&mut assets, tag.clone(), AssetId::Ada).unwrap();
        assert!(
            record_asset(&mut assets, tag, AssetId::native([0; 28], vec![]).unwrap(),).is_err()
        );
    }
}

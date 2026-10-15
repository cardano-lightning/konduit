use crate::config::consumer::Config;
use cardano_sdk::VerificationKey;
use konduit_client::l1;
use konduit_data::{AssetCatalog, Duration, Tag};
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
    #[arg(long, value_names = ["TAG,AMOUNT,ASSET_ALIAS"])]
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
    pub asset_alias: String,
}

impl str::FromStr for TagAmount {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let [tag, amount, asset_alias] = <[&str; 3]>::try_from(s.split(',').collect::<Vec<_>>())
            .map_err(|_| anyhow::anyhow!("Expected 3 args"))?;
        Ok(Self {
            tag: tag.parse()?,
            amount: amount.parse::<u64>()?,
            asset_alias: asset_alias.to_owned(),
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

        let requested_adds = add
            .into_iter()
            .map(|args| {
                let definition = catalog
                    .by_alias(&args.asset_alias)
                    .ok_or_else(|| anyhow::anyhow!("unknown asset alias '{}'", args.asset_alias))?;
                Ok((
                    args.tag,
                    args.amount,
                    definition.asset.clone(),
                    definition.decimals,
                ))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        let requested_assets = requested_adds
            .iter()
            .map(|(tag, _, asset, _)| (tag.clone(), asset.clone()))
            .collect::<BTreeMap<_, _>>();
        let matched_assets = if requested_assets.is_empty() {
            BTreeMap::new()
        } else {
            client
                .channels(None)
                .await?
                .filter(|channel| {
                    requested_assets.get(channel.tag()) == Some(&channel.constants().asset)
                })
                .map(|channel| (channel.tag().clone(), channel.constants().asset.clone()))
                .collect::<BTreeMap<_, _>>()
        };

        let adds = requested_adds
            .into_iter()
            .map(|(tag, amount, asset, decimals)| {
                if matched_assets.get(&tag) != Some(&asset) {
                    anyhow::bail!("channel not found for add: {tag:?}");
                }
                Ok((
                    tag,
                    Intent::Add {
                        amount: scale(amount, decimals)?,
                        asset,
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
    fn add_requires_explicit_asset_alias() {
        assert!("deadbeef,2".parse::<TagAmount>().is_err());
        let parsed = "deadbeef,2,ada".parse::<TagAmount>().unwrap();
        assert_eq!(parsed.amount, 2);
        assert_eq!(parsed.asset_alias, "ada");
    }
}

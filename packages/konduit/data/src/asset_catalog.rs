use std::{collections::BTreeMap, fs, path::Path};

use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::AssetId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct AssetDefinition {
    #[n(0)]
    pub alias: String,
    #[n(1)]
    pub asset: AssetId,
    #[n(2)]
    pub decimals: u8,
    #[n(3)]
    pub pricing: Pricing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Pricing {
    #[n(0)]
    Ada,
    #[n(1)]
    UsdPeg,
    #[n(2)]
    CoinGecko {
        #[n(0)]
        coin_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetCatalog {
    aliases: BTreeMap<String, AssetDefinition>,
    assets: BTreeMap<AssetId, String>,
}

#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("failed to read asset catalog {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid asset catalog JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid asset alias '{0}'")]
    Alias(String),
    #[error("custom Ada definitions are not allowed")]
    CustomAda,
    #[error("asset decimals must be between 0 and 19")]
    Decimals,
    #[error("invalid CoinGecko coin id '{0}'")]
    CoinGeckoId(String),
    #[error("custom Pricing::Ada is not allowed")]
    CustomAdaPricing,
    #[error("duplicate asset alias '{0}'")]
    DuplicateAlias(String),
    #[error("duplicate asset identity")]
    DuplicateAsset,
}

impl AssetCatalog {
    pub fn builtins() -> Self {
        let mut catalog = Self {
            aliases: BTreeMap::new(),
            assets: BTreeMap::new(),
        };
        for definition in [
            AssetDefinition {
                alias: "ada".into(),
                asset: AssetId::Ada,
                decimals: 6,
                pricing: Pricing::Ada,
            },
            builtin(
                "usdm",
                "c48cbb3d5e57ed56e276bc45f99ab39abe94e6cd7ac39fb402da47ad",
                "0014df105553444d",
            ),
            builtin(
                "usdcx",
                "1f3aec8bfe7ea4fe14c5f121e2a92e301afe414147860d557cac7e34",
                "5553444378",
            ),
            builtin(
                "usda",
                "fe7c786ab321f41c654ef6c1af7b3250a613c24e4213e0425a7ae456",
                "55534441",
            ),
        ] {
            catalog.insert(definition).expect("valid built-in asset");
        }
        catalog
    }

    pub fn load(path: Option<&Path>) -> Result<Self, CatalogError> {
        let mut catalog = Self::builtins();
        let Some(path) = path else {
            return Ok(catalog);
        };
        let json = fs::read_to_string(path).map_err(|source| CatalogError::Read {
            path: path.display().to_string(),
            source,
        })?;
        catalog.extend_json(&json)?;
        Ok(catalog)
    }

    pub fn by_alias(&self, alias: &str) -> Option<&AssetDefinition> {
        if alias.bytes().all(|b| !b.is_ascii_uppercase()) {
            self.aliases.get(alias)
        } else {
            self.aliases.get(&alias.to_ascii_lowercase())
        }
    }

    pub fn by_asset(&self, asset: &AssetId) -> Option<&AssetDefinition> {
        self.assets
            .get(asset)
            .and_then(|alias| self.aliases.get(alias))
    }

    pub fn coin_gecko_feeds(&self) -> Vec<(String, String)> {
        self.aliases
            .values()
            .filter_map(|definition| match &definition.pricing {
                Pricing::CoinGecko { coin_id } => Some((definition.alias.clone(), coin_id.clone())),
                Pricing::Ada | Pricing::UsdPeg => None,
            })
            .collect()
    }

    #[cfg(feature = "json")]
    pub fn digest(&self) -> Result<String, serde_json::Error> {
        use cryptoxide::{digest::Digest, sha2::Sha256};

        let canonical = serde_json::to_vec(&self.aliases)?;
        let mut digest = [0; 32];
        let mut hasher = Sha256::new();
        hasher.input(&canonical);
        hasher.result(&mut digest);
        Ok(hex::encode(digest))
    }

    fn extend_json(&mut self, json: &str) -> Result<(), CatalogError> {
        for mut definition in serde_json::from_str::<Vec<AssetDefinition>>(json)? {
            definition.alias.make_ascii_lowercase();
            self.validate_custom(&definition)?;
            self.insert(definition)?;
        }
        Ok(())
    }

    fn validate_custom(&self, definition: &AssetDefinition) -> Result<(), CatalogError> {
        if definition.alias.is_empty()
            || definition.alias.len() > 32
            || !definition
                .alias
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-'))
        {
            return Err(CatalogError::Alias(definition.alias.clone()));
        }
        if definition.asset == AssetId::Ada {
            return Err(CatalogError::CustomAda);
        }
        if definition.decimals > 19 {
            return Err(CatalogError::Decimals);
        }
        match &definition.pricing {
            Pricing::Ada => return Err(CatalogError::CustomAdaPricing),
            Pricing::UsdPeg => {}
            Pricing::CoinGecko { coin_id }
                if coin_id.is_empty()
                    || coin_id.len() > 128
                    || !coin_id
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-') =>
            {
                return Err(CatalogError::CoinGeckoId(coin_id.clone()));
            }
            Pricing::CoinGecko { .. } => {}
        }
        Ok(())
    }

    fn insert(&mut self, definition: AssetDefinition) -> Result<(), CatalogError> {
        if self.aliases.contains_key(&definition.alias) {
            return Err(CatalogError::DuplicateAlias(definition.alias));
        }
        if self.assets.contains_key(&definition.asset) {
            return Err(CatalogError::DuplicateAsset);
        }
        self.assets
            .insert(definition.asset.clone(), definition.alias.clone());
        self.aliases.insert(definition.alias.clone(), definition);
        Ok(())
    }
}

fn builtin(alias: &str, policy_id: &str, asset_name: &str) -> AssetDefinition {
    AssetDefinition {
        alias: alias.into(),
        asset: AssetId::native(
            hex::decode(policy_id).unwrap().try_into().unwrap(),
            hex::decode(asset_name).unwrap(),
        )
        .unwrap(),
        decimals: 6,
        pricing: Pricing::UsdPeg,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SNEK: &str = r#"[{"alias":"snek","asset":{"kind":"native","policy_id":"00000000000000000000000000000000000000000000000000000000","asset_name":"534e454b"},"decimals":0,"pricing":{"kind":"coin_gecko","coin_id":"snek"}}]"#;

    #[test]
    fn exact_builtins_and_custom_example() {
        let mut catalog = AssetCatalog::builtins();
        for (alias, policy, name) in [
            (
                "usdm",
                "c48cbb3d5e57ed56e276bc45f99ab39abe94e6cd7ac39fb402da47ad",
                "0014df105553444d",
            ),
            (
                "usdcx",
                "1f3aec8bfe7ea4fe14c5f121e2a92e301afe414147860d557cac7e34",
                "5553444378",
            ),
            (
                "usda",
                "fe7c786ab321f41c654ef6c1af7b3250a613c24e4213e0425a7ae456",
                "55534441",
            ),
        ] {
            let definition = catalog.by_alias(alias).unwrap();
            assert_eq!(definition.decimals, 6);
            assert_eq!(
                definition.asset,
                AssetId::native(
                    hex::decode(policy).unwrap().try_into().unwrap(),
                    hex::decode(name).unwrap()
                )
                .unwrap()
            );
        }
        catalog.extend_json(SNEK).unwrap();
        assert_eq!(catalog.by_alias("SNEK").unwrap().decimals, 0);
        assert_eq!(
            catalog.coin_gecko_feeds(),
            vec![("snek".into(), "snek".into())]
        );
    }

    #[test]
    fn rejects_invalid_custom_definitions() {
        for json in [
            r#"[{"alias":"ada2","asset":{"kind":"ada"},"decimals":6,"pricing":{"kind":"usd_peg"}}]"#,
            r#"[{"alias":"x","asset":{"kind":"native","policy_id":"00","asset_name":""},"decimals":0,"pricing":{"kind":"usd_peg"}}]"#,
            r#"[{"alias":"x","asset":{"kind":"native","policy_id":"00000000000000000000000000000000000000000000000000000000","asset_name":"0"},"decimals":0,"pricing":{"kind":"usd_peg"}}]"#,
            r#"[{"alias":"x","asset":{"kind":"native","policy_id":"00000000000000000000000000000000000000000000000000000000","asset_name":""},"decimals":20,"pricing":{"kind":"usd_peg"}}]"#,
            r#"[{"alias":"x","asset":{"kind":"native","policy_id":"00000000000000000000000000000000000000000000000000000000","asset_name":""},"decimals":0,"pricing":{"kind":"coin_gecko","coin_id":""}}]"#,
            r#"[{"alias":"x","asset":{"kind":"native","policy_id":"00000000000000000000000000000000000000000000000000000000","asset_name":""},"decimals":0,"pricing":{"kind":"ada"}}]"#,
        ] {
            assert!(
                AssetCatalog::builtins().extend_json(json).is_err(),
                "{json}"
            );
        }
        let duplicate = format!("[{value},{value}]", value = &SNEK[1..SNEK.len() - 1]);
        assert!(AssetCatalog::builtins().extend_json(&duplicate).is_err());
    }

    #[test]
    fn catalog_snapshot_cbor_roundtrip() {
        let definition = AssetCatalog::builtins().by_alias("usdm").unwrap().clone();
        let bytes = minicbor::to_vec(&definition).unwrap();
        assert_eq!(definition, minicbor::decode(&bytes).unwrap());
    }

    #[test]
    fn catalog_digest_is_stable_and_content_sensitive() {
        let builtins = AssetCatalog::builtins();
        assert_eq!(
            builtins.digest().unwrap(),
            AssetCatalog::builtins().digest().unwrap()
        );
        let mut extended = AssetCatalog::builtins();
        extended.extend_json(SNEK).unwrap();
        assert_ne!(builtins.digest().unwrap(), extended.digest().unwrap());
    }
}

use cardano_sdk::{Hash, Value};
use konduit_data::AssetId;

use crate::MIN_ADA_BUFFER;

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("channel value has less than the minimum Ada reserve")]
    Reserve,
    #[error("Ada channel value contains native assets")]
    AdaWithNativeAssets,
    #[error("native channel value contains an unexpected asset")]
    UnexpectedAsset,
    #[error("native channel asset quantity must be positive")]
    NonPositiveQuantity,
}

pub(crate) fn amount(asset: &AssetId, value: &Value<u64>) -> Result<u64, Error> {
    if value.lovelace() < MIN_ADA_BUFFER {
        return Err(Error::Reserve);
    }
    match asset {
        AssetId::Ada => {
            if !value.assets().is_empty() {
                return Err(Error::AdaWithNativeAssets);
            }
            Ok(value.lovelace() - MIN_ADA_BUFFER)
        }
        AssetId::Native {
            policy_id,
            asset_name,
        } => {
            if value.assets().is_empty() {
                return Ok(0);
            }
            if value.assets().len() != 1 {
                return Err(Error::UnexpectedAsset);
            }
            let (policy, assets) = value.assets().first_key_value().unwrap();
            if assets.len() != 1 {
                return Err(Error::UnexpectedAsset);
            }
            let (name, quantity) = assets.first_key_value().unwrap();
            if policy.as_ref() != policy_id || name != asset_name {
                return Err(Error::UnexpectedAsset);
            }
            if *quantity == 0 {
                return Err(Error::NonPositiveQuantity);
            }
            Ok(*quantity)
        }
    }
}

pub(crate) fn value(asset: &AssetId, amount: u64) -> Value<u64> {
    match asset {
        AssetId::Ada => Value::new(
            amount
                .checked_add(MIN_ADA_BUFFER)
                .expect("channel amount exceeds u64"),
        ),
        AssetId::Native { .. } if amount == 0 => Value::new(MIN_ADA_BUFFER),
        AssetId::Native {
            policy_id,
            asset_name,
        } => Value::new(MIN_ADA_BUFFER)
            .with_assets([(Hash::<28>::from(*policy_id), [(asset_name, amount)])]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn native() -> AssetId {
        AssetId::native([1; 28], b"TOKEN".to_vec()).unwrap()
    }

    #[test]
    fn selected_asset_roundtrips_and_rejects_extras() {
        for (asset, quantity) in [(AssetId::Ada, 25), (native(), 25), (native(), 0)] {
            let value = value(&asset, quantity);
            assert_eq!(amount(&asset, &value), Ok(quantity));
        }

        let wrong =
            Value::new(MIN_ADA_BUFFER).with_assets([(Hash::<28>::from([2; 28]), [(b"TOKEN", 1)])]);
        assert_eq!(amount(&native(), &wrong), Err(Error::UnexpectedAsset));

        let extra = value(&native(), 1).with_assets([(Hash::<28>::from([2; 28]), [(b"OTHER", 1)])]);
        assert_eq!(amount(&native(), &extra), Err(Error::UnexpectedAsset));
    }
}

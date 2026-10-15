use crate::{core, wasm_proxy};
use anyhow::anyhow;
use wasm_bindgen::prelude::*;

wasm_proxy! {
    #[derive(Debug, Clone)]
    #[doc = "A channel's immutable Cardano asset identity"]
    AssetId => core::AssetId
}

#[wasm_bindgen]
impl AssetId {
    /// Construct the Ada identity.
    pub fn ada() -> AssetId {
        core::AssetId::Ada.into()
    }

    /// Construct a native asset from policy-id and asset-name hex.
    pub fn native(policy_id_hex: &str, asset_name_hex: &str) -> crate::wasm::Result<AssetId> {
        let policy_id = hex::decode(policy_id_hex)
            .map_err(|error| anyhow!("invalid policy id hex: {error}"))?;
        let policy_id = <[u8; 28]>::try_from(policy_id)
            .map_err(|_| anyhow!("policy id must be exactly 56 hex characters"))?;
        let asset_name = hex::decode(asset_name_hex)
            .map_err(|error| anyhow!("invalid asset name hex: {error}"))?;
        Ok(core::AssetId::native(policy_id, asset_name)
            .map_err(|error| anyhow!(error))?
            .into())
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn _wasm_to_string(&self) -> String {
        if self.0 == core::AssetId::Ada {
            "ada".to_owned()
        } else {
            format!(
                "{}.{}",
                hex::encode(self.0.policy_id().unwrap()),
                hex::encode(self.0.asset_name().unwrap())
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_forms_are_canonical() {
        assert_eq!(AssetId::ada()._wasm_to_string(), "ada");
        let native = AssetId::native(
            "00000000000000000000000000000000000000000000000000000000",
            "534e454b",
        )
        .ok()
        .unwrap();
        assert_eq!(
            native._wasm_to_string(),
            "00000000000000000000000000000000000000000000000000000000.534e454b"
        );
        assert!(AssetId::native("00", "").is_err());
        assert!(
            AssetId::native(
                "00000000000000000000000000000000000000000000000000000000",
                &"00".repeat(33),
            )
            .is_err()
        );
    }
}

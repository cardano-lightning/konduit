use anyhow::anyhow;
use cardano_tx_builder::{Hash, PlutusScript, PlutusVersion};
use std::{collections::BTreeMap, sync::LazyLock};

// TODO: embed the whole blueprint? blueprint_json
pub struct KonduitValidator {
    pub hash: Hash<28>,
    pub script: PlutusScript,
}

pub fn plutus_version_from_str(s: &str) -> anyhow::Result<PlutusVersion> {
    match s {
        "v1" => Ok(PlutusVersion::V1),
        "v2" => Ok(PlutusVersion::V2),
        "v3" => Ok(PlutusVersion::V3),
        _ => Err(anyhow!(
            "unknown plutus version version={s}; only v1, v2 and v3 are known"
        )),
    }
}

/// Get the validator blueprint at compile-time, and make the validator hash available on-demand.
pub static KONDUIT_VALIDATOR: LazyLock<KonduitValidator> = LazyLock::new(|| {
    let blueprint: BTreeMap<String, serde_json::Value> = serde_json::from_str(include_str!(
        concat!(std::env!("CARGO_MANIFEST_DIR"), "/plutus.json")
    ))
    .unwrap_or_else(|e| panic!("failed to parse blueprint: {e}"));

    let validator = blueprint
        .get("validators")
        .and_then(|value| value.as_array())
        .and_then(|validators| validators.first())
        .and_then(|value| value.as_object());

    let hash = validator
        .and_then(|validator| validator.get("hash"))
        .and_then(|value| value.as_str())
        .and_then(|s| Hash::try_from(s).ok())
        .unwrap_or_else(|| panic!("failed to extract validator's hash from blueprint"));

    let plutus_version = blueprint
        .get("preamble")
        .unwrap_or_else(|| panic!("failed to extract preamble from blueprint"))
        .get("plutusVersion")
        .and_then(|value| value.as_str())
        .and_then(|s| plutus_version_from_str(s).ok())
        .unwrap_or_else(|| panic!("failed to extract plutus version from blueprint"));

    // We should decode hex into Vec<u8>
    let script_bytes = validator
        .and_then(|validator| validator.get("compiledCode"))
        .and_then(|value| value.as_str())
        .and_then(|s| hex::decode(s).ok())
        .unwrap_or_else(|| panic!("failed to extract validator's compiled code from blueprint"));
    let script = PlutusScript::new(plutus_version, script_bytes);
    KonduitValidator { hash, script }
});

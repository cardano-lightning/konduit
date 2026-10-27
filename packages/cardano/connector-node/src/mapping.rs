use anyhow::{Context, anyhow};
use cardano_sdk::{
    Address, Hash, Input, NetworkId, Output, PlutusScript, PlutusVersion, Value, cbor,
};
use pallas_network::miniprotocols::localstate::queries_v16::{
    self, Addrs, PostAlonsoTransactionOutput, TransactionOutput, UTxOByAddress,
};
use std::collections::BTreeMap;

type NativeAsset = (Vec<u8>, u64);
type NativeAssets = Vec<(Hash<28>, Vec<NativeAsset>)>;

/// Translate a `cardano_sdk::Credential` (payment) — and optional delegation —
/// into the raw Shelley address bytes the local-state query protocol expects
/// as part of a `GetUTxOByAddress` request. The node matches the address bytes
/// exactly, so we must build the address with whatever delegation part the
/// caller is interested in; callers that don't care about stake leave it
/// `None` and get the bare payment address.
pub fn addrs_for_query(
    payment: &cardano_sdk::Credential,
    delegation: Option<&cardano_sdk::Credential>,
) -> anyhow::Result<Addrs> {
    let mut address = Address::new(NetworkId::TESTNET, payment.clone());
    if let Some(delegation) = delegation {
        address = address.with_delegation(delegation.clone());
    }
    let bytes: Vec<u8> = (&address).into();
    Ok(vec![bytes.into()])
}

pub fn map_utxos(utxos: UTxOByAddress) -> anyhow::Result<BTreeMap<Input, Output>> {
    let mut collected = BTreeMap::new();
    for (utxo, output) in utxos.utxo.iter() {
        let (input, output) =
            map_utxo(utxo, output).with_context(|| format!("failed to map UTxO for {utxo:?}"))?;
        collected.insert(input, output);
    }
    Ok(collected)
}

fn map_utxo(
    utxo: &queries_v16::UTxO,
    output: &TransactionOutput,
) -> anyhow::Result<(Input, Output)> {
    let index = u64::from(utxo.index);
    let input = Input::new(Hash::from(utxo.transaction_id), index);
    let output = match output {
        TransactionOutput::Current(post) => map_post_alonso(post)?,
        TransactionOutput::Legacy(legacy) => map_legacy(legacy)?,
    };
    Ok((input, output))
}

fn map_post_alonso(post: &PostAlonsoTransactionOutput) -> anyhow::Result<Output> {
    let address = Address::try_from(post.address.as_slice())
        .map_err(|error| anyhow!("invalid Shelley address: {error}"))?;
    let value = map_value(&post.amount)?;
    let mut output = Output::new(address, value);

    if let Some((_era, inline)) = post.inline_datum.as_ref() {
        let bytes: &[u8] = &inline.0;
        let datum = cbor::decode(bytes)
            .map_err(|error| anyhow!("failed to decode inline datum: {error}"))?;
        output = output.with_datum(datum);
    }

    if let Some(script_ref) = post.script_ref.as_ref() {
        let bytes: &[u8] = &script_ref.0;
        output = output.with_plutus_script(map_reference_script(bytes)?);
    }

    Ok(output)
}

fn map_legacy(legacy: &queries_v16::LegacyTransactionOutput) -> anyhow::Result<Output> {
    let address = Address::try_from(legacy.address.as_slice())
        .map_err(|error| anyhow!("invalid Shelley address: {error}"))?;
    let value = map_value(&legacy.amount)?;
    let mut output = Output::new(address, value);

    if let Some(hash) = legacy.datum_hash {
        output = output.with_datum_hash(Hash::from(hash));
    }

    Ok(output)
}

fn map_value(value: &queries_v16::Value) -> anyhow::Result<Value<u64>> {
    match value {
        queries_v16::Value::Coin(coin) => {
            let lovelace = u64::from(*coin);
            Ok(Value::new(lovelace))
        }
        queries_v16::Value::Multiasset(coin, multiasset) => {
            let lovelace = u64::from(*coin);
            let mut assets: NativeAssets = Vec::new();
            for (policy, inner) in multiasset.iter() {
                let mut entries: Vec<NativeAsset> = Vec::new();
                for (name, amount) in inner.iter() {
                    let quantity = u64::from(*amount);
                    entries.push((name.to_vec(), quantity));
                }
                assets.push((Hash::from(*policy), entries));
            }
            Ok(Value::new(lovelace).with_assets(assets))
        }
    }
}

fn map_reference_script(bytes: &[u8]) -> anyhow::Result<PlutusScript> {
    // Cardano encodes a reference script as a 2-element CBOR array where the
    // first element is the language tag (0 = native, 1 = V1, 2 = V2, 3 = V3)
    // and the second is the script payload. Decode that shape directly
    // rather than relying on minicbor's enum-tagged derive which uses map
    // encoding instead.
    use minicbor::data::Type;

    let mut decoder = minicbor::Decoder::new(bytes);
    decoder
        .array()
        .map_err(|error| anyhow!("reference script is not a CBOR array: {error}"))?;
    let tag = decoder
        .u32()
        .map_err(|error| anyhow!("reference script tag is not an unsigned integer: {error}"))?;
    let payload = decoder
        .bytes()
        .map_err(|error| anyhow!("reference script payload is not bytes: {error}"))?
        .to_vec();

    let trailing = decoder.datatype();
    let trailing_is_end = match &trailing {
        Ok(Type::Break) => true,
        Err(error) => error.is_end_of_input(),
        _ => false,
    };
    if !trailing_is_end {
        return Err(anyhow!(
            "reference script array has unexpected trailing elements: {trailing:?}"
        ));
    }

    match tag {
        1 => Ok(PlutusScript::new(PlutusVersion::V1, payload)),
        2 => Ok(PlutusScript::new(PlutusVersion::V2, payload)),
        3 => Ok(PlutusScript::new(PlutusVersion::V3, payload)),
        0 => Err(anyhow!(
            "found unsupported native reference script at output"
        )),
        other => Err(anyhow!(
            "unknown plutus version tag {other} in reference script"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cardano_sdk::Credential;

    fn key_credential(hex_seed: &str) -> Credential {
        let bytes: Vec<u8> = (0..Credential::DIGEST_SIZE)
            .map(|ix| hex_seed.as_bytes()[ix % hex_seed.len()])
            .collect();
        Credential::from_key(bytes.try_into().unwrap())
    }

    #[test]
    fn addrs_for_query_with_only_payment_returns_29_bytes() {
        let payment = key_credential("ab");
        let addrs = addrs_for_query(&payment, None).expect("addrs");
        assert_eq!(addrs.len(), 1);
        // Shelley addresses are 29 bytes for a key-only payment credential.
        assert_eq!(addrs[0].len(), 29);
    }

    #[test]
    fn addrs_for_query_with_delegation_returns_57_bytes() {
        let payment = key_credential("ab");
        let delegation = key_credential("cd");
        let addrs = addrs_for_query(&payment, Some(&delegation)).expect("addrs");
        assert_eq!(addrs.len(), 1);
        // Shelley base addresses are 57 bytes (29 payment + 28 delegation).
        assert_eq!(addrs[0].len(), 57);
    }
}

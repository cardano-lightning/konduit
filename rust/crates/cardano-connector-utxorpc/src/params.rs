use anyhow::{Context, anyhow};
use cardano_sdk::ProtocolParameters;
use num::rational::Ratio;
use utxorpc::spec::{cardano, query};

const BYRON_SLOT_LENGTH_SECS: u64 = 20;

pub async fn read(client: &mut utxorpc::CardanoQueryClient) -> anyhow::Result<ProtocolParameters> {
    let params = client
        .read_params()
        .await
        .map_err(|error| anyhow!(error))
        .context("failed to read protocol parameters from UTxO RPC")?;

    let era_summary = client
        .read_era_summary()
        .await
        .map_err(|error| anyhow!(error))
        .context("failed to read era summary from UTxO RPC")?;

    build(params, era_summary)
}

fn build(
    params: query::AnyChainParams,
    era_summary: query::read_era_summary_response::Summary,
) -> anyhow::Result<ProtocolParameters> {
    let params = match params.params {
        Some(query::any_chain_params::Params::Cardano(params)) => params,
        _ => {
            return Err(anyhow!(
                "UTxO RPC did not return Cardano protocol parameters"
            ));
        }
    };

    let query::read_era_summary_response::Summary::Cardano(summaries) = era_summary;

    let shelley = summaries
        .summaries
        .iter()
        .find(|era| era.name.eq_ignore_ascii_case("shelley"))
        .ok_or_else(|| anyhow!("UTxO RPC era summary missing Shelley era"))?;

    let start = shelley
        .start
        .as_ref()
        .ok_or_else(|| anyhow!("Shelley era summary missing start point"))?;

    let first_shelley_slot = start.slot;
    let shelley_start_time = start.time / 1000;
    let start_time = shelley_start_time
        .checked_sub(first_shelley_slot.saturating_mul(BYRON_SLOT_LENGTH_SECS))
        .ok_or_else(|| anyhow!("computed negative Cardano start time"))?;
    let prices = params
        .prices
        .ok_or_else(|| anyhow!("UTxO RPC protocol parameters missing execution prices"))?;
    let cost_models = params
        .cost_models
        .ok_or_else(|| anyhow!("UTxO RPC protocol parameters missing cost models"))?;

    let base = ProtocolParameters::default()
        .with_fee_per_byte(big_int_to_u64(
            params.min_fee_coefficient.as_ref(),
            "min_fee_coefficient",
        )?)
        .with_fee_constant(big_int_to_u64(
            params.min_fee_constant.as_ref(),
            "min_fee_constant",
        )?)
        .with_collateral_coefficient(params.collateral_percentage as f64 / 100.0)
        .with_referenced_scripts_base_fee_per_byte(rational_to_u64(
            params.min_fee_script_ref_cost_per_byte.as_ref(),
            "min_fee_script_ref_cost_per_byte",
        )?)
        .with_referenced_scripts_fee_multiplier(Ratio::new(12, 10))
        .with_referenced_scripts_fee_step_size(25_000)
        .with_execution_price_mem(rational_to_f64(
            prices.memory.as_ref(),
            "execution price memory",
        )?)
        .with_execution_price_cpu(rational_to_f64(
            prices.steps.as_ref(),
            "execution price steps",
        )?)
        .with_start_time(start_time)
        .with_first_shelley_slot(first_shelley_slot);

    let plutus_v3 = cost_models
        .plutus_v3
        .map(|model| model.values)
        .ok_or_else(|| anyhow!("UTxO RPC protocol parameters missing Plutus V3 cost model"))?;

    Ok(base.with_plutus_v3_cost_model(plutus_v3))
}

fn big_int_to_u64(value: Option<&cardano::BigInt>, label: &str) -> anyhow::Result<u64> {
    match value.and_then(|value| value.big_int.as_ref()) {
        Some(cardano::big_int::BigInt::Int(value)) if *value >= 0 => Ok(*value as u64),
        Some(cardano::big_int::BigInt::Int(value)) => {
            Err(anyhow!("invalid {label}: negative value {value}"))
        }
        Some(cardano::big_int::BigInt::BigUInt(bytes)) => bytes_to_u64(bytes, label),
        Some(cardano::big_int::BigInt::BigNInt(_)) => {
            Err(anyhow!("invalid {label}: negative big integer"))
        }
        None => Err(anyhow!("missing {label}")),
    }
}

fn rational_to_u64(value: Option<&cardano::RationalNumber>, label: &str) -> anyhow::Result<u64> {
    let value = rational_to_f64(value, label)?;
    if value < 0.0 {
        return Err(anyhow!("invalid {label}: negative ratio"));
    }
    Ok(value.round() as u64)
}

fn rational_to_f64(value: Option<&cardano::RationalNumber>, label: &str) -> anyhow::Result<f64> {
    let value = value.ok_or_else(|| anyhow!("missing {label}"))?;
    if value.denominator == 0 {
        return Err(anyhow!("invalid {label}: zero denominator"));
    }
    Ok(f64::from(value.numerator) / f64::from(value.denominator))
}

fn bytes_to_u64(bytes: &[u8], label: &str) -> anyhow::Result<u64> {
    if bytes.len() > 8 {
        return Err(anyhow!("invalid {label}: value exceeds u64"));
    }

    Ok(bytes
        .iter()
        .fold(0u64, |acc, byte| (acc << 8) | u64::from(*byte)))
}

#[cfg(test)]
mod tests {
    use super::{bytes_to_u64, rational_to_f64};
    use utxorpc::spec::cardano::RationalNumber;

    #[test]
    fn bytes_to_u64_rejects_values_larger_than_u64() {
        let error = bytes_to_u64(&[0; 9], "bigint").expect_err("overflow should fail");
        assert!(error.to_string().contains("value exceeds u64"));
    }

    #[test]
    fn rational_to_f64_rejects_zero_denominator() {
        let error = rational_to_f64(
            Some(&RationalNumber {
                numerator: 1,
                denominator: 0,
            }),
            "price",
        )
        .expect_err("zero denominator should fail");

        assert!(error.to_string().contains("zero denominator"));
    }
}

use crate::dolos;
use anyhow::{Context, anyhow};
use cardano_sdk::ProtocolParameters;
use num::rational::Ratio;
use std::collections::BTreeMap;
use utxorpc::spec::{cardano, query};

const BYRON_SLOT_LENGTH_SECS: u64 = 20;

pub async fn read(client: &mut utxorpc::CardanoQueryClient) -> anyhow::Result<ProtocolParameters> {
    let params = dolos(client.read_params())
        .await?
        .map_err(|error| anyhow!(error))
        .context("failed to read protocol parameters from UTxO RPC")?;

    let era_summary = dolos(client.read_era_summary())
        .await?
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

fn rational_to_decimal(
    value: Option<&cardano::RationalNumber>,
    label: &str,
) -> anyhow::Result<String> {
    let value = value.ok_or_else(|| anyhow!("missing {label}"))?;
    if value.numerator < 0 {
        return Err(anyhow!("invalid {label}: negative ratio"));
    }
    if value.denominator == 0 {
        return Err(anyhow!("invalid {label}: zero denominator"));
    }

    let ratio = Ratio::new(
        u64::from(value.numerator as u32),
        u64::from(value.denominator),
    );
    let mut denominator = *ratio.denom();
    while denominator % 2 == 0 {
        denominator /= 2;
    }
    while denominator % 5 == 0 {
        denominator /= 5;
    }
    if denominator != 1 {
        return Err(anyhow!("invalid {label}: non-terminating decimal"));
    }

    let denominator = *ratio.denom();
    let mut remainder = *ratio.numer() % denominator;
    let mut decimal = (*ratio.numer() / denominator).to_string();
    if remainder != 0 {
        decimal.push('.');
        while remainder != 0 {
            remainder *= 10;
            decimal.push(char::from(b'0' + (remainder / denominator) as u8));
            remainder %= denominator;
        }
    }
    Ok(decimal)
}

fn bytes_to_u64(bytes: &[u8], label: &str) -> anyhow::Result<u64> {
    if bytes.len() > 8 {
        return Err(anyhow!("invalid {label}: value exceeds u64"));
    }

    Ok(bytes
        .iter()
        .fold(0u64, |acc, byte| (acc << 8) | u64::from(*byte)))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BloxbeanPayload {
    pub min_fee_a: u64,
    pub min_fee_b: u64,
    pub max_tx_size: u64,
    pub key_deposit: u64,
    pub pool_deposit: u64,
    pub min_pool_cost: u64,
    pub protocol_major_ver: u32,
    pub protocol_minor_ver: u32,
    pub coins_per_utxo_size: u64,
    pub collateral_percent: u64,
    pub max_collateral_inputs: u64,
    pub price_mem: String,
    pub price_step: String,
    pub min_fee_ref_script_cost_per_byte: String,
    pub max_tx_ex_mem: u64,
    pub max_tx_ex_steps: u64,
    pub cost_models_raw: BTreeMap<String, Vec<i64>>,
}

pub async fn cardano_pparams(
    client: &mut utxorpc::CardanoQueryClient,
) -> anyhow::Result<cardano::PParams> {
    let params = dolos(client.read_params())
        .await?
        .map_err(|error| anyhow!(error))
        .context("failed to read protocol parameters from UTxO RPC")?;
    match params.params {
        Some(query::any_chain_params::Params::Cardano(params)) => Ok(params),
        _ => Err(anyhow!(
            "UTxO RPC did not return Cardano protocol parameters"
        )),
    }
}

pub fn bloxbean(params: &cardano::PParams) -> anyhow::Result<BloxbeanPayload> {
    let version = params
        .protocol_version
        .as_ref()
        .ok_or_else(|| anyhow!("UTxO RPC protocol parameters missing protocol version"))?;
    if params.max_tx_size == 0 {
        return Err(anyhow!("UTxO RPC protocol parameters missing max_tx_size"));
    }
    if params.collateral_percentage == 0 {
        return Err(anyhow!(
            "UTxO RPC protocol parameters missing collateral_percentage"
        ));
    }
    if params.max_collateral_inputs == 0 {
        return Err(anyhow!(
            "UTxO RPC protocol parameters missing max_collateral_inputs"
        ));
    }

    let prices = params
        .prices
        .as_ref()
        .ok_or_else(|| anyhow!("UTxO RPC protocol parameters missing execution prices"))?;
    let execution_units = params
        .max_execution_units_per_transaction
        .as_ref()
        .ok_or_else(|| {
            anyhow!("UTxO RPC protocol parameters missing transaction execution limits")
        })?;
    if execution_units.memory == 0
        || execution_units.memory > i64::MAX as u64
        || execution_units.steps == 0
        || execution_units.steps > i64::MAX as u64
    {
        return Err(anyhow!(
            "UTxO RPC protocol parameters invalid transaction execution limits"
        ));
    }

    let cost_models = params
        .cost_models
        .as_ref()
        .ok_or_else(|| anyhow!("UTxO RPC protocol parameters missing cost models"))?;
    if cost_models.plutus_v3.is_none() {
        return Err(anyhow!(
            "UTxO RPC protocol parameters missing Plutus V3 cost model"
        ));
    }
    let mut cost_models_raw = BTreeMap::new();
    for (name, model) in [
        ("PlutusV1", cost_models.plutus_v1.as_ref()),
        ("PlutusV2", cost_models.plutus_v2.as_ref()),
        ("PlutusV3", cost_models.plutus_v3.as_ref()),
    ] {
        if let Some(model) = model {
            if !(1..=1024).contains(&model.values.len()) {
                return Err(anyhow!(
                    "UTxO RPC protocol parameters invalid {name} cost model length"
                ));
            }
            cost_models_raw.insert(name.to_owned(), model.values.clone());
        }
    }

    Ok(BloxbeanPayload {
        min_fee_a: big_int_to_u64(params.min_fee_coefficient.as_ref(), "min_fee_coefficient")?,
        min_fee_b: big_int_to_u64(params.min_fee_constant.as_ref(), "min_fee_constant")?,
        max_tx_size: params.max_tx_size,
        key_deposit: big_int_to_u64(params.stake_key_deposit.as_ref(), "stake_key_deposit")?,
        pool_deposit: big_int_to_u64(params.pool_deposit.as_ref(), "pool_deposit")?,
        min_pool_cost: big_int_to_u64(params.min_pool_cost.as_ref(), "min_pool_cost")?,
        protocol_major_ver: version.major,
        protocol_minor_ver: version.minor,
        coins_per_utxo_size: big_int_to_u64(
            params.coins_per_utxo_byte.as_ref(),
            "coins_per_utxo_byte",
        )?,
        collateral_percent: params.collateral_percentage,
        max_collateral_inputs: params.max_collateral_inputs,
        price_mem: rational_to_decimal(prices.memory.as_ref(), "execution price memory")?,
        price_step: rational_to_decimal(prices.steps.as_ref(), "execution price steps")?,
        min_fee_ref_script_cost_per_byte: rational_to_decimal(
            params.min_fee_script_ref_cost_per_byte.as_ref(),
            "min_fee_script_ref_cost_per_byte",
        )?,
        max_tx_ex_mem: execution_units.memory,
        max_tx_ex_steps: execution_units.steps,
        cost_models_raw,
    })
}

pub fn era_epoch(
    summary: &query::read_era_summary_response::Summary,
    slot: u64,
) -> anyhow::Result<(String, u64)> {
    let query::read_era_summary_response::Summary::Cardano(summaries) = summary;
    let mut current = None;
    for (index, era) in summaries.summaries.iter().enumerate() {
        let start = era
            .start
            .as_ref()
            .ok_or_else(|| anyhow!("era summary missing start point"))?;
        let ended = era.end.as_ref().is_some_and(|end| slot >= end.slot);
        if slot >= start.slot && !ended {
            current = Some((index, era, start));
        }
    }
    let (index, era, start) = current.ok_or_else(|| anyhow!("no era contains tip slot"))?;
    let epoch_length = summaries
        .summaries
        .get(index + 1)
        .and_then(|next| {
            let next_start = next.start.as_ref()?;
            let slots = next_start.slot.checked_sub(start.slot)?;
            let epochs = next_start.epoch.checked_sub(start.epoch)?;
            slots.checked_div(epochs)
        })
        .or_else(|| {
            index.checked_sub(1).and_then(|prev| {
                let prev = summaries.summaries.get(prev)?;
                let prev_start = prev.start.as_ref()?;
                let slots = start.slot.checked_sub(prev_start.slot)?;
                let epochs = start.epoch.checked_sub(prev_start.epoch)?;
                slots.checked_div(epochs)
            })
        })
        .ok_or_else(|| anyhow!("era summary cannot establish epoch length"))?;
    if epoch_length == 0 {
        return Err(anyhow!("computed zero epoch length"));
    }
    let epoch = start.epoch + (slot.saturating_sub(start.slot) / epoch_length);
    Ok((era.name.clone(), epoch))
}

#[cfg(test)]
mod tests {
    use super::{bloxbean, build, bytes_to_u64, era_epoch, rational_to_decimal, rational_to_f64};
    use std::time::Duration;
    use utxorpc::spec::{cardano, query};

    fn bigint(value: i64) -> cardano::BigInt {
        cardano::BigInt {
            big_int: Some(cardano::big_int::BigInt::Int(value)),
        }
    }

    fn rational(numerator: i32, denominator: u32) -> cardano::RationalNumber {
        cardano::RationalNumber {
            numerator,
            denominator,
        }
    }

    fn params() -> query::AnyChainParams {
        query::AnyChainParams {
            params: Some(query::any_chain_params::Params::Cardano(cardano::PParams {
                min_fee_coefficient: Some(bigint(44)),
                min_fee_constant: Some(bigint(155_381)),
                collateral_percentage: 150,
                min_fee_script_ref_cost_per_byte: Some(rational(15, 1)),
                prices: Some(cardano::ExPrices {
                    steps: Some(rational(721, 10_000_000)),
                    memory: Some(rational(577, 10_000)),
                }),
                cost_models: Some(cardano::CostModels {
                    plutus_v1: None,
                    plutus_v2: None,
                    plutus_v3: Some(cardano::CostModel {
                        values: vec![1, 2, 3],
                    }),
                }),
                ..Default::default()
            })),
        }
    }

    fn bloxbean_params() -> cardano::PParams {
        let mut pparams = match params().params {
            Some(query::any_chain_params::Params::Cardano(params)) => params,
            _ => panic!("cardano"),
        };
        pparams.max_tx_size = 16_384;
        pparams.stake_key_deposit = Some(bigint(2_000_000));
        pparams.pool_deposit = Some(bigint(500_000_000));
        pparams.min_pool_cost = Some(bigint(170_000_000));
        pparams.coins_per_utxo_byte = Some(bigint(4310));
        pparams.protocol_version = Some(cardano::ProtocolVersion {
            major: 10,
            minor: 0,
        });
        pparams.max_collateral_inputs = 3;
        pparams.max_execution_units_per_transaction = Some(cardano::ExUnits {
            memory: 14_000_000,
            steps: 10_000_000_000,
        });
        pparams
    }

    fn assert_bloxbean_rejected(mutate: impl FnOnce(&mut cardano::PParams), expected: &str) {
        let mut params = bloxbean_params();
        mutate(&mut params);
        let error = bloxbean(&params).expect_err("invalid parameters should fail");
        assert!(
            error.to_string().contains(expected),
            "expected {expected:?}, got {error}"
        );
    }

    fn era_summary(
        start_time_ms: u64,
        first_shelley_slot: u64,
    ) -> query::read_era_summary_response::Summary {
        query::read_era_summary_response::Summary::Cardano(cardano::EraSummaries {
            summaries: vec![cardano::EraSummary {
                name: "Shelley".to_string(),
                start: Some(cardano::EraBoundary {
                    time: start_time_ms,
                    slot: first_shelley_slot,
                    epoch: 0,
                }),
                end: None,
                protocol_params: None,
            }],
        })
    }

    #[test]
    fn bytes_to_u64_rejects_values_larger_than_u64() {
        let error = bytes_to_u64(&[0; 9], "bigint").expect_err("overflow should fail");
        assert!(error.to_string().contains("value exceeds u64"));
    }

    #[test]
    fn rational_to_f64_rejects_zero_denominator() {
        let error = rational_to_f64(
            Some(&cardano::RationalNumber {
                numerator: 1,
                denominator: 0,
            }),
            "price",
        )
        .expect_err("zero denominator should fail");

        assert!(error.to_string().contains("zero denominator"));
    }

    #[test]
    fn build_derives_protocol_parameters_from_era_boundary() {
        let params = build(params(), era_summary(1_700_000_000_000, 4_492_800))
            .expect("protocol parameters should build");

        assert_eq!(params.base_fee(1), 155_425);
        assert_eq!(params.minimum_collateral(100), 150);
        assert_eq!(params.plutus_v3_cost_model(), &vec![1, 2, 3]);
        assert_eq!(
            params.posix_to_slot(Duration::from_secs(1_700_000_000)),
            4_492_800
        );
    }

    #[test]
    fn build_rejects_negative_chain_start() {
        let error = build(params(), era_summary(10_000, 1_000))
            .expect_err("underflowing chain start should fail");

        assert!(
            error
                .to_string()
                .contains("computed negative Cardano start time")
        );
    }

    #[test]
    fn build_requires_shelley_era_summary() {
        let error = build(
            params(),
            query::read_era_summary_response::Summary::Cardano(cardano::EraSummaries {
                summaries: vec![],
            }),
        )
        .expect_err("missing shelley summary should fail");

        assert!(error.to_string().contains("missing Shelley era"));
    }

    #[test]
    fn build_requires_execution_prices() {
        let mut params = params();
        let query::any_chain_params::Params::Cardano(pparams) =
            params.params.as_mut().expect("cardano params");
        pparams.prices = None;

        let error = build(params, era_summary(1_700_000_000_000, 4_492_800))
            .expect_err("missing prices should fail");

        assert!(error.to_string().contains("missing execution prices"));
    }

    #[test]
    fn build_requires_plutus_v3_cost_model() {
        let mut params = params();
        let query::any_chain_params::Params::Cardano(pparams) =
            params.params.as_mut().expect("cardano params");
        pparams.cost_models = Some(cardano::CostModels {
            plutus_v1: None,
            plutus_v2: None,
            plutus_v3: None,
        });

        let error = build(params, era_summary(1_700_000_000_000, 4_492_800))
            .expect_err("missing plutus v3 should fail");

        assert!(error.to_string().contains("missing Plutus V3 cost model"));
    }

    #[test]
    fn bloxbean_exports_exact_live_values() {
        let mut params = bloxbean_params();
        params.cost_models.as_mut().unwrap().plutus_v1 = Some(cardano::CostModel {
            values: vec![-3, 2, -1],
        });

        let payload = bloxbean(&params).expect("payload");
        assert_eq!(payload.min_fee_a, 44);
        assert_eq!(payload.coins_per_utxo_size, 4310);
        assert_eq!(payload.protocol_major_ver, 10);
        assert_eq!(payload.price_mem, "0.0577");
        assert_eq!(payload.price_step, "0.0000721");
        assert_eq!(payload.min_fee_ref_script_cost_per_byte, "15");
        assert_eq!(payload.max_tx_ex_mem, 14_000_000);
        assert_eq!(payload.max_tx_ex_steps, 10_000_000_000);
        assert_eq!(payload.cost_models_raw["PlutusV1"], vec![-3, 2, -1]);
        assert_eq!(payload.cost_models_raw["PlutusV3"], vec![1, 2, 3]);
    }

    #[test]
    fn exact_decimal_rejects_invalid_rationals() {
        assert_eq!(
            rational_to_decimal(Some(&rational(0, 7)), "price").unwrap(),
            "0"
        );
        assert!(
            rational_to_decimal(None, "price")
                .unwrap_err()
                .to_string()
                .contains("missing")
        );
        assert!(
            rational_to_decimal(Some(&rational(-1, 2)), "price")
                .unwrap_err()
                .to_string()
                .contains("negative")
        );
        assert!(
            rational_to_decimal(Some(&rational(1, 0)), "price")
                .unwrap_err()
                .to_string()
                .contains("zero denominator")
        );
        assert!(
            rational_to_decimal(Some(&rational(1, 3)), "price")
                .unwrap_err()
                .to_string()
                .contains("non-terminating")
        );
    }

    #[test]
    fn bloxbean_requires_all_prices() {
        assert_bloxbean_rejected(|params| params.prices = None, "missing execution prices");
        assert_bloxbean_rejected(
            |params| params.prices.as_mut().unwrap().memory = None,
            "missing execution price memory",
        );
        assert_bloxbean_rejected(
            |params| params.min_fee_script_ref_cost_per_byte = None,
            "missing min_fee_script_ref_cost_per_byte",
        );
        assert_bloxbean_rejected(
            |params| params.prices.as_mut().unwrap().steps = Some(rational(-1, 2)),
            "negative ratio",
        );
    }

    #[test]
    fn bloxbean_rejects_non_decimal_prices() {
        assert_bloxbean_rejected(
            |params| params.prices.as_mut().unwrap().memory = Some(rational(1, 0)),
            "zero denominator",
        );
        assert_bloxbean_rejected(
            |params| params.min_fee_script_ref_cost_per_byte = Some(rational(1, 3)),
            "non-terminating decimal",
        );
    }

    #[test]
    fn bloxbean_requires_bounded_transaction_execution_limits() {
        assert_bloxbean_rejected(
            |params| params.max_execution_units_per_transaction = None,
            "missing transaction execution limits",
        );
        assert_bloxbean_rejected(
            |params| {
                params
                    .max_execution_units_per_transaction
                    .as_mut()
                    .unwrap()
                    .memory = 0
            },
            "invalid transaction execution limits",
        );
        assert_bloxbean_rejected(
            |params| {
                params
                    .max_execution_units_per_transaction
                    .as_mut()
                    .unwrap()
                    .steps = i64::MAX as u64 + 1
            },
            "invalid transaction execution limits",
        );
    }

    #[test]
    fn bloxbean_requires_bounded_cost_models_and_plutus_v3() {
        assert_bloxbean_rejected(|params| params.cost_models = None, "missing cost models");
        assert_bloxbean_rejected(
            |params| params.cost_models.as_mut().unwrap().plutus_v3 = None,
            "missing Plutus V3 cost model",
        );
        assert_bloxbean_rejected(
            |params| {
                params
                    .cost_models
                    .as_mut()
                    .unwrap()
                    .plutus_v3
                    .as_mut()
                    .unwrap()
                    .values
                    .clear()
            },
            "invalid PlutusV3 cost model length",
        );
        assert_bloxbean_rejected(
            |params| {
                params.cost_models.as_mut().unwrap().plutus_v2 = Some(cardano::CostModel {
                    values: vec![0; 1025],
                })
            },
            "invalid PlutusV2 cost model length",
        );
    }

    #[test]
    fn era_epoch_from_consecutive_eras() {
        let summary = query::read_era_summary_response::Summary::Cardano(cardano::EraSummaries {
            summaries: vec![
                cardano::EraSummary {
                    name: "Shelley".into(),
                    start: Some(cardano::EraBoundary {
                        time: 0,
                        slot: 0,
                        epoch: 0,
                    }),
                    end: Some(cardano::EraBoundary {
                        time: 0,
                        slot: 864_000,
                        epoch: 2,
                    }),
                    protocol_params: None,
                },
                cardano::EraSummary {
                    name: "Conway".into(),
                    start: Some(cardano::EraBoundary {
                        time: 0,
                        slot: 864_000,
                        epoch: 2,
                    }),
                    end: None,
                    protocol_params: None,
                },
            ],
        });
        let (era, epoch) = era_epoch(&summary, 864_000 + 432_000).expect("epoch");
        assert_eq!(era, "Conway");
        assert_eq!(epoch, 3);
    }

    #[test]
    fn era_epoch_rejects_missing_epoch_length() {
        let error = era_epoch(&era_summary(0, 0), 1).expect_err("single era is ambiguous");
        assert!(error.to_string().contains("cannot establish epoch length"));
    }
}

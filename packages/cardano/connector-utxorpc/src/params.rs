use anyhow::{Context, anyhow};
use cardano_sdk::{Milliseconds, PosixMilliseconds, ProtocolParameters, Slot};
use num::rational::Ratio;
use utxorpc::spec::{cardano, query};

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

/// The length of a slot, derived from the era summary.
///
/// Two consecutive eras give us two (slot, time) pairs; their quotient is the
/// slot length the chain actually runs with. The era boundaries are expressed
/// in milliseconds, like the ledger does.
fn slot_length_ms(
    summaries: &[cardano::EraSummary],
    shelley: &cardano::EraSummary,
) -> Option<Milliseconds> {
    let shelley_start = shelley.start.as_ref()?;
    let shelley_index = summaries
        .iter()
        .position(|era| std::ptr::eq(era, shelley))?;

    let next = summaries
        .iter()
        .skip(shelley_index + 1)
        .find_map(|era| era.start.as_ref())?;

    let slot_delta = next.slot.checked_sub(shelley_start.slot)?;
    let time_delta_ms = next.time.checked_sub(shelley_start.time)?;
    if slot_delta == 0 {
        return None;
    }
    Some(Milliseconds::new(time_delta_ms / slot_delta))
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

    // The chain's time anchor: the POSIX time of the Shelley era's first slot.
    // (The Shelley era's slot number is a *global* one, so the anchor is
    // `posix(slot) = start.time + (slot - start.slot) * slot_length`.)
    let first_shelley_slot = Slot::new(start.slot);
    let start_time = PosixMilliseconds::new(start.time);
    let slot_length_ms = slot_length_ms(&summaries.summaries, shelley);
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

    // The slot length is a genesis parameter; the era summary only tells us
    // it indirectly (through the distance between two era boundaries). When
    // the chain has not crossed an era boundary yet we keep the default (one
    // second, the public networks' slot length).
    let base = match slot_length_ms {
        Some(slot_length_ms) => base.with_slot_length_ms(slot_length_ms),
        None => base,
    };

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
    use super::{build, bytes_to_u64, rational_to_f64};
    use cardano_sdk::{Milliseconds, PosixMilliseconds, Slot};
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

    fn era_summaries(
        summaries: Vec<cardano::EraSummary>,
    ) -> query::read_era_summary_response::Summary {
        query::read_era_summary_response::Summary::Cardano(cardano::EraSummaries { summaries })
    }

    fn era_summary(
        start_time_ms: u64,
        first_shelley_slot: u64,
    ) -> query::read_era_summary_response::Summary {
        era_summaries(vec![cardano::EraSummary {
            name: "Shelley".to_string(),
            start: Some(cardano::EraBoundary {
                time: start_time_ms,
                slot: first_shelley_slot,
                epoch: 0,
            }),
            end: None,
            protocol_params: None,
        }])
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
            params.posix_to_slot(PosixMilliseconds::new(1_700_000_000_000)),
            Slot::new(4_492_800)
        );
    }

    #[test]
    fn build_anchors_on_the_shelley_era_boundary() {
        let params = build(params(), era_summary(1_700_000_000_000, 4_492_800))
            .expect("protocol parameters should build");

        // The anchor is the era boundary itself — the Byron-era offset that
        // used to be subtracted from it is gone, so a chain whose Shelley era
        // started long after the epoch is mapped as it really is.
        assert_eq!(
            params.posix_to_slot(PosixMilliseconds::new(1_700_000_000_000)),
            Slot::new(4_492_800)
        );
        assert_eq!(
            params.slot_to_posix(Slot::new(4_492_800)),
            PosixMilliseconds::new(1_700_000_000_000)
        );
    }

    #[test]
    fn build_derives_the_slot_length_from_consecutive_eras() {
        // Two eras, 500 slots apart and 100 seconds apart: the chain runs with
        // slots of 200 milliseconds.
        let summaries = era_summaries(vec![
            cardano::EraSummary {
                name: "Shelley".to_string(),
                start: Some(cardano::EraBoundary {
                    time: 1_700_000_000_000,
                    slot: 4_492_800,
                    epoch: 0,
                }),
                end: None,
                protocol_params: None,
            },
            cardano::EraSummary {
                name: "Allegra".to_string(),
                start: Some(cardano::EraBoundary {
                    time: 1_700_000_100_000,
                    slot: 4_493_300,
                    epoch: 1,
                }),
                end: None,
                protocol_params: None,
            },
        ]);

        let params = build(params(), summaries).expect("protocol parameters should build");

        assert_eq!(params.slot_length_ms(), Milliseconds::new(200));
        assert_eq!(
            params.posix_to_slot(PosixMilliseconds::new(1_700_000_000_000 + 200)),
            Slot::new(4_492_801)
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
}

use anyhow::{Context, anyhow};
use cardano_sdk::{Network, ProtocolParameters, PublicNetwork};
use num::rational::Ratio;
use pallas_network::miniprotocols::localstate::queries_v16::{self, ProtocolParam, SystemStart};

pub async fn read(
    client: &mut pallas_network::miniprotocols::localstate::Client,
    network: Network,
) -> anyhow::Result<ProtocolParameters> {
    let era = queries_v16::get_current_era(client)
        .await
        .map_err(|error| anyhow!(error))
        .context("failed to query current era from local node")?;

    let params = queries_v16::get_current_pparams(client, era)
        .await
        .map_err(|error| anyhow!(error))
        .context("failed to query protocol parameters from local node")?;

    let system_start = queries_v16::get_system_start(client)
        .await
        .map_err(|error| anyhow!(error))
        .context("failed to query system start from local node")?;

    let plutus_v3_cost_model = crate::v3_cost_model::extract(client, era).await?;

    let params = params
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("local node returned an empty protocol parameters set"))?;

    build(params, system_start, network, plutus_v3_cost_model)
}

fn build(
    params: ProtocolParam,
    system_start: SystemStart,
    network: Network,
    plutus_v3_cost_model: Vec<i64>,
) -> anyhow::Result<ProtocolParameters> {
    let min_fee_a = params
        .minfee_a
        .ok_or_else(|| anyhow!("local node protocol parameters missing minfee_a"))?;
    let min_fee_b = params
        .minfee_b
        .ok_or_else(|| anyhow!("local node protocol parameters missing minfee_b"))?;
    let collateral_percentage = params
        .collateral_percentage
        .ok_or_else(|| anyhow!("local node protocol parameters missing collateral_percentage"))?;
    let reference_scripts_fee_per_byte = params
        .ada_per_utxo_byte
        .ok_or_else(|| anyhow!("local node protocol parameters missing ada_per_utxo_byte"))?;
    let execution_costs = params
        .execution_costs
        .ok_or_else(|| anyhow!("local node protocol parameters missing execution_costs"))?;

    let start_time = system_start_to_posix(system_start)?;
    let first_shelley_slot = match PublicNetwork::try_from(network) {
        Ok(PublicNetwork::Mainnet) => 4_492_800,
        Ok(PublicNetwork::Preprod) => 86_400,
        Ok(PublicNetwork::Preview) => 0,
        Err(_) => 0,
    };

    Ok(ProtocolParameters::default()
        .with_fee_per_byte(u64::from(min_fee_a))
        .with_fee_constant(u64::from(min_fee_b))
        .with_collateral_coefficient(f64::from(collateral_percentage) / 100.0)
        .with_referenced_scripts_base_fee_per_byte(u64::from(reference_scripts_fee_per_byte))
        .with_referenced_scripts_fee_multiplier(Ratio::new(12, 10))
        .with_referenced_scripts_fee_step_size(25_000)
        .with_execution_price_mem(rational_to_f64(
            execution_costs.mem_price,
            "execution price memory",
        )?)
        .with_execution_price_cpu(rational_to_f64(
            execution_costs.step_price,
            "execution price steps",
        )?)
        .with_start_time(start_time)
        .with_first_shelley_slot(first_shelley_slot)
        .with_plutus_v3_cost_model(plutus_v3_cost_model))
}

fn rational_to_f64(rational: queries_v16::RationalNumber, label: &str) -> anyhow::Result<f64> {
    if rational.denominator == 0 {
        return Err(anyhow!("invalid {label}: zero denominator"));
    }
    Ok(rational.numerator as f64 / rational.denominator as f64)
}

/// Convert a Cardano genesis `SystemStart` (`year`, `day_of_year`,
/// `picoseconds_of_day`) into a UNIX timestamp in seconds.
///
/// Neither `cardano-sdk` nor `pallas 0.35.1` ship this conversion — pallas
/// only returns the raw `SystemStart` record, while `cardano-sdk` exposes
/// `posix_to_slot` (which needs `start_time`) but not the inverse. We
/// implement the small amount of leap-year / day-counting arithmetic
/// locally; the unit test pins the well-known `2024-01-01T00:00:00Z`
/// reference value at `1_704_067_200`.
fn system_start_to_posix(system_start: SystemStart) -> anyhow::Result<u64> {
    let seconds_in_day = 86_400_u64;
    let picos_in_second = 1_000_000_000_000_u64;

    let unix_epoch_year = 1970_u32;
    let unix_epoch_days_before_year = days_before_year(unix_epoch_year)
        .ok_or_else(|| anyhow!("system start year {} out of range", system_start.year))?;

    let days_before_this_year = days_before_year(system_start.year)
        .ok_or_else(|| anyhow!("system start year {} out of range", system_start.year))?;

    let day_of_year = u64::from(system_start.day_of_year);
    let total_days = days_before_this_year
        .checked_add(day_of_year.saturating_sub(1))
        .and_then(|d| d.checked_sub(unix_epoch_days_before_year))
        .ok_or_else(|| anyhow!("system start {} out of range", system_start.year))?;

    let seconds_of_day = system_start.picoseconds_of_day / picos_in_second;
    if seconds_of_day >= seconds_in_day {
        return Err(anyhow!(
            "system start picoseconds_of_day {} overflows a day",
            system_start.picoseconds_of_day
        ));
    }

    let total_seconds = total_days
        .checked_mul(seconds_in_day)
        .and_then(|s| s.checked_add(seconds_of_day))
        .ok_or_else(|| anyhow!("system start time overflows u64"))?;

    Ok(total_seconds)
}

fn is_leap_year(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_before_year(year: u32) -> Option<u64> {
    if year < 1970 {
        return None;
    }
    let mut days = 0_u64;
    for y in 1970..year {
        days = days.checked_add(if is_leap_year(y) { 366 } else { 365 })?;
    }
    Some(days)
}

#[cfg(test)]
mod tests {
    use super::{rational_to_f64, system_start_to_posix};
    use pallas_network::miniprotocols::localstate::queries_v16::{RationalNumber, SystemStart};

    #[test]
    fn rational_to_f64_rejects_zero_denominator() {
        let error = rational_to_f64(
            RationalNumber {
                numerator: 1,
                denominator: 0,
            },
            "price",
        )
        .expect_err("zero denominator should fail");

        assert!(error.to_string().contains("zero denominator"));
    }

    #[test]
    fn system_start_to_posix_decodes_a_known_year() {
        let posix = system_start_to_posix(SystemStart {
            year: 2024,
            day_of_year: 1,
            picoseconds_of_day: 0,
        })
        .expect("known date should decode");

        assert_eq!(posix, 1_704_067_200);
    }

    #[test]
    fn system_start_to_posix_handles_non_leap_year() {
        // 2024 is a leap year; 2023 is not. 2023-01-01T00:00:00Z ==
        // 1_672_531_200.
        let posix = system_start_to_posix(SystemStart {
            year: 2023,
            day_of_year: 1,
            picoseconds_of_day: 0,
        })
        .expect("known date should decode");

        assert_eq!(posix, 1_672_531_200);
    }

    #[test]
    fn system_start_to_posix_picks_up_picoseconds_in_day() {
        // Half a day into 2024 should add 43_200 seconds to the epoch
        // start of 2024.
        let posix = system_start_to_posix(SystemStart {
            year: 2024,
            day_of_year: 1,
            picoseconds_of_day: 43_200 * 1_000_000_000_000,
        })
        .expect("known date should decode");

        assert_eq!(posix, 1_704_067_200 + 43_200);
    }

    #[test]
    fn system_start_to_posix_rejects_years_before_unix_epoch() {
        let error = system_start_to_posix(SystemStart {
            year: 1969,
            day_of_year: 1,
            picoseconds_of_day: 0,
        })
        .expect_err("pre-epoch years should be rejected");

        assert!(error.to_string().contains("out of range"));
    }

    #[test]
    fn system_start_to_posix_rejects_picoseconds_overflowing_a_day() {
        let error = system_start_to_posix(SystemStart {
            year: 2024,
            day_of_year: 1,
            picoseconds_of_day: 86_400 * 1_000_000_000_000,
        })
        .expect_err("overflowing picoseconds should fail");

        assert!(error.to_string().contains("overflows a day"));
    }
}

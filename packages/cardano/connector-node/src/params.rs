use anyhow::{Context, anyhow};
use cardano_sdk::{ProtocolParameters, Slot};
use num::rational::Ratio;
use pallas_network::miniprotocols::localstate::queries_v16::{self, ProtocolParam};

use crate::genesis_config::{self, GenesisConfig};

/// The ledger's cost, in lovelace, of one byte of a *reference script*.
///
/// The ledger charges a fee for the reference scripts a transaction carries
/// (see `Cardano.Ledger.Alonzo.Rules.Utxo` and the ADR
/// "2024-08-14_009-refscripts-fee-change"): the script's size is multiplied
/// by this rate, and every 25 000 bytes above that the rate grows by 20%.
///
/// **This is not `adaPerUTxOByte`.** That parameter (4310 on the public
/// networks) prices the *storage* of a UTxO and has nothing to do with
/// scripts. Using it here charged 4310 lovelace per byte of script — with the
/// konduit validator's 5 470 bytes that is **23.6 ADA of pure reference-script
/// fee per transaction**, on top of a base fee of 0.17 ADA. The node accepted
/// it (it only checks that the fee is *at least* the minimum), so the bug was
/// invisible on-chain: it just made every transaction cost ~140x more than it
/// should and starved the wallets the e2e scenarios drive.
///
/// The value below is the one the public networks' genesis carries
/// (`minFeeRefScriptCostPerByte`), and it is what the UTxO-RPC connector
/// reads from the node as well. Chains whose genesis asks for less are
/// over-charged by a few hundred thousand lovelace, which the node accepts;
/// chains asking for more would need this to be raised.
const REFERENCE_SCRIPT_COST_PER_BYTE: u64 = 15;

pub async fn read(
    client: &mut pallas_network::miniprotocols::localstate::Client,
) -> anyhow::Result<ProtocolParameters> {
    let era = queries_v16::get_current_era(client)
        .await
        .map_err(|error| anyhow!(error))
        .context("failed to query current era from local node")?;

    let params = queries_v16::get_current_pparams(client, era)
        .await
        .map_err(|error| anyhow!(error))
        .context("failed to query protocol parameters from local node")?;

    // The slot length is a genesis parameter, so it does not come with the
    // protocol parameters.
    let genesis = genesis_config::read(client, era).await?;

    // The chain's current slot, needed to anchor the slot ⇄ time mapping.
    let point = queries_v16::get_chain_point(client)
        .await
        .map_err(|error| anyhow!(error))
        .context("failed to query chain point from local node")?;
    let current_slot = point.slot_or_default();

    let plutus_v3_cost_model = crate::v3_cost_model::extract(client, era).await?;

    let params = params
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("local node returned an empty protocol parameters set"))?;

    build(params, genesis, current_slot, plutus_v3_cost_model)
}

fn build(
    params: ProtocolParam,
    genesis: GenesisConfig,
    current_slot: u64,
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
    let execution_costs = params
        .execution_costs
        .ok_or_else(|| anyhow!("local node protocol parameters missing execution_costs"))?;

    // The chain's time anchor: the era's start time (from the genesis) and the
    // global slot number it sits at. The genesis does not carry the slot
    // number, so we derive it from the chain's current slot and the wall
    // clock — accurate to within a slot, which is all a validity range needs.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| anyhow!("system time is before the unix epoch: {error}"))?
        .as_millis() as u64;
    let elapsed_ms = now.saturating_sub(genesis.start_time.as_millis());
    let slots_since_start = elapsed_ms / genesis.slot_length_ms.as_millis().max(1);
    let first_shelley_slot = Slot::new(current_slot.saturating_sub(slots_since_start));

    Ok(ProtocolParameters::default()
        .with_fee_per_byte(u64::from(min_fee_a))
        .with_fee_constant(u64::from(min_fee_b))
        .with_collateral_coefficient(f64::from(collateral_percentage) / 100.0)
        .with_referenced_scripts_base_fee_per_byte(REFERENCE_SCRIPT_COST_PER_BYTE)
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
        .with_start_time(genesis.start_time)
        .with_first_shelley_slot(first_shelley_slot)
        .with_slot_length_ms(genesis.slot_length_ms)
        .with_plutus_v3_cost_model(plutus_v3_cost_model))
}

fn rational_to_f64(rational: queries_v16::RationalNumber, label: &str) -> anyhow::Result<f64> {
    if rational.denominator == 0 {
        return Err(anyhow!("invalid {label}: zero denominator"));
    }
    Ok(rational.numerator as f64 / rational.denominator as f64)
}

#[cfg(test)]
mod tests {
    use super::{REFERENCE_SCRIPT_COST_PER_BYTE, rational_to_f64};
    use pallas_network::miniprotocols::localstate::queries_v16::RationalNumber;

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
    fn the_reference_script_rate_is_not_the_utxo_storage_rate() {
        // `adaPerUTxOByte` (4310) is the price of *storing* a utxo. Using it
        // as the reference-script rate charged every konduit transaction
        // 23.6 ada of script fee (5 470 bytes of validator at 4310 per byte).
        // The ledger charges 15.
        assert_eq!(REFERENCE_SCRIPT_COST_PER_BYTE, 15);
        assert_ne!(REFERENCE_SCRIPT_COST_PER_BYTE, 4310);
    }
}

//! Extract the chain's time-related genesis parameters from a local Cardano
//! node.
//!
//! The slot length is a *genesis* parameter, not a protocol parameter, so it
//! cannot be read from the node's protocol-parameters response. It is part of
//! the genesis configuration, which the node serves through the local-state
//! query `BlockQuery::GetGenesisConfig`.
//!
//! Pallas models that response as a flat `GenesisConfig` struct, but the node
//! sends more fields than the struct has, so we decode the raw CBOR ourselves —
//! the same trick as `v3_cost_model.rs`, and for the same reason.
//!
//! The response looks like this (field indices as sent by the node):
//!
//! ```text
//! array(1)                          -- [GenConfig]
//!   [0] array(18)                   -- the Shelley genesis and friends
//!     [0] array(3)                  -- SystemStart { year, day_of_year, picoseconds_of_day }
//!     [5] uint                      -- epochLength
//!     [8] uint                      -- slotLength, in *microseconds*
//!     ...
//! ```

use anyhow::{Context, anyhow};
use cardano_sdk::{Milliseconds, PosixMilliseconds};
use minicbor::Decoder;
use pallas_codec::utils::AnyCbor;
use pallas_network::miniprotocols::localstate::Client;
use pallas_network::miniprotocols::localstate::queries_v16::{
    BlockQuery, Era, LedgerQuery, Request,
};

/// The time-related part of the chain's genesis configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenesisConfig {
    /// The POSIX time at which the era described by this configuration
    /// started.
    pub start_time: PosixMilliseconds,
    /// The length of a slot.
    pub slot_length_ms: Milliseconds,
    /// The number of slots in an epoch.
    pub epoch_length: u32,
}

/// The field index of the `SystemStart` inside the genesis configuration.
const SYSTEM_START_FIELD: u64 = 0;
/// The field index of `epochLength` inside the genesis configuration.
const EPOCH_LENGTH_FIELD: u64 = 5;
/// The field index of `slotLength` inside the genesis configuration.
const SLOT_LENGTH_FIELD: u64 = 8;

/// The ledger reports the slot length in microseconds; the rest of our stack
/// works in milliseconds.
const MICROSECONDS_PER_MILLISECOND: u64 = 1_000;

pub async fn read(client: &mut Client, era: Era) -> anyhow::Result<GenesisConfig> {
    let request = AnyCbor::from_encode(Request::LedgerQuery(LedgerQuery::BlockQuery(
        era,
        BlockQuery::GetGenesisConfig,
    )));

    let response = client
        .query_any(request)
        .await
        .map_err(|error| anyhow!(error))
        .context("failed to query raw genesis configuration from local node")?;

    decode(response.raw_bytes())
}

/// Decode the raw `GetGenesisConfig` response.
pub fn decode(bytes: &[u8]) -> anyhow::Result<GenesisConfig> {
    let mut decoder = Decoder::new(bytes);

    // array(1) -- the list of era configurations
    let eras = read_array_len(&mut decoder, "genesis configuration")?;
    if eras == 0 {
        return Err(anyhow!(
            "local node returned an empty genesis configuration"
        ));
    }

    // array(N) -- the configuration of the era we asked about
    let fields = read_array_len(&mut decoder, "genesis configuration entry")?;

    let mut start_time = None;
    let mut slot_length_ms = None;
    let mut epoch_length = None;

    for field in 0..fields {
        match field {
            SYSTEM_START_FIELD => {
                let (year, day_of_year, picoseconds_of_day) = read_system_start(&mut decoder)?;
                start_time = Some(PosixMilliseconds::new(
                    system_start_to_posix(year, day_of_year, picoseconds_of_day)? * 1000,
                ));
            }
            EPOCH_LENGTH_FIELD => {
                epoch_length = Some(read_uint(&mut decoder)? as u32);
            }
            SLOT_LENGTH_FIELD => {
                let microseconds = read_uint(&mut decoder)?;
                let milliseconds = microseconds / MICROSECONDS_PER_MILLISECOND;
                if milliseconds == 0 {
                    return Err(anyhow!(
                        "local node reported a slot length of {microseconds}µs, which is shorter than a millisecond"
                    ));
                }
                slot_length_ms = Some(Milliseconds::new(milliseconds));
            }
            _ => skip_item(&mut decoder)?,
        }
    }

    Ok(GenesisConfig {
        start_time: start_time
            .ok_or_else(|| anyhow!("genesis configuration has no system start"))?,
        slot_length_ms: slot_length_ms
            .ok_or_else(|| anyhow!("genesis configuration has no slot length"))?,
        epoch_length: epoch_length.unwrap_or_default(),
    })
}

fn read_uint(decoder: &mut Decoder<'_>) -> anyhow::Result<u64> {
    decoder
        .u64()
        .map_err(|error| anyhow!("expected an unsigned integer: {error}"))
}

/// The length of the next array. Indefinite-length arrays report `None`.
fn read_array_len(decoder: &mut Decoder<'_>, what: &str) -> anyhow::Result<u64> {
    decoder
        .array()
        .map_err(|error| anyhow!("{what} is not an array: {error}"))?
        .ok_or_else(|| anyhow!("{what} has an indefinite length"))
}

/// The `SystemStart` is `{ year, day_of_year, picoseconds_of_day }`.
fn read_system_start(decoder: &mut Decoder<'_>) -> anyhow::Result<(u64, u64, u64)> {
    let len = read_array_len(decoder, "system start")?;
    if len != 3 {
        return Err(anyhow!("system start has {len} fields, expected 3"));
    }
    let year = read_uint(decoder)?;
    let day_of_year = read_uint(decoder)?;
    let picoseconds_of_day = read_uint(decoder)?;
    Ok((year, day_of_year, picoseconds_of_day))
}

/// Skip over one CBOR item, whatever it is.
fn skip_item(decoder: &mut Decoder<'_>) -> anyhow::Result<()> {
    decoder
        .skip()
        .map_err(|error| anyhow!("failed to skip over an unused genesis field: {error}"))
}

/// Convert a Cardano genesis `SystemStart` (`year`, `day_of_year`,
/// `picoseconds_of_day`) into a UNIX timestamp in seconds.
///
/// Neither `cardano-sdk` nor `pallas` ship this conversion — pallas only
/// returns the raw `SystemStart` record. We implement the small amount of
/// leap-year / day-counting arithmetic locally; the unit test pins the
/// well-known `2024-01-01T00:00:00Z` reference value at `1_704_067_200`.
fn system_start_to_posix(
    year: u64,
    day_of_year: u64,
    picoseconds_of_day: u64,
) -> anyhow::Result<u64> {
    let seconds_in_day = 86_400_u64;
    let picos_in_second = 1_000_000_000_000_u64;

    let unix_epoch_year = 1970_u32;
    let unix_epoch_days_before_year = days_before_year(unix_epoch_year)
        .ok_or_else(|| anyhow!("system start year {year} out of range"))?;

    let days_before_this_year = days_before_year(year as u32)
        .ok_or_else(|| anyhow!("system start year {year} out of range"))?;

    let total_days = days_before_this_year
        .checked_add(day_of_year.saturating_sub(1))
        .and_then(|d| d.checked_sub(unix_epoch_days_before_year))
        .ok_or_else(|| anyhow!("system start {year} out of range"))?;

    let seconds_of_day = picoseconds_of_day / picos_in_second;
    if seconds_of_day >= seconds_in_day {
        return Err(anyhow!(
            "system start picoseconds_of_day {picoseconds_of_day} overflows a day"
        ));
    }

    let total_seconds = total_days
        .checked_mul(seconds_in_day)
        .and_then(|s| s.checked_add(seconds_of_day))
        .ok_or_else(|| anyhow!("system start time overflows u64"))?;

    Ok(total_seconds)
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

fn is_leap_year(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use cardano_sdk::Milliseconds;

    /// The raw response of a local `conway_fast` testnet (slot length 200ms),
    /// captured from `cardano-node` 10.2.1.
    const RAW_GENESIS_CONFIG: &str = "818f831907ea1901291b0106ea3ef3f54000182a0082010a0a1903e81a0001fa4018401a00030d40011b009fdf42f6e4800092182c1a00025ef51a0001000019400019044c1a00061a801a1dcd6500121901f4d81e82030ad81e820b191388d81e820114d81e82040581000a000100a3581c21a3aaba70e677d0514052cf974364d61969782c8d95336e8214f63682581cc2e40cf4e4f24c29fc6165d64d738807c23fc3a6e7a80e408bfdeff35820024c3d2ddee8fa84e88e7a469d82e4228aa243b77ca6a693b1ecc877957eb8d1581c274f6ddd31702fde880db8bf60bafce0b06250ed76a98db51cb9008782581c972a853c424573df03837354bbd7825f9030a806d24adabfe6268d9658202243af5a8545c23e00fabf2f2d4fd64142918b3054695e682869f600eba39d8d581c6cfcaefc5453c8e886c9be4c10d3fbf7ff05a958b10229d401d8beec82581cdfed80d8a4e0136ad4f935e2a8de633bb8b834fa839a0db4036e697658201fb8512d19a4851c790281dfce90f0e9d776f286f31e9806d6b1d2bd778af99fa082a0a0";

    #[test]
    fn decodes_the_shelley_genesis_time_parameters() {
        let config = decode(&hex::decode(RAW_GENESIS_CONFIG).unwrap()).unwrap();

        // 2026-10-24T20:33:24Z
        assert_eq!(config.start_time.as_millis(), 1_792_874_004_000);
        // The testnet runs with 200ms slots.
        assert_eq!(config.slot_length_ms, Milliseconds::new(200));
        assert_eq!(config.epoch_length, 1000);
    }

    #[test]
    fn system_start_matches_the_known_reference() {
        // 2024-01-01T00:00:00Z
        let seconds = system_start_to_posix(2024, 1, 0).unwrap();
        assert_eq!(seconds, 1_704_067_200);
    }

    #[test]
    fn system_start_handles_non_leap_year() {
        // 2024 is a leap year; 2023 is not. 2023-01-01T00:00:00Z == 1_672_531_200.
        let seconds = system_start_to_posix(2023, 1, 0).unwrap();
        assert_eq!(seconds, 1_672_531_200);
    }

    #[test]
    fn system_start_picks_up_picoseconds_in_day() {
        // Half a day into 2024 should add 43_200 seconds to the epoch start.
        let seconds = system_start_to_posix(2024, 1, 43_200 * 1_000_000_000_000).unwrap();
        assert_eq!(seconds, 1_704_067_200 + 43_200);
    }

    #[test]
    fn system_start_rejects_years_before_unix_epoch() {
        let error = system_start_to_posix(1969, 1, 0).expect_err("pre-epoch years are rejected");
        assert!(error.to_string().contains("out of range"));
    }

    #[test]
    fn system_start_rejects_a_day_overflow() {
        assert!(system_start_to_posix(2024, 1, 86_400 * 1_000_000_000_000).is_err());
    }
}

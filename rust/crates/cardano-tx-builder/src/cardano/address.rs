//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{Hash, pallas};
use anyhow::anyhow;
use std::{fmt, str::FromStr};

#[derive(Debug, Clone)]
pub struct Address(Style);

#[derive(Debug, Clone)]
enum Style {
    Byron(pallas::ByronAddress),
    Shelley(pallas::ShelleyAddress),
}

// ------------------------------------------------------------------------- Inspecting

impl Address {
    /// Obtain the script payment credential of the address, if any. Returns 'None' if the address
    /// is not locked by a script.
    pub fn payment_script(&self) -> Option<Hash<28>> {
        match &self.0 {
            Style::Shelley(shelley) if shelley.payment().is_script() => {
                Some(Hash::from(shelley.payment().as_hash()))
            }
            Style::Byron(..) | Style::Shelley(..) => None,
        }
    }
}

// ------------------------------------------------------------------ Converting (from)

impl TryFrom<pallas::Address> for Address {
    type Error = anyhow::Error;

    fn try_from(address: pallas::Address) -> anyhow::Result<Address> {
        match address {
            pallas_addresses::Address::Byron(byron) => Ok(Address(Style::Byron(byron))),
            pallas_addresses::Address::Shelley(shelley) => Ok(Address(Style::Shelley(shelley))),
            pallas_addresses::Address::Stake(_) => {
                Err(anyhow!("found stake address masquerading as address"))
            }
        }
    }
}

impl TryFrom<&str> for Address {
    type Error = anyhow::Error;

    fn try_from(text: &str) -> anyhow::Result<Self> {
        Self::try_from(pallas::Address::from_str(text).map_err(|e| anyhow!(e))?)
    }
}

impl TryFrom<&[u8]> for Address {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Address> {
        Self::try_from(pallas::Address::from_bytes(bytes).map_err(|e| anyhow!(e))?)
    }
}

// ------------------------------------------------------------------ Converting (to)

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match &self.0 {
            Style::Byron(byron) => f.write_str(byron.to_base58().as_str()),
            Style::Shelley(shelley) => f.write_str(
                shelley
                    .to_bech32()
                    .expect("failed to convert to bech32!?")
                    .as_str(),
            ),
        }
    }
}

impl From<&Address> for Vec<u8> {
    fn from(address: &Address) -> Self {
        match &address.0 {
            Style::Byron(byron) => byron.to_vec(),
            Style::Shelley(shelley) => shelley.to_vec(),
        }
    }
}

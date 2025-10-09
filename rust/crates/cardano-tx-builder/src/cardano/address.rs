//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{Credential, NetworkId, pallas};
use anyhow::anyhow;
use std::{cmp::Ordering, fmt, marker::PhantomData, rc::Rc, str::FromStr};

pub mod style;
pub use style::KnownStyle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address<T: KnownStyle>(Rc<Style>, PhantomData<T>);

impl<T: KnownStyle + Eq> PartialOrd for Address<T> {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.cmp(rhs))
    }
}

impl<T: KnownStyle + Eq> Ord for Address<T> {
    fn cmp(&self, rhs: &Self) -> Ordering {
        <Vec<u8>>::from(self).cmp(&<Vec<u8>>::from(rhs))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Style {
    Byron(pallas::ByronAddress),
    Shelley(pallas::ShelleyAddress),
}

// ------------------------------------------------------------------------- Inspecting

impl<T: KnownStyle> Address<T> {
    pub fn is_byron(&self) -> bool {
        matches!(self.0.as_ref(), &Style::Byron(..))
    }

    pub fn as_byron(&self) -> Option<Address<style::Byron>> {
        if self.is_byron() {
            return Some(Address(self.0.clone(), PhantomData));
        }

        None
    }

    pub fn is_shelley(&self) -> bool {
        matches!(&self.0.as_ref(), Style::Shelley(..))
    }

    pub fn as_shelley(&self) -> Option<Address<style::Shelley>> {
        if self.is_shelley() {
            return Some(Address(self.0.clone(), PhantomData));
        }

        None
    }
}

impl Address<style::Shelley> {
    fn cast(&self) -> &pallas::ShelleyAddress {
        match self.0.as_ref() {
            Style::Shelley(shelley) => shelley,
            _ => unreachable!(),
        }
    }

    pub fn network(&self) -> NetworkId {
        NetworkId::from(self.cast().network())
    }

    pub fn payment_credential(&self) -> Credential {
        Credential::from(self.cast().payment())
    }

    pub fn delegation_credential(&self) -> Option<Credential> {
        Credential::try_from(self.cast().delegation()).ok()
    }
}

// -------------------------------------------------------------------- Building

impl Address<style::Shelley> {
    pub fn new(network: NetworkId, payment_credential: Credential) -> Self {
        Self::from(pallas::ShelleyAddress::new(
            pallas::Network::from(network),
            pallas::ShelleyPaymentPart::from(payment_credential),
            pallas::ShelleyDelegationPart::Null,
        ))
    }

    pub fn with_delegation(mut self, delegation_credential: Credential) -> Self {
        self = Self::from(pallas::ShelleyAddress::new(
            pallas::Network::from(self.network()),
            pallas::ShelleyPaymentPart::from(self.payment_credential()),
            pallas::ShelleyDelegationPart::from(delegation_credential),
        ));

        self
    }
}

impl Default for Address<style::Any> {
    fn default() -> Self {
        Self::from(
            Address::new(NetworkId::mainnet(), Credential::default())
                .with_delegation(Credential::default()),
        )
    }
}

// ----------------------------------------------------------- Converting (from)

impl From<pallas::ByronAddress> for Address<style::Byron> {
    fn from(byron_address: pallas::ByronAddress) -> Self {
        Self(Rc::new(Style::Byron(byron_address)), PhantomData)
    }
}

impl From<pallas::ShelleyAddress> for Address<style::Shelley> {
    fn from(shelley_address: pallas::ShelleyAddress) -> Self {
        Self(Rc::new(Style::Shelley(shelley_address)), PhantomData)
    }
}

impl From<Address<style::Byron>> for Address<style::Any> {
    fn from(byron_address: Address<style::Byron>) -> Self {
        Self(byron_address.0, PhantomData)
    }
}

impl From<Address<style::Shelley>> for Address<style::Any> {
    fn from(shelley_address: Address<style::Shelley>) -> Self {
        Self(shelley_address.0, PhantomData)
    }
}

impl TryFrom<pallas::Address> for Address<style::Any> {
    type Error = anyhow::Error;

    fn try_from(address: pallas::Address) -> anyhow::Result<Self> {
        match address {
            pallas_addresses::Address::Byron(byron) => Ok(Address::<style::Any>(
                Rc::new(Style::Byron(byron)),
                PhantomData,
            )),
            pallas_addresses::Address::Shelley(shelley) => Ok(Address::<style::Any>(
                Rc::new(Style::Shelley(shelley)),
                PhantomData,
            )),
            pallas_addresses::Address::Stake(_) => {
                Err(anyhow!("found stake address masquerading as address"))
            }
        }
    }
}

impl TryFrom<pallas::Address> for Address<style::Shelley> {
    type Error = anyhow::Error;

    fn try_from(address: pallas::Address) -> anyhow::Result<Self> {
        match address {
            pallas_addresses::Address::Shelley(shelley) => Ok(Address::<style::Shelley>(
                Rc::new(Style::Shelley(shelley)),
                PhantomData,
            )),
            pallas_addresses::Address::Byron(_) | pallas_addresses::Address::Stake(_) => {
                Err(anyhow!("not a shelley address"))
            }
        }
    }
}

impl<T: KnownStyle> TryFrom<&str> for Address<T>
where
    Address<T>: TryFrom<pallas::Address, Error = anyhow::Error>,
{
    type Error = anyhow::Error;

    fn try_from(text: &str) -> anyhow::Result<Self> {
        Self::try_from(pallas::Address::from_str(text).map_err(|e| anyhow!(e))?)
    }
}

impl<T: KnownStyle> TryFrom<&[u8]> for Address<T>
where
    Address<T>: TryFrom<pallas::Address, Error = anyhow::Error>,
{
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        Self::try_from(pallas::Address::from_bytes(bytes).map_err(|e| anyhow!(e))?)
    }
}

// --------------------------------------------------------------- Converting (to)

impl<T: KnownStyle> fmt::Display for Address<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self.0.as_ref() {
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

impl<T: KnownStyle> From<&Address<T>> for Vec<u8> {
    fn from(address: &Address<T>) -> Self {
        match address.0.as_ref() {
            Style::Byron(byron) => byron.to_vec(),
            Style::Shelley(shelley) => shelley.to_vec(),
        }
    }
}

//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{Credential, NetworkId, pallas};
use anyhow::anyhow;
use std::{
    borrow::{Borrow, Cow},
    fmt,
    marker::PhantomData,
    str::FromStr,
};

#[derive(Debug, Clone)]
pub struct Address<'a, T: KnownStyle>(Cow<'a, Style>, PhantomData<T>);

#[derive(Debug, Clone)]
enum Style {
    Byron(pallas::ByronAddress),
    Shelley(pallas::ShelleyAddress),
}

#[derive(Debug, Clone, Copy)]
pub struct Any;

#[derive(Debug, Clone, Copy)]
pub struct Shelley;

#[derive(Debug, Clone, Copy)]
pub struct Byron;

pub trait KnownStyle {}
impl KnownStyle for Any {}
impl KnownStyle for Byron {}
impl KnownStyle for Shelley {}

// ------------------------------------------------------------------------- Inspecting

impl<T: KnownStyle> Address<'static, T> {
    pub fn borrow<'a>(&'a self) -> Address<'a, T> {
        Address(Cow::Borrowed(self.0.borrow()), PhantomData)
    }
}

impl<'a, T: KnownStyle> Address<'a, T> {
    pub fn is_byron(&self) -> bool {
        matches!(&self.0.as_ref(), Style::Byron(..))
    }

    pub fn as_byron(&self) -> Option<Address<'_, Byron>> {
        if self.is_byron() {
            return Some(Address(Cow::Borrowed(self.0.as_ref()), PhantomData));
        }

        None
    }

    pub fn is_shelley(&self) -> bool {
        matches!(&self.0.as_ref(), Style::Shelley(..))
    }

    pub fn as_shelley(&self) -> Option<Address<'_, Shelley>> {
        if self.is_shelley() {
            return Some(Address(Cow::Borrowed(self.0.as_ref()), PhantomData));
        }

        None
    }
}

impl<'a> Address<'a, Shelley> {
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

impl Address<'static, Shelley> {
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

impl Default for Address<'static, Any> {
    fn default() -> Self {
        Self::from(
            Address::new(NetworkId::mainnet(), Credential::default())
                .with_delegation(Credential::default()),
        )
    }
}

// ----------------------------------------------------------- Converting (from)

impl From<pallas::ByronAddress> for Address<'static, Byron> {
    fn from(byron_address: pallas::ByronAddress) -> Self {
        Self(Cow::Owned(Style::Byron(byron_address)), PhantomData)
    }
}

impl From<pallas::ShelleyAddress> for Address<'static, Shelley> {
    fn from(shelley_address: pallas::ShelleyAddress) -> Self {
        Self(Cow::Owned(Style::Shelley(shelley_address)), PhantomData)
    }
}

impl<'a> From<Address<'a, Byron>> for Address<'a, Any> {
    fn from(byron_address: Address<'a, Byron>) -> Self {
        Self(byron_address.0, PhantomData)
    }
}

impl<'a> From<Address<'a, Shelley>> for Address<'a, Any> {
    fn from(shelley_address: Address<'a, Shelley>) -> Self {
        Self(shelley_address.0, PhantomData)
    }
}

impl TryFrom<pallas::Address> for Address<'static, Any> {
    type Error = anyhow::Error;

    fn try_from(address: pallas::Address) -> anyhow::Result<Self> {
        match address {
            pallas_addresses::Address::Byron(byron) => {
                Ok(Address::<Any>(Cow::Owned(Style::Byron(byron)), PhantomData))
            }
            pallas_addresses::Address::Shelley(shelley) => Ok(Address::<Any>(
                Cow::Owned(Style::Shelley(shelley)),
                PhantomData,
            )),
            pallas_addresses::Address::Stake(_) => {
                Err(anyhow!("found stake address masquerading as address"))
            }
        }
    }
}

impl TryFrom<pallas::Address> for Address<'static, Shelley> {
    type Error = anyhow::Error;

    fn try_from(address: pallas::Address) -> anyhow::Result<Self> {
        match address {
            pallas_addresses::Address::Shelley(shelley) => Ok(Address::<Shelley>(
                Cow::Owned(Style::Shelley(shelley)),
                PhantomData,
            )),
            pallas_addresses::Address::Byron(_) | pallas_addresses::Address::Stake(_) => {
                Err(anyhow!("not a shelley address"))
            }
        }
    }
}

impl<T: KnownStyle> TryFrom<&str> for Address<'static, T>
where
    Address<'static, T>: TryFrom<pallas::Address, Error = anyhow::Error>,
{
    type Error = anyhow::Error;

    fn try_from(text: &str) -> anyhow::Result<Self> {
        Self::try_from(pallas::Address::from_str(text).map_err(|e| anyhow!(e))?)
    }
}

impl<T: KnownStyle> TryFrom<&[u8]> for Address<'static, T>
where
    Address<'static, T>: TryFrom<pallas::Address, Error = anyhow::Error>,
{
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        Self::try_from(pallas::Address::from_bytes(bytes).map_err(|e| anyhow!(e))?)
    }
}

// --------------------------------------------------------------- Converting (to)

impl<'a, T: KnownStyle> Address<'a, T> {
    pub fn to_owned(self) -> Address<'static, T> {
        let style: Style = self.0.into_owned();
        Address(Cow::Owned(style), PhantomData)
    }
}

impl<'a, T: KnownStyle> fmt::Display for Address<'a, T> {
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

impl<'a, T: KnownStyle> From<&Address<'a, T>> for Vec<u8> {
    fn from(address: &Address<'a, T>) -> Self {
        match address.0.as_ref() {
            Style::Byron(byron) => byron.to_vec(),
            Style::Shelley(shelley) => shelley.to_vec(),
        }
    }
}

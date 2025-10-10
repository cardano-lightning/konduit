//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{Credential, NetworkId, pallas};
use anyhow::anyhow;
use std::{cmp::Ordering, fmt, marker::PhantomData, rc::Rc, str::FromStr};

pub mod kind;
pub use kind::IsAddressKind;

/// An address captures spending and delegation conditions of assets in the network.
///
/// Addresses can be one of two [`kind`]:
///
/// - [`kind::Byron`]: legacy, not longer used. Also called _"bootstrap"_ addresses sometimes.
/// - [`kind::Shelley`]: most used and modern format, which can bear delegation rights.
///
/// An [`Address`] can be constructed in a variety of ways.
///
/// 1. Either directly using the provided builder:
///    - [`Address<kind::Shelley>::new`]
///    - [`Address<kind::Shelley>::with_delegation`]
///
/// 2. Using the [`address!`](crate::address!) or [`address_test!`](crate::address_test!) macros.
///
/// 3. Or by converting from another representation (e.g. bech32, base58 or base16 text strings, or
///    raw bytes):
///
///    ```rust
///    # use cardano_tx_builder::{Address, address::kind};
///    // Parse a string as Shelley address; will fail if presented with a Byron address:
///    assert!(
///      <Address<kind::Shelley>>::try_from(
///        "addr1v83gkkw3nqzakg5xynlurqcfqhgd65vkfvf5xv8tx25ufds2yvy2h"
///      ).is_ok()
///    );
///
///    assert!(
///      <Address<kind::Shelley>>::try_from(
///        "Ae2tdPwUPEYwNguM7TB3dMnZMfZxn1pjGHyGdjaF4mFqZF9L3bj6cdhiH8t"
///      ).is_err()
///    );
///    ```
///
///    ```rust
///    # use cardano_tx_builder::{Address, address::kind};
///    // Parse a string as any address; will also success on Byron addresses:
///    assert!(
///      <Address<kind::Any>>::try_from(
///        "addr1v83gkkw3nqzakg5xynlurqcfqhgd65vkfvf5xv8tx25ufds2yvy2h"
///      ).is_ok()
///    );
///
///    assert!(
///      <Address<kind::Any>>::try_from(
///        "Ae2tdPwUPEYwNguM7TB3dMnZMfZxn1pjGHyGdjaF4mFqZF9L3bj6cdhiH8t"
///      ).is_ok()
///    );
///    ```
///
///    ```rust
///    # use cardano_tx_builder::{Address, address::kind};
///    // Also work with base16 encoded addresses:
///    assert!(
///      <Address<kind::Shelley>>::try_from(
///        "61e28b59d19805db228624ffc1830905d0dd51964b134330eb32a9c4b6"
///      ).is_ok()
///    );
///    ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address<T: IsAddressKind>(Rc<AddressKind>, PhantomData<T>);

impl<T: IsAddressKind + Eq> PartialOrd for Address<T> {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.cmp(rhs))
    }
}

impl<T: IsAddressKind + Eq> Ord for Address<T> {
    fn cmp(&self, rhs: &Self) -> Ordering {
        <Vec<u8>>::from(self).cmp(&<Vec<u8>>::from(rhs))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AddressKind {
    Byron(pallas::ByronAddress),
    Shelley(pallas::ShelleyAddress),
}

// ------------------------------------------------------ Building (Shelley)

impl Address<kind::Shelley> {
    /// See also [`address!`](crate::address!)/[`address_test!`](crate::address_test!)
    pub fn new(network: NetworkId, payment_credential: Credential) -> Self {
        Self::from(pallas::ShelleyAddress::new(
            pallas::Network::from(network),
            pallas::ShelleyPaymentPart::from(payment_credential),
            pallas::ShelleyDelegationPart::Null,
        ))
    }

    /// See also [`address!`](crate::address!)/[`address_test!`](crate::address_test!)
    pub fn with_delegation(mut self, delegation_credential: Credential) -> Self {
        self = Self::from(pallas::ShelleyAddress::new(
            pallas::Network::from(self.network_id()),
            pallas::ShelleyPaymentPart::from(self.payment_credential()),
            pallas::ShelleyDelegationPart::from(delegation_credential),
        ));

        self
    }
}

// ---------------------------------------------------- Inspecting (Shelley)

impl Address<kind::Shelley> {
    fn cast(&self) -> &pallas::ShelleyAddress {
        match self.0.as_ref() {
            AddressKind::Shelley(shelley) => shelley,
            _ => unreachable!(),
        }
    }

    // NOTE: Technically, this method should also be available on Byron kind. But that requires
    // accessing the internal address attributes, which Pallas doesn't provide support for and this
    // is quite out of scope of our mission right now.
    pub fn network_id(&self) -> NetworkId {
        NetworkId::from(self.cast().network())
    }

    pub fn payment_credential(&self) -> Credential {
        Credential::from(self.cast().payment())
    }

    pub fn delegation_credential(&self) -> Option<Credential> {
        Credential::try_from(self.cast().delegation()).ok()
    }
}

// ------------------------------------------------------------- Constructing (Any)

impl Default for Address<kind::Any> {
    fn default() -> Self {
        Self::from(
            Address::new(NetworkId::mainnet(), Credential::default())
                .with_delegation(Credential::default()),
        )
    }
}

// ------------------------------------------------------------- Inspecting (Any)

impl<T: IsAddressKind> Address<T> {
    /// Check whether an address is a [`kind::Byron`] address. To carry this proof at the
    /// type-level, use [`Self::as_byron`].
    ///
    /// # examples
    ///
    /// ```rust
    /// # use cardano_tx_builder::{address};
    /// assert_eq!(
    ///     address!(
    ///         "37btjrVyb4KDXBNC4haBVPCrro8AQPHwvCMp3R\
    ///          FhhSVWwfFmZ6wwzSK6JK1hY6wHNmtrpTf1kdbv\
    ///          a8TCneM2YsiXT7mrzT21EacHnPpz5YyUdj64na"
    ///     ).is_byron(),
    ///     true,
    /// );
    /// ```
    ///
    /// ```rust
    /// # use cardano_tx_builder::{address};
    /// assert_eq!(
    ///     address!("addr1v83gkkw3nqzakg5xynlurqcfqhgd65vkfvf5xv8tx25ufds2yvy2h").is_byron(),
    ///     false,
    /// );
    /// ```
    pub fn is_byron(&self) -> bool {
        matches!(self.0.as_ref(), &AddressKind::Byron(..))
    }

    /// Refine the kind of the address, assuming it is a [`kind::Byron`] to enable specific methods
    /// for this kind.
    pub fn as_byron(&self) -> Option<Address<kind::Byron>> {
        if self.is_byron() {
            return Some(Address(self.0.clone(), PhantomData));
        }

        None
    }

    /// Check whether an address is a [`kind::Byron`] address. To carry this proof at the
    /// type-level, use [`Self::as_shelley`].
    ///
    /// # examples
    ///
    /// ```rust
    /// # use cardano_tx_builder::{address};
    /// assert_eq!(
    ///     address!(
    ///         "37btjrVyb4KDXBNC4haBVPCrro8AQPHwvCMp3R\
    ///          FhhSVWwfFmZ6wwzSK6JK1hY6wHNmtrpTf1kdbv\
    ///          a8TCneM2YsiXT7mrzT21EacHnPpz5YyUdj64na"
    ///     ).is_shelley(),
    ///     false,
    /// );
    /// ```
    ///
    /// ```rust
    /// # use cardano_tx_builder::{address};
    /// assert_eq!(
    ///     address!("addr1v83gkkw3nqzakg5xynlurqcfqhgd65vkfvf5xv8tx25ufds2yvy2h").is_shelley(),
    ///     true,
    /// );
    /// ```
    pub fn is_shelley(&self) -> bool {
        matches!(&self.0.as_ref(), AddressKind::Shelley(..))
    }

    /// Refine the kind of the address, assuming it is a [`kind::Shelley`] to enable specific methods
    /// for this kind.
    pub fn as_shelley(&self) -> Option<Address<kind::Shelley>> {
        if self.is_shelley() {
            return Some(Address(self.0.clone(), PhantomData));
        }

        None
    }
}

// ----------------------------------------------------------- Converting (from)

impl From<pallas::ByronAddress> for Address<kind::Byron> {
    fn from(byron_address: pallas::ByronAddress) -> Self {
        Self(Rc::new(AddressKind::Byron(byron_address)), PhantomData)
    }
}

impl From<pallas::ShelleyAddress> for Address<kind::Shelley> {
    fn from(shelley_address: pallas::ShelleyAddress) -> Self {
        Self(Rc::new(AddressKind::Shelley(shelley_address)), PhantomData)
    }
}

impl From<Address<kind::Byron>> for Address<kind::Any> {
    fn from(byron_address: Address<kind::Byron>) -> Self {
        Self(byron_address.0, PhantomData)
    }
}

impl From<Address<kind::Shelley>> for Address<kind::Any> {
    fn from(shelley_address: Address<kind::Shelley>) -> Self {
        Self(shelley_address.0, PhantomData)
    }
}

impl TryFrom<pallas::Address> for Address<kind::Any> {
    type Error = anyhow::Error;

    fn try_from(address: pallas::Address) -> anyhow::Result<Self> {
        match address {
            pallas_addresses::Address::Byron(byron) => Ok(Address::<kind::Any>(
                Rc::new(AddressKind::Byron(byron)),
                PhantomData,
            )),
            pallas_addresses::Address::Shelley(shelley) => Ok(Address::<kind::Any>(
                Rc::new(AddressKind::Shelley(shelley)),
                PhantomData,
            )),
            pallas_addresses::Address::Stake(_) => {
                Err(anyhow!("found stake address masquerading as address"))
            }
        }
    }
}

impl TryFrom<pallas::Address> for Address<kind::Shelley> {
    type Error = anyhow::Error;

    fn try_from(address: pallas::Address) -> anyhow::Result<Self> {
        match address {
            pallas_addresses::Address::Shelley(shelley) => Ok(Address::<kind::Shelley>(
                Rc::new(AddressKind::Shelley(shelley)),
                PhantomData,
            )),
            pallas_addresses::Address::Byron(_) | pallas_addresses::Address::Stake(_) => {
                Err(anyhow!("not a shelley address"))
            }
        }
    }
}

impl<T: IsAddressKind> TryFrom<&str> for Address<T>
where
    Address<T>: TryFrom<pallas::Address, Error = anyhow::Error>,
{
    type Error = anyhow::Error;

    fn try_from(text: &str) -> anyhow::Result<Self> {
        Self::try_from(pallas::Address::from_str(text).map_err(|e| anyhow!(e))?)
    }
}

impl<T: IsAddressKind> TryFrom<&[u8]> for Address<T>
where
    Address<T>: TryFrom<pallas::Address, Error = anyhow::Error>,
{
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> anyhow::Result<Self> {
        Self::try_from(pallas::Address::from_bytes(bytes).map_err(|e| anyhow!(e))?)
    }
}

// --------------------------------------------------------------- Converting (to)

impl<T: IsAddressKind> fmt::Display for Address<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self.0.as_ref() {
            AddressKind::Byron(byron) => f.write_str(byron.to_base58().as_str()),
            AddressKind::Shelley(shelley) => f.write_str(
                shelley
                    .to_bech32()
                    .expect("failed to convert to bech32!?")
                    .as_str(),
            ),
        }
    }
}

impl<T: IsAddressKind> From<&Address<T>> for Vec<u8> {
    fn from(address: &Address<T>) -> Self {
        match address.0.as_ref() {
            AddressKind::Byron(byron) => byron.to_vec(),
            AddressKind::Shelley(shelley) => shelley.to_vec(),
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
pub mod tests {
    use crate::{Address, address::kind::*, any};
    use proptest::{option, prelude::*};

    // -------------------------------------------------------------- Generators

    pub mod generators {
        use super::*;

        prop_compose! {
            pub fn address_shelley()(
                network_id in any::network_id(),
                payment_credential in any::credential(),
                delegation_credential_opt in option::of(any::credential()),
            ) -> Address<Shelley> {
                let address = Address::new(network_id, payment_credential);

                if let Some(delegation_credential) = delegation_credential_opt {
                    return address.with_delegation(delegation_credential)
                }

                address
            }
        }
    }
}

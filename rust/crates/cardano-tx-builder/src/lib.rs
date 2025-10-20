//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

/// A re-export of [`minicbor`](https://docs.rs/minicbor/latest/minicbor/index.html).
pub mod cbor;
pub use cardano::{
    // Re-export types for easier consumption.
    //
    // NOTE: The main function of this comment is to force the formatter to put one import per
    // line in the following import list; making diffs and extensions easier.
    address::{Address, IsAddressKind},
    credential::Credential,
    crypto::ed25519::{Signature, SigningKey, VerificationKey},
    datum::Datum,
    execution_units::ExecutionUnits,
    hash::Hash,
    input::Input,
    network_id::NetworkId,
    output::Output,
    output::change_strategy::ChangeStrategy,
    plutus_data::PlutusData,
    plutus_script::PlutusScript,
    plutus_version::PlutusVersion,
    protocol_parameters::ProtocolParameters,
    redeemer_pointer::RedeemerPointer,
    transaction::{IsTransactionBodyState, Transaction},
    value::Value,
};

/// A namespace for address-related sub-modules.
pub use cardano::address;

/// A namespace for transaction-related sub-modules.
pub use cardano::transaction;

pub use cardano::macros;

#[cfg(any(test, feature = "test-utils"))]
pub mod any {
    pub use super::cardano::{
        address::tests::generators::*, credential::tests::generators::*,
        execution_units::tests::generators::*, hash::tests::generators::*,
        input::tests::generators::*, network_id::tests::generators::*,
    };
}

mod cardano;
mod non_extensible;
mod pallas;
mod pretty;

pub(crate) type BoxedIterator<'iter, T> = Box<dyn Iterator<Item = T> + 'iter>;

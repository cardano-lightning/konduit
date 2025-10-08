//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

#[derive(Debug)]
pub struct ProtocolParameters {
    /// Multiplier coefficient on fee, in lovelace/bytes
    fee_per_byte: u64,

    /// Flat/fixed fee, in lovelace
    fee_constant: u64,

    /// Price of a single memory execution unit, in lovelace/unit
    price_mem: f64,

    /// Price of a single cpu execution unit, in lovelace/unit
    price_cpu: f64,

    /// The multiplier coefficient to apply to fees to obtain the collateral, in lovelace
    collateral_coefficient: f64,

    /// Cost model for Plutus version 3
    plutus_v3_cost_model: PlutusV3CostModel,

    /// The network POSIX start time, in seconds.
    start_time: u64,

    /// The first (not-necessarily active) slot of the Shelley era.
    first_shelley_slot: u64,
}

type PlutusV3CostModel = [i64; 297];

// ------------------------------------------------------------------ Inspecting

impl ProtocolParameters {
    /// Base transaction fee, computed from the size of a serialised transaction.
    pub fn base_fee(&self, size: u64) -> u64 {
        size * self.fee_per_byte + self.fee_constant
    }

    pub fn collateral_coefficient(&self) -> f64 {
        self.collateral_coefficient
    }

    pub fn price_mem(&self, execution_units: u64) -> u64 {
        (self.price_mem * execution_units as f64).ceil() as u64
    }

    pub fn price_cpu(&self, execution_units: u64) -> u64 {
        (self.price_cpu * execution_units as f64).ceil() as u64
    }

    pub fn plutus_v3_cost_model(&self) -> &PlutusV3CostModel {
        &self.plutus_v3_cost_model
    }
}

// --------------------------------------------------------------------- Building

impl Default for ProtocolParameters {
    fn default() -> Self {
        Self {
            fee_per_byte: 0,
            fee_constant: 0,
            price_mem: 0.0,
            price_cpu: 0.0,
            collateral_coefficient: 1.5,
            plutus_v3_cost_model: [
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
            start_time: 0,
            first_shelley_slot: 0,
        }
    }
}

impl ProtocolParameters {
    pub fn with_fee_per_byte(mut self, fee_per_byte: u64) -> Self {
        self.fee_per_byte = fee_per_byte;
        self
    }

    pub fn with_fee_constant(mut self, fee_constant: u64) -> Self {
        self.fee_constant = fee_constant;
        self
    }

    pub fn with_collateral_coefficient(mut self, collateral_coefficient: f64) -> Self {
        self.collateral_coefficient = collateral_coefficient;
        self
    }

    pub fn with_execution_price_mem(mut self, price_mem: f64) -> Self {
        self.price_mem = price_mem;
        self
    }

    pub fn with_execution_price_cpu(mut self, price_cpu: f64) -> Self {
        self.price_cpu = price_cpu;
        self
    }

    pub fn with_start_time(mut self, start_time: u64) -> Self {
        self.start_time = start_time;
        self
    }

    pub fn with_first_shelley_slot(mut self, first_shelley_slot: u64) -> Self {
        self.first_shelley_slot = first_shelley_slot;
        self
    }

    pub fn with_plutus_v3_cost_model(mut self, cost_model: PlutusV3CostModel) -> Self {
        self.plutus_v3_cost_model = cost_model;
        self
    }
}

// -------------------------------------------------------------- Converting (to)

impl From<&ProtocolParameters> for uplc::tx::SlotConfig {
    fn from(params: &ProtocolParameters) -> Self {
        let byron_slot_length = 20; // in seconds
        Self {
            slot_length: 1000, // Shelley slot length, in milliseconds
            zero_slot: params.first_shelley_slot,
            zero_time: (params.start_time + byron_slot_length * params.first_shelley_slot) * 1000,
        }
    }
}

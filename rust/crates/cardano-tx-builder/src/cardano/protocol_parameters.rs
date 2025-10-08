//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use num::rational::Ratio;

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

    /// Multiplier coefficient on the size of referenced scripts, in lovelace/bytes
    referenced_scripts_base_fee_per_byte: u64,

    /// Multiplier exponentially increasing the cost of reference scripts at each size step.
    ///
    /// NOTE: This isn't an actual protocol parameter and is currently hard-wired in the ledger.
    /// But it may very well be soon enough.
    referenced_scripts_fee_multiplier: Ratio<u64>,

    /// Size of each step after which the cost of referenced script bytes increases, in bytes
    ///
    /// NOTE: This isn't an actual protocol parameter and is currently hard-wired in the ledger.
    /// But it may very well be soon enough.
    referenced_scripts_fee_step_size: u64,

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

    /// According to https://github.com/IntersectMBO/cardano-ledger/blob/master/docs/adr/2024-08-14_009-refscripts-fee-change.md
    pub fn referenced_scripts_fee(&self, mut size: u64) -> u64 {
        let mut cost: Ratio<u64> = Ratio::ZERO;
        let mut fee_per_byte: Ratio<u64> = Ratio::from(self.referenced_scripts_base_fee_per_byte);

        loop {
            if size < self.referenced_scripts_fee_step_size {
                return (cost + fee_per_byte * size).floor().to_integer();
            }

            cost += fee_per_byte * self.referenced_scripts_fee_step_size;
            fee_per_byte *= self.referenced_scripts_fee_multiplier;
            size -= self.referenced_scripts_fee_step_size;
        }
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
            collateral_coefficient: 0.0,
            referenced_scripts_base_fee_per_byte: 0,
            referenced_scripts_fee_multiplier: Ratio::ONE,
            referenced_scripts_fee_step_size: 0,
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

    pub fn with_referenced_scripts_base_fee_per_byte(
        mut self,
        referenced_scripts_base_fee_per_byte: u64,
    ) -> Self {
        self.referenced_scripts_base_fee_per_byte = referenced_scripts_base_fee_per_byte;
        self
    }

    pub fn with_referenced_scripts_fee_multiplier(
        mut self,
        referenced_scripts_fee_multiplier: Ratio<u64>,
    ) -> Self {
        self.referenced_scripts_fee_multiplier = referenced_scripts_fee_multiplier;
        self
    }

    pub fn with_referenced_scripts_fee_step_size(
        mut self,
        referenced_scripts_fee_step_size: u64,
    ) -> Self {
        self.referenced_scripts_fee_step_size = referenced_scripts_fee_step_size;
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

impl ProtocolParameters {
    pub fn mainnet() -> Self {
        Self::default()
            .with_fee_per_byte(44)
            .with_fee_constant(155381)
            .with_referenced_scripts_base_fee_per_byte(15)
            .with_referenced_scripts_fee_multiplier(Ratio::new(12, 10))
            .with_referenced_scripts_fee_step_size(25000)
            .with_execution_price_mem(0.0577)
            .with_execution_price_cpu(7.21e-05)
            .with_start_time(1506203091)
            .with_first_shelley_slot(4492800)
            .with_plutus_v3_cost_model([
                100788, 420, 1, 1, 1000, 173, 0, 1, 1000, 59957, 4, 1, 11183, 32, 201305, 8356, 4,
                16000, 100, 16000, 100, 16000, 100, 16000, 100, 16000, 100, 16000, 100, 100, 100,
                16000, 100, 94375, 32, 132994, 32, 61462, 4, 72010, 178, 0, 1, 22151, 32, 91189,
                769, 4, 2, 85848, 123203, 7305, -900, 1716, 549, 57, 85848, 0, 1, 1, 1000, 42921,
                4, 2, 24548, 29498, 38, 1, 898148, 27279, 1, 51775, 558, 1, 39184, 1000, 60594, 1,
                141895, 32, 83150, 32, 15299, 32, 76049, 1, 13169, 4, 22100, 10, 28999, 74, 1,
                28999, 74, 1, 43285, 552, 1, 44749, 541, 1, 33852, 32, 68246, 32, 72362, 32, 7243,
                32, 7391, 32, 11546, 32, 85848, 123203, 7305, -900, 1716, 549, 57, 85848, 0, 1,
                90434, 519, 0, 1, 74433, 32, 85848, 123203, 7305, -900, 1716, 549, 57, 85848, 0, 1,
                1, 85848, 123203, 7305, -900, 1716, 549, 57, 85848, 0, 1, 955506, 213312, 0, 2,
                270652, 22588, 4, 1457325, 64566, 4, 20467, 1, 4, 0, 141992, 32, 100788, 420, 1, 1,
                81663, 32, 59498, 32, 20142, 32, 24588, 32, 20744, 32, 25933, 32, 24623, 32,
                43053543, 10, 53384111, 14333, 10, 43574283, 26308, 10, 16000, 100, 16000, 100,
                962335, 18, 2780678, 6, 442008, 1, 52538055, 3756, 18, 267929, 18, 76433006, 8868,
                18, 52948122, 18, 1995836, 36, 3227919, 12, 901022, 1, 166917843, 4307, 36, 284546,
                36, 158221314, 26549, 36, 74698472, 36, 333849714, 1, 254006273, 72, 2174038, 72,
                2261318, 64571, 4, 207616, 8310, 4, 1293828, 28716, 63, 0, 1, 1006041, 43623, 251,
                0, 1, 100181, 726, 719, 0, 1, 100181, 726, 719, 0, 1, 100181, 726, 719, 0, 1,
                107878, 680, 0, 1, 95336, 1, 281145, 18848, 0, 1, 180194, 159, 1, 1, 158519, 8942,
                0, 1, 159378, 8813, 0, 1, 107490, 3298, 1, 106057, 655, 1, 1964219, 24520, 3,
            ])
    }

    pub fn preprod() -> Self {
        Self::mainnet()
            .with_start_time(1654041600)
            .with_first_shelley_slot(86400)
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

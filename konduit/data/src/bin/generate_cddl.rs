//! Generates the konduit-data CDDL spec.
//!
//! Usage:
//!   cargo run -p konduit-data --bin generate_cddl --features cddl > spec/konduit-data.cddl

use cuddly::CddlSpec;
use konduit_data::Duration;
use konduit_data::{
    Cheque, ChequeBody, Indexes, Keytag, Lock, Locked, Pending, Receipt, Secret, Squash,
    SquashBody, Tag, Unlocked, Unpend, Used,
};

#[derive(CddlSpec)]
#[cddl_spec(types(
    Duration, Lock, Secret, Tag, Keytag, Unpend, Indexes, Used, ChequeBody, Pending, SquashBody,
    Locked, Unlocked, Squash, Cheque, Receipt,
))]
struct KonduitDataSpec;

fn main() {
    print!("{}", KonduitDataSpec::cddl());
}

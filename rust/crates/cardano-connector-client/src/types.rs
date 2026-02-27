mod asset_object;
pub use asset_object::{AssetObject, from_asset_objects};

mod input_summary;
pub use input_summary::InputSummary;

mod output_summary;
pub use output_summary::OutputSummary;

mod transaction_summary;
pub use transaction_summary::TransactionSummary;

#[cfg(feature = "wasm")]
pub mod wasm {
    use super::*;
    pub use input_summary::wasm::InputSummary;
    pub use output_summary::wasm::OutputSummary;
    pub use transaction_summary::wasm::TransactionSummary;
}

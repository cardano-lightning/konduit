use crate::types::OutputSummary;
use cardano_sdk::Input;

#[derive(Debug, Clone)]
pub struct InputSummary {
    pub input: Input,
    pub output: OutputSummary,
}

use crate::Stage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct L1Channel {
    pub amount: u64,
    pub stage: Stage,
}

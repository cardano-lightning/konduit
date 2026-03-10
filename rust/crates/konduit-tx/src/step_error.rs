use konduit_data::Duration;

#[derive(Debug, Clone, thiserror::Error)]
pub enum StepError {
    #[error("Step had no effect on variables")]
    NoStep,
    #[error("Lower bound required but not set")]
    NoLower,
    #[error("Too early: Set {0}, Need > {0}")]
    Early(Duration, Duration),
    #[error("Pair (Stage, Step) ({0}, {1}) are incompat")]
    Pair(String, String),
    #[error("Terminal `Expire`. Should be `End`")]
    Expire,
    #[error("Other :: {0}")]
    Other(String),
}

impl StepError {
    /// Constructs a Pair variant from any two types that can be converted to a String
    pub fn pair<T, U>(stage: T, step: U) -> Self
    where
        T: ToString,
        U: ToString,
    {
        Self::Pair(stage.to_string(), step.to_string())
    }
}

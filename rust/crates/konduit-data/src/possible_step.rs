use crate::Duration;

/// Possible steps describe what Consumer can do.
/// In the cases of Elapse and Expire, the possible steps depend on time.
/// If Adaptor submits a transation, then steps may no longer be possible.
/// For example, Consumer cannot expire a pending cheque that has been unlocked,
/// nor elapse a channel after Adaptor has responded.

#[derive(Debug, Clone)]
pub enum PossibleStep {
    Add,
    Close,
    Expire { after: Duration, gain: u64 },
    Elapse { after: Duration },
    End,
}

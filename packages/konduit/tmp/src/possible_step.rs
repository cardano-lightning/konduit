use konduit_data::{Duration, Stage};

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

impl PossibleStep {
    pub fn from_stage(stage: &Stage) -> Vec<Self> {
        match stage {
            Stage::Opened(_, _) => vec![Self::Add, Self::Close],
            Stage::Closed(_, _, elapse_at) => vec![Self::Elapse { after: *elapse_at }],
            Stage::Responded(_, pendings) => {
                if pendings.is_empty() {
                    vec![Self::End]
                } else {
                    pendings
                        .iter()
                        .map(|x| Self::Expire {
                            after: x.timeout,
                            gain: x.amount,
                        })
                        .collect::<Vec<_>>()
                }
            }
        }
    }
}

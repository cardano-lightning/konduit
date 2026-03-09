use konduit_data::{Duration, Receipt};

/// Some intermediary between `Step`, `Intent` and `Evidence`.
///
/// FIXME :: Do we care whether there is redundant information here?
/// For examples: lockeds are not required in a sub; only secrets are needed in unlock
#[derive(Debug, Clone)]
pub enum StepAnd {
    Add {
        amount: u64,
    },
    Sub {
        receipt: Receipt,
        upper: Duration,
    },
    Close {
        upper: Duration,
        close_period: Duration,
    },
    Respond {
        receipt: Receipt,
        upper: Duration,
    },
    Expire {
        lower: Duration,
    },
    Unlock {
        receipt: Receipt,
        upper: Duration,
    },
    End {
        lower: Duration,
    },
    Elapse {
        lower: Duration,
    },
}

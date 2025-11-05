use konduit_data::{MixedCheque, PosixSeconds, Secret, Squash, Unlocked};

pub type Amount = u64;

pub type Timestamp = PosixSeconds;

#[derive(Debug, Clone)]
pub enum Intent {
    Consumer(ConsumerIntent),
    Adaptor(AdaptorIntent),
}

/// TODO: Update and bring back the original docs from the original w/toolz's version.

#[derive(Debug, Clone)]
pub enum ConsumerIntent {
    Add(Amount),
    Close(Timestamp),
    Timeout(Timestamp),
}

impl ConsumerIntent {
    pub fn new_add(amount: Amount) -> Self {
        Self::Add(amount)
    }

    pub fn new_close(upper_bound: Timestamp) -> Self {
        Self::Close(upper_bound)
    }

    pub fn new_timeout(lower_bound: Timestamp) -> Self {
        Self::Timeout(lower_bound)
    }
}

// TODO: Update and bring back the original docs from the original w/toolz's version.

#[derive(Debug, Clone)]
pub enum AdaptorIntent {
    Sub(Squash, Vec<Unlocked>),
    Respond(Squash, Vec<MixedCheque>),
    Unlock(Vec<Secret>, Timestamp),
}

impl AdaptorIntent {
    pub fn new_sub(squash: Squash, unlockeds: Vec<Unlocked>) -> Self {
        Self::Sub(squash, unlockeds)
    }

    pub fn new_respond(squash: Squash, mixed_cheques: Vec<MixedCheque>) -> Self {
        Self::Respond(squash, mixed_cheques)
    }

    pub fn new_unlock(secrets: Vec<Secret>, upper_bound: Timestamp) -> Self {
        Self::Unlock(secrets, upper_bound)
    }
}

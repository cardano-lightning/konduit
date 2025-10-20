use konduit_data::{MixedCheques, Secret, Squash, Timestamp, Unlockeds};

#[derive(Debug, Clone)]
pub enum Intent {
    Consumer(ConsumerIntent),
    Adaptor(AdaptorIntent),
}

/// In the case of `Close` Consumer supplies the tx upper bound.
///
/// In the case of `Timeout` Consumer supplies the tx lower bound.
/// When the datum is `Datum::Closed`, there are two cases:
///
/// 1. The channel has elapsed. It will result in an `Step::Eol(Eol::Elapse(_))` step
/// 2. An error.
///
/// When the datum is `Datum::Responded`, there are three cases:
///
/// 1. There is at least one unpending, and no unpending has expired. It will error.
/// 2. All unpending has expired. Then it will result in an `Step::Eol(Eol::End(_))`
/// 3. Otherwise (there is at least one expired and one not expired). It will result in an
///    `Step::Cont(Cont::Expire(_))`
///
/// The `Unpends` are derived from the timeout.

#[derive(Debug, Clone)]
pub enum ConsumerIntent {
    Add(u64),
    Close(Timestamp),
    Timeout(Timestamp),
}

impl ConsumerIntent {
    pub fn new_add(amount: u64) -> Self {
        Self::Add(amount)
    }

    pub fn new_close(upper_bound: Timestamp) -> Self {
        Self::Close(upper_bound)
    }

    pub fn new_timeout(lower_bound: Timestamp) -> Self {
        Self::Timeout(lower_bound)
    }
}

/// The tx upper bound constraint is deduced from the cheques
/// if any present.
/// In the case of `Unlock` Adaptor supplies a least upper bound.
/// Any pending cheques which expire before time stamp are ignored.
/// The tx constraint is derived from the timeouts of the unlocked pendings.
///
/// The `Unpends` do not have to be in order.
/// They are derived from the channel state.
/// Note that an `Unlock` results in an error if:
///
/// 1. There is nothing to drop.
///
/// Why the timestamp?
/// It allows Adaptor to specify a time bound that is realistic,
/// and ignore the unpends that could cause the tx to fail.

#[derive(Debug, Clone)]
pub enum AdaptorIntent {
    Sub(Squash, Unlockeds),
    Respond(Squash, MixedCheques),
    Unlock(Vec<Secret>, Timestamp),
}

impl AdaptorIntent {
    pub fn new_sub(squash: Squash, unlockeds: Unlockeds) -> Self {
        Self::Sub(squash, unlockeds)
    }

    pub fn new_respond(squash: Squash, mixed_cheques: MixedCheques) -> Self {
        Self::Respond(squash, mixed_cheques)
    }

    pub fn new_unlock(secrets: Vec<Secret>, upper_bound: Timestamp) -> Self {
        Self::Unlock(secrets, upper_bound)
    }
}

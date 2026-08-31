//! This is based on the existing implmenation of receipt.
//! But there subtle divergences between: Inbound and outbound behaviour.
//! The previous implementation is an Inbound receipt.
//! We need here an outbound receipt.

use konduit_data::{
    Cheque, Duration, Indexes, IndexesError, Lock, Locked, Secret, Squash, SquashBody,
    SquashBodyError, Tag, Unlocked, Verified, VerifyError, VerifyingKey,
};
use minicbor::{Decode, Decoder, Encode};
use serde::{Deserialize, Deserializer, Serialize};
use std::cmp;

pub use crate::wire::sync::Receipt as WireReceipt;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("Squash cannot include a (locked) cheque. {0}")]
    IncludesCheque(u64),

    #[error("Squash body was not reproduced")]
    NotReproduced,

    #[error("squash amount less than expected")]
    SquashAmountLess,

    #[error("Bad input")]
    Input,

    #[error("indexes: {0}")]
    SquashBody(#[from] SquashBodyError),

    #[error("indexes: {0}")]
    Indexes(#[from] IndexesError),

    #[error("Expected a change, but none observed")]
    Unchanged,

    #[error("Other")]
    Other,
}

/// From the servers perspective the receipt is always verified.
/// Unfortunately this means we need impl the deserializaiton by hand.

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Encode)]
pub struct Receipt {
    #[n(0)]
    squash: Squash<Verified>,
    #[n(1)]
    cheques: Vec<Cheque<Verified>>,
}

impl From<Receipt> for WireReceipt {
    fn from(value: Receipt) -> Self {
        Self {
            squash: value.squash.into_unverified(),
            cheques: value
                .cheques
                .into_iter()
                .map(|x| x.into_unverified())
                .collect(),
        }
    }
}

/// For deserializaiton and decoding, go via WireReceipt.
/// Note! that this should only be used from trusted sources.
impl From<WireReceipt> for Receipt {
    fn from(value: WireReceipt) -> Self {
        Self {
            squash: value.squash.skip_verify(),
            cheques: value.cheques.into_iter().map(|x| x.skip_verify()).collect(),
        }
    }
}

impl Receipt {
    pub fn try_verify(
        wire: WireReceipt,
        key: &VerifyingKey,
        tag: &Tag,
    ) -> Result<Receipt, VerifyError> {
        Ok(Receipt {
            squash: wire.squash.try_verify(key, tag)?,
            cheques: wire
                .cheques
                .into_iter()
                .map(|x| x.try_verify(key, tag))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

impl<'de> Deserialize<'de> for Receipt {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        WireReceipt::deserialize(deserializer).map(Into::into)
    }
}

impl<'b, C> Decode<'b, C> for Receipt {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        WireReceipt::decode(d, ctx).map(Into::into)
    }
}

impl Receipt {
    pub fn new(squash: Squash<Verified>) -> Self {
        Self {
            squash,
            cheques: vec![],
        }
    }

    /// Internal constructor to associate state markers.
    /// FIXME :: this looks sus.
    pub fn new_with_state(squash: Squash<Verified>, cheques: Vec<Cheque<Verified>>) -> Self {
        Self { squash, cheques }
    }

    // ------------------------------------------------------------------------
    // -- Accessors
    // ------------------------------------------------------------------------

    fn cheques(&self) -> impl Iterator<Item = &Cheque<Verified>> {
        self.cheques.iter()
    }

    pub fn unlockeds(&self) -> impl Iterator<Item = Unlocked<Verified>> {
        self.cheques().filter_map(Cheque::<Verified>::as_unlocked)
    }

    pub fn lockeds(&self) -> impl Iterator<Item = Locked<Verified>> {
        self.cheques().filter_map(Cheque::<Verified>::as_locked)
    }

    pub fn squash(&self) -> &Squash<Verified> {
        &self.squash
    }

    fn max_index(&self) -> u64 {
        let mc_index = self.cheques.last().map(|mc| mc.index()).unwrap_or(0);
        cmp::max(self.squash.index(), mc_index)
    }

    pub fn owed(&self) -> u64 {
        self.squash.amount() + self.unlockeds().map(|x| x.amount()).sum::<u64>()
    }

    pub fn committed(&self) -> u64 {
        self.squash.amount() + self.cheques().map(|x| x.amount()).sum::<u64>()
    }

    // ------------------------------------------------------------------------
    // -- Mutations
    // ------------------------------------------------------------------------

    /// Appends a new locked cheque to the collection if it passes the sequential index check.
    pub fn apply_locked(&mut self, locked: Locked<Verified>) -> Result<(), Error> {
        if locked.index() != self.max_index() + 1 {
            return Err(Error::Input);
        }
        self.cheques.push(Cheque::from(locked));
        Ok(())
    }

    /// Locked -> Unlocked with secret. Err if nothing changes.
    pub fn apply_secret(&mut self, secret: Secret) -> Result<(), Error> {
        let lock = Lock::from(secret);
        let mut changed = Err(Error::Unchanged);
        for cheque in &mut self.cheques {
            if let Cheque::Locked(locked) = cheque
                && locked.lock() == &lock
            {
                *cheque = Cheque::from(
                    Unlocked::<Verified>::try_from_locked(locked, secret)
                        .expect("Already verified!"),
                );
                changed = Ok(());
            }
        }
        changed
    }

    /// Drop all locked cheques for which timeout is <= now.
    /// We assume unlockeds are used, and then persisted for squash proposal.
    pub fn apply_timeout(&mut self, now: Duration) {
        self.cheques
            .retain(|c| c.as_locked().is_none_or(|l| l.timeout() > now));
    }

    /// Replace a locked. FIXME :: Not currently used.
    pub fn apply_replace(&mut self, new_locked: Locked<Verified>) -> Result<(), Error> {
        // Find the existing item by index
        let existing_cheque = self
            .cheques
            .iter_mut()
            .find(|c| c.index() == new_locked.index())
            .ok_or(Error::Input)?; // Assuming you have a NotFound error

        // Ensure it is actually in the Locked state
        let Some(old_locked) = existing_cheque.as_locked() else {
            return Err(Error::Other);
        };

        // Enforce the strict safety invariants
        if new_locked.lock() != old_locked.lock()
            || new_locked.amount() <= old_locked.amount()
            || new_locked.timeout() <= old_locked.timeout()
        {
            return Err(Error::Input);
        }

        // Perform the dangerous replacement
        *existing_cheque = Cheque::from(new_locked);
        Ok(())
    }

    /// Applied squash _ought_ to be following a squash proposal.
    /// When is a squash valid: the squash amount must at least the current squash amount
    /// plus the total of all squashed cheques.
    pub fn apply_squash(&mut self, squash: Squash<Verified>) -> Result<(), Error> {
        let squashed: u64 = self
            .cheques()
            .filter(|c| squash.is_index_squashed(c.index()))
            .map(|c| c.amount())
            .sum();
        match squash.amount().cmp(&(self.squash.amount() + squashed)) {
            cmp::Ordering::Less => return Err(Error::SquashAmountLess),
            cmp::Ordering::Equal => (),
            cmp::Ordering::Greater => (),
        }
        self.cheques
            .retain(|c| !squash.is_index_squashed(c.index()));
        self.squash = squash;
        Ok(())
    }

    /// Apply sync
    pub fn apply_sync(&mut self, their: Receipt) -> Result<(), Error> {
        if self.squash.body() < their.squash.body() {
            self.apply_squash(their.squash.clone())?
        }
        // FIXME!!
        // 2. Cheques: Take all of ours, + theirs: on duplicate take unlocked, and then of highest
        //    amount. Its possible that a new commit has been made since the sync request was made.
        // 3. Drop any now squashed.
        // ... then apply new squash and sync again.
        Ok(())
    }

    // ------------------------------------------------------------------------
    // -- L2 Admin
    // ------------------------------------------------------------------------

    /// Propose the next cheque index
    pub fn propose_index(&self) -> u64 {
        self.max_index() + 1
    }

    pub fn propose_squash_body(&self) -> Result<SquashBody, Error> {
        let mut body = self.squash.body().clone();
        for u in self.unlockeds() {
            body.squash(u.index(), u.amount())
                .map_err(|_| Error::IncludesCheque(u.index()))?
        }
        let exclude = Indexes::new(
            self.cheques
                .iter()
                .map(|x| x.index())
                .filter(|x| *x < body.index())
                .collect(),
        )?;
        let body = SquashBody::new(body.amount(), body.index(), exclude)?;
        Ok(body)
    }

    pub fn maybe_propose_squash_body(&self) -> Result<Option<SquashBody>, Error> {
        let proposal = self.propose_squash_body()?;
        if proposal == *self.squash.body() {
            Ok(None)
        } else {
            Ok(Some(proposal))
        }
    }
}

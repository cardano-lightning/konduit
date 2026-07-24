use konduit_data::{
    Cheque, Duration, Lock, Locked, MAX_UNSQUASHED, Pending, Secret, Squash, SquashBody, Unlocked,
    Used, Verified,
};
use minicbor::{Decode, Decoder, Encode};
use serde::{Deserialize, Deserializer, Serialize};
use std::cmp;

use crate::SquashProposal;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("Squash cannot include a (locked) cheque.")]
    IncludesCheque,

    #[error("Squash body was not reproduced")]
    NotReproduced,

    #[error("Bad input")]
    Input,

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct WireReceipt {
    #[n(0)]
    squash: Squash,
    #[n(1)]
    cheques: Vec<Cheque>,
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
    /// Internal constructor to associate state markers.
    pub fn new_with_state(squash: Squash<Verified>, cheques: Vec<Cheque<Verified>>) -> Self {
        Self { squash, cheques }
    }

    pub fn new(squash: Squash<Verified>) -> Self {
        Self {
            squash,
            cheques: vec![],
        }
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

    // FIXME :: Not currently used
    pub fn capacity(&self) -> usize {
        MAX_UNSQUASHED.saturating_sub(self.cheques.len())
    }

    // ------------------------------------------------------------------------
    // -- Mutations
    // ------------------------------------------------------------------------

    /// Appends a new locked cheque to the collection if it passes the sequential index check.
    pub fn apply_locked(&mut self, locked: Locked<Verified>) -> Result<(), Error> {
        if locked.index() == self.max_index() + 1 {
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

    /// Replace a locked. Do this when Consumer has made a commitment
    /// but Adaptor has not! Happens when a pay fails before BLN commitment.
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

    pub fn apply_squash(&mut self, squash: Squash<Verified>) -> Result<(), Error> {
        let squashed: u64 = self
            .cheques()
            .filter(|c| squash.is_index_squashed(c.index()))
            .map(|c| c.amount())
            .sum();
        if squash.amount() < self.squash.amount() + squashed {
            return Err(Error::Input);
        }
        self.cheques
            .retain(|c| !squash.is_index_squashed(c.index()));
        self.squash = squash;
        Ok(())
    }

    // ------------------------------------------------------------------------
    // -- L1 admin
    // ------------------------------------------------------------------------

    /// Prep for sub step: spendable (unlocked) items for the next tx,
    /// and the running history of used items to appear in the datum.
    pub fn prep_sub(
        &self,
        useds: &[Used],
        upper: &Duration,
    ) -> (Vec<Unlocked<Verified>>, Vec<Used>) {
        let unlockeds: Vec<Unlocked<Verified>> = self
            .cheques
            .iter()
            .filter_map(|c| c.as_unlocked())
            .filter(|u| u.timeout() > *upper && !useds.iter().any(|x| x.index == u.index()))
            .collect();

        let mut next_useds: Vec<Used> = useds
            .iter()
            .filter(|u| !self.squash.is_index_squashed(u.index))
            .cloned()
            .chain(unlockeds.iter().map(Used::from))
            .collect();

        next_useds.sort_by_key(|u| u.index);

        (unlockeds, next_useds)
    }

    /// Prep for respond step: gathers active cheques, pending items,
    /// and aggregates the total amount required to compute a response.
    pub fn prep_respond(
        &self,
        useds: &[Used],
        upper: &Duration,
    ) -> (Vec<Cheque<Verified>>, Vec<Pending>, u64) {
        let mut cheques = Vec::new();
        let mut pendings = Vec::new();

        let mut total_amount: u64 = useds
            .iter()
            .filter(|u| !self.squash.is_index_squashed(u.index))
            .map(|u| u.amount)
            .sum();

        for c in &self.cheques {
            if c.timeout() > *upper && !useds.iter().any(|x| x.index == c.index()) {
                cheques.push(c.clone());

                if let Some(locked) = c.as_locked() {
                    pendings.push(Pending::from(locked));
                } else if let Some(unlocked) = c.as_unlocked() {
                    total_amount += unlocked.amount();
                }
            }
        }

        (cheques, pendings, total_amount)
    }

    /// Aka prep_unlock. Used in unlock steps
    pub fn secrets(&self) -> impl Iterator<Item = Secret> {
        self.unlockeds().map(|x| *x.secret())
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
                .map_err(|_| Error::IncludesCheque)?
        }
        Ok(body)
    }

    /// Propose the next squash state
    pub fn propose_squash(&self) -> Result<SquashProposal, Error> {
        Ok(SquashProposal {
            current: self.squash.clone().into_unverified(),
            unlockeds: self
                .unlockeds()
                .map(|x| x.into_unverified())
                .collect::<Vec<_>>(),
            lockeds: self
                .lockeds()
                .map(|x| x.into_unverified())
                .collect::<Vec<_>>(),
            proposal: self.propose_squash_body()?,
        })
    }
}

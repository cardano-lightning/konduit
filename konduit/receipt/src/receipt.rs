use konduit_data::{
    Cheque, Duration, Lock, Locked, MAX_UNSQUASHED, Pending, Secret, Squash, Tag, Unlocked,
    Unverified, Used, Verified, VerifyError, VerifyState, VerifyingKey,
};
use minicbor::Encode;
use serde::{Deserialize, Serialize};
use std::cmp;

mod error;
pub use error::{Error, Result};

use serde_with::serde_as;

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode)]
pub struct Receipt<V: VerifyState = Unverified> {
    #[n(0)]
    squash: Squash<V>,
    #[n(1)]
    cheques: Vec<Cheque<V>>,
}

// Cannot automagically derive this cos `Verified` is not supported.
impl<'b, Ctx> minicbor::Decode<'b, Ctx> for Receipt<Unverified>
where
    Squash<Unverified>: minicbor::Decode<'b, Ctx>,
    Cheque<Unverified>: minicbor::Decode<'b, Ctx>,
{
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        ctx: &mut Ctx,
    ) -> std::result::Result<Self, minicbor::decode::Error> {
        d.array()?;
        Ok(Self {
            squash: minicbor::Decode::decode(d, ctx)?,
            cheques: minicbor::Decode::decode(d, ctx)?,
        })
    }
}

impl<V: VerifyState + Clone> Receipt<V> {
    pub fn new(squash: Squash<V>) -> Self {
        Self {
            squash,
            cheques: vec![],
        }
    }
    // ------------------------------------------------------------------------
    // -- Accessors
    // ------------------------------------------------------------------------

    fn cheques(&self) -> impl Iterator<Item = &Cheque<V>> {
        self.cheques.iter()
    }

    fn unlockeds(&self) -> impl Iterator<Item = Unlocked<V>> {
        self.cheques().filter_map(Cheque::<V>::as_unlocked)
    }

    pub fn squash(&self) -> &Squash<V> {
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

    pub fn apply_secret(&mut self, secret: Secret) -> Result<()> {
        let lock = Lock::from(secret.clone());
        let mut changed = Err(Error::Unchanged);
        for cheque in &mut self.cheques {
            if let Cheque::Locked(locked) = cheque {
                if locked.lock() == &lock {
                    *cheque = Cheque::from(
                        Unlocked::<V>::try_from_locked(&locked, secret).expect("Already verified!"),
                    );
                    changed = Ok(());
                }
            }
        }
        changed
    }

    pub fn apply_squash(&mut self, squash: Squash<V>) -> Result<()> {
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

    pub fn apply_replace(&mut self, new_locked: Locked<V>) -> Result<()> {
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

    /// Appends a new locked cheque to the collection if it passes the sequential index check.
    pub fn apply_locked(&mut self, locked: Locked<V>) -> Result<()> {
        if locked.index() == self.max_index() + 1 {
            return Err(Error::Input);
        }
        self.cheques.push(Cheque::from(locked));
        Ok(())
    }

    /// Drop all locked cheques for which timeout is <= now.
    /// We assume unlockeds are used, and then persisted for squash proposal.
    pub fn apply_timeout(&mut self, now: Duration) {
        self.cheques
            .retain(|c| c.as_locked().map_or(true, |l| l.timeout() > now));
    }

    // ------------------------------------------------------------------------
    // -- L1 admin
    // ------------------------------------------------------------------------

    /// Prep for sub step: spendable (unlocked) items for the next tx,
    /// and the running history of used items to appear in the datum.
    pub fn prep_sub(&self, useds: &[Used], upper: &Duration) -> (Vec<Unlocked<V>>, Vec<Used>) {
        let unlockeds: Vec<Unlocked<V>> = self
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
    ) -> (Vec<Cheque<V>>, Vec<Pending>, u64) {
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

    /// Used in unlock steps
    pub fn secrets(&self) -> impl Iterator<Item = Secret> {
        self.unlockeds().map(|x| x.secret().clone())
    }

    // ------------------------------------------------------------------------
    // -- L2 Admin
    // ------------------------------------------------------------------------

    /// Propose the next squash state
    pub fn propose_squash(&self) {
        todo!()
    }
}

// -------------------------------------------------------------------------
// Unverified State Methods
// -------------------------------------------------------------------------
impl Receipt<Unverified> {
    /// Verifies the cryptographic signature against the verifying key and tag.
    /// Iterates over all components.
    pub fn try_verify(
        self,
        verification_key: &VerifyingKey,
        tag: &Tag,
    ) -> std::result::Result<Receipt<Verified>, VerifyError> {
        todo!()
    }

    /// Skips verify. Use when sourcing data form trusted sources eg own database
    pub fn skip_verify() {
        todo!()
    }
}

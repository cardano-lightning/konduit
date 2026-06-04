use konduit_data::{
    Cheque, Duration, Lock, Locked, MAX_UNSQUASHED, Pending, Secret, Squash, SquashBody, Tag,
    Unlocked, Unverified, Used, Verified, VerifyError, VerifyState, VerifyingKey,
};
use minicbor::Encode;
use serde::{Deserialize, Serialize};
use std::{cmp, marker::PhantomData};

mod error;
pub use error::Error;

use crate::SquashProposal;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode)]
#[serde(bound(deserialize = "V : Default"))]
pub struct Receipt<V: VerifyState = Unverified> {
    #[n(0)]
    squash: Squash<V>,
    #[n(1)]
    cheques: Vec<Cheque<V>>,
    #[serde(skip)]
    #[cbor(skip)]
    _marker: PhantomData<V>,
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
            _marker: PhantomData,
        })
    }
}

impl<V: VerifyState + Clone> Receipt<V> {
    /// Internal constructor to associate state markers.
    pub fn new_with_state(squash: Squash<V>, cheques: Vec<Cheque<V>>) -> Self {
        Self {
            squash,
            cheques,
            _marker: PhantomData,
        }
    }

    pub fn new(squash: Squash<V>) -> Self {
        Self {
            squash,
            cheques: vec![],
            _marker: PhantomData,
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

    fn lockeds(&self) -> impl Iterator<Item = Locked<V>> {
        self.cheques().filter_map(Cheque::<V>::as_locked)
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

    /// Appends a new locked cheque to the collection if it passes the sequential index check.
    pub fn apply_locked(&mut self, locked: Locked<V>) -> Result<(), Error> {
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
                    Unlocked::<V>::try_from_locked(locked, secret).expect("Already verified!"),
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
    pub fn apply_replace(&mut self, new_locked: Locked<V>) -> Result<(), Error> {
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

    pub fn apply_squash(&mut self, squash: Squash<V>) -> Result<(), Error> {
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
    pub fn propose_squash(&self) -> Result<SquashProposal<V>, Error> {
        Ok(SquashProposal {
            proposal: self.propose_squash_body()?,
            current: self.squash.clone(),
            unlockeds: self.unlockeds().collect::<Vec<_>>(),
            lockeds: self.lockeds().collect::<Vec<_>>(),
        })
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
        Ok(Receipt::new_with_state(
            self.squash
                .try_verify(verification_key, tag)
                .map_err(|_| VerifyError::InvalidSignature)?,
            self.cheques
                .into_iter()
                .map(|x| {
                    x.try_verify(verification_key, tag)
                        .map_err(|_| VerifyError::InvalidSignature)
                })
                .collect::<Result<Vec<_>, VerifyError>>()?,
        ))
    }

    /// The unsafe version. Suitable when the data comes from a trusted source,
    /// such as your own database.
    pub fn skip_verify(self) -> Receipt<Verified> {
        Receipt::new_with_state(
            self.squash.skip_verify(),
            self.cheques.into_iter().map(|x| x.skip_verify()).collect(),
        )
    }
}

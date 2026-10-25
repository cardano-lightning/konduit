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

        // Even if the user thinkg this is too much, they did already sign it.
        if squash.amount() < self.squash.amount() + squashed {
            return Err(Error::SquashAmountLess);
        }

        self.cheques
            .retain(|c| !squash.is_index_squashed(c.index()));
        self.squash = squash;
        Ok(())
    }

    pub fn apply_sync(&mut self, their: Receipt) -> Result<(), Error> {
        let _ = self.apply_squash(their.squash.clone());
        // TODO:: testme
        let mut merged: std::collections::BTreeMap<u64, Cheque<Verified>> =
            std::collections::BTreeMap::new();
        for cheque in self.cheques.drain(..).chain(their.cheques) {
            merged
                .entry(cheque.index())
                .and_modify(|kept| {
                    if prefer(&cheque, kept) {
                        *kept = cheque.clone();
                    }
                })
                .or_insert(cheque);
        }
        self.cheques = merged.into_values().collect();

        // 3. Drop any now squashed.
        let squash = &self.squash;
        self.cheques
            .retain(|c| !squash.is_index_squashed(c.index()));

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
                .filter(|c| !c.as_unlocked().is_some())
                .map(|c| c.index())
                .filter(|i| *i < body.index())
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

fn prefer(candidate: &Cheque<Verified>, kept: &Cheque<Verified>) -> bool {
    match (
        candidate.as_unlocked().is_some(),
        kept.as_unlocked().is_some(),
    ) {
        (true, false) => true,
        (false, true) => false,
        _ => candidate.amount() > kept.amount(),
    }
}

#[cfg(test)]
mod tests {
    use crate::{Signer, signer, time};

    use super::*;
    use konduit_data::{ChequeBody, Lock, Secret, SquashBody, Tag};

    fn test_signer() -> Signer {
        Signer::new(signer::Config { key: [0; 32] })
    }

    fn tag() -> Tag {
        Tag::from(vec![0, 1, 2, 3])
    }

    fn zero_receipt(signer: &Signer, tag: &Tag) -> Receipt {
        Receipt::new(signer.squash(tag.clone(), SquashBody::zero()))
    }

    fn with_locked(receipt: &mut Receipt, signer: &Signer, tag: &Tag, amount: u64) -> (u64, Lock) {
        let index = receipt.max_index() + 1;
        let secret = Secret(rand_bytes());
        let lock = Lock::from(secret);
        let body = ChequeBody::new(index, amount, far_future(), lock.clone());
        let locked = signer.locked(tag.clone(), body);
        receipt.apply_locked(locked).unwrap();
        (index, lock)
    }

    fn with_unlocked(receipt: &mut Receipt, signer: &Signer, tag: &Tag, amount: u64) -> u64 {
        let index = receipt.max_index() + 1;
        let secret = Secret(rand_bytes());
        let lock = Lock::from(secret.clone());
        let body = ChequeBody::new(index, amount, far_future(), lock);
        let locked = signer.locked(tag.clone(), body);
        receipt.apply_locked(locked).unwrap();
        receipt.apply_secret(secret).unwrap();
        index
    }

    fn far_future() -> konduit_data::Duration {
        time::now() + Duration::from_secs(10000000000)
    }

    /// Distinct bytes per call, no real randomness needed — a process-wide
    /// counter is enough to guarantee every cheque in every test gets a
    /// unique Lock/Secret, avoiding accidental collisions between cheques
    /// that are supposed to be independent.
    fn rand_bytes() -> [u8; 32] {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut bytes = [0u8; 32];
        bytes[..8].copy_from_slice(&n.to_le_bytes());
        bytes
    }

    // --- 1. squash adoption gate ---

    #[test]
    fn adopts_squash_when_theirs_is_ahead() {
        let signer = test_signer();
        let tag = tag();
        let mut ours = zero_receipt(&signer, &tag);
        with_unlocked(&mut ours, &signer, &tag, 100);

        let body = ours.propose_squash_body().unwrap();
        let mut theirs = zero_receipt(&signer, &tag);
        theirs
            .apply_squash(signer.squash(tag.clone(), body.clone()))
            .unwrap();

        ours.apply_sync(theirs).unwrap();
        assert_eq!(ours.squash.body(), &body);
    }

    #[test]
    fn ignores_squash_when_theirs_is_behind_or_equal() {
        let signer = test_signer();
        let tag = tag();
        let mut ours = zero_receipt(&signer, &tag);
        with_unlocked(&mut ours, &signer, &tag, 100);
        let ours_body = ours.propose_squash_body().unwrap();
        ours.apply_squash(signer.squash(tag.clone(), ours_body.clone()))
            .unwrap();

        let theirs = zero_receipt(&signer, &tag); // still at zero — behind ours
        ours.apply_sync(theirs).unwrap();
        assert_eq!(ours.squash.body(), &ours_body); // unchanged
    }

    // --- 2. cheque merge ---

    #[test]
    fn keeps_cheque_only_we_have() {
        let signer = test_signer();
        let tag = tag();
        let mut ours = zero_receipt(&signer, &tag);
        let (idx, _) = with_locked(&mut ours, &signer, &tag, 100);

        let theirs = zero_receipt(&signer, &tag); // no cheques
        ours.apply_sync(theirs).unwrap();

        assert_eq!(
            ours.cheques.iter().map(|c| c.index()).collect::<Vec<_>>(),
            vec![idx]
        );
    }

    #[test]
    fn adopts_cheque_only_they_have() {
        let signer = test_signer();
        let tag = tag();
        let mut ours = zero_receipt(&signer, &tag); // no cheques

        let mut theirs = zero_receipt(&signer, &tag);
        let (idx, _) = with_locked(&mut theirs, &signer, &tag, 200);

        ours.apply_sync(theirs).unwrap();
        assert_eq!(
            ours.cheques.iter().map(|c| c.index()).collect::<Vec<_>>(),
            vec![idx]
        );
    }

    #[test]
    fn unlocked_beats_locked_at_same_index() {
        let signer = test_signer();
        let tag = tag();

        // Both receipts start fresh and call the helper once, so they
        // land on the same index independently — no need to hand-pick one.
        let mut ours = zero_receipt(&signer, &tag);
        let (idx, _) = with_locked(&mut ours, &signer, &tag, 999); // stale, still locked on our side

        let mut theirs = zero_receipt(&signer, &tag);
        let their_idx = with_unlocked(&mut theirs, &signer, &tag, 999); // they've revealed it
        assert_eq!(idx, their_idx);

        ours.apply_sync(theirs).unwrap();
        let kept = ours.cheques.iter().find(|c| c.index() == idx).unwrap();
        assert!(kept.as_unlocked().is_some());
    }

    #[test]
    fn higher_amount_wins_when_both_same_lock_state() {
        let signer = test_signer();
        let tag = tag();

        let mut ours = zero_receipt(&signer, &tag);
        let (idx, _) = with_locked(&mut ours, &signer, &tag, 100);

        let mut theirs = zero_receipt(&signer, &tag);
        let their_idx = with_locked(&mut theirs, &signer, &tag, 150).0; // a later, larger commit
        assert_eq!(idx, their_idx);

        ours.apply_sync(theirs).unwrap();
        let kept = ours.cheques.iter().find(|c| c.index() == idx).unwrap();
        assert_eq!(kept.amount(), 150);
    }

    // --- 3. pruning after squash adoption ---

    #[test]
    fn drops_cheques_covered_by_adopted_squash() {
        let signer = test_signer();
        let tag = tag();

        let mut ours = zero_receipt(&signer, &tag);
        let idx0 = with_unlocked(&mut ours, &signer, &tag, 100);
        let (idx1, _) = with_locked(&mut ours, &signer, &tag, 50); // stays live: not squashed by peer

        // Peer independently reaches the same state (same call order ->
        // same indices), then squashes just the unlocked one.
        let mut theirs = zero_receipt(&signer, &tag);
        let their_idx0 = with_unlocked(&mut theirs, &signer, &tag, 100);
        let (their_idx1, _) = with_locked(&mut theirs, &signer, &tag, 50);
        assert_eq!((idx0, idx1), (their_idx0, their_idx1));

        let body = theirs.propose_squash_body().unwrap();
        theirs
            .apply_squash(signer.squash(tag.clone(), body))
            .unwrap();

        ours.apply_sync(theirs).unwrap();

        let indexes: Vec<_> = ours.cheques.iter().map(|c| c.index()).collect();
        assert!(!indexes.contains(&idx0), "squashed cheque should be pruned");
        assert!(indexes.contains(&idx1), "excluded cheque should survive");
    }
}

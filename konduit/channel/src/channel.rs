//! A channel is what the adaptor recognizes as the channel:
//! All the state and logic associated to the adaptors needs.
//! This includes:
//!
//! + utxos at a kernel address backing the channel
//! + cheques and squashes
//! + state concerning the users off-chain usage such resource usage

use cardano_sdk::VerificationKey;
use konduit_data::{Keytag, Locked, Receipt, Secret, Squash, Tag};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::Error;
use crate::backing::Backing;
use crate::nota::Nota;

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Channel {
    #[serde_as(as = "serde_with::hex::Hex")]
    #[cbor(n(0), with = "cbor_with::display_from_str")]
    key: VerificationKey,
    #[serde_as(as = "serde_with::hex::Hex")]
    #[cbor(n(1), with = "cbor_with::display_from_str")]
    tag: Tag,
    #[n(2)]
    backing: Backing,
    #[cbor(n(3), with = "cbor_with::nullable_same")]
    receipt: Option<Receipt>,
    #[n(4)]
    nota: Nota,
}

impl Channel {
    /// Open a new channel for a consumer identified by `keytag`, with the
    /// given rate-limit/quota configuration in `nota`. Starts with empty
    /// backing and no receipt.
    pub fn new(keytag: Keytag, nota: Nota) -> Self {
        let (key, tag) = keytag.split();
        Self {
            key,
            tag,
            backing: Backing::empty(),
            receipt: None,
            nota,
        }
    }

    // --- Accessors ----------------------------------------------------------

    pub fn key(&self) -> &VerificationKey {
        &self.key
    }
    pub fn tag(&self) -> &Tag {
        &self.tag
    }
    pub fn receipt(&self) -> Option<&Receipt> {
        self.receipt.as_ref()
    }
    pub fn backing(&self) -> &Backing {
        &self.backing
    }
    pub fn backing_mut(&mut self) -> &mut Backing {
        &mut self.backing
    }
    pub fn nota(&self) -> &Nota {
        &self.nota
    }
    pub fn nota_mut(&mut self) -> &mut Nota {
        &mut self.nota
    }

    // --- Events -------------------------------------------------------------

    /// Apply a consumer-signed squash.
    /// Creates the receipt if this is the first squash; advances it otherwise.
    pub fn apply_squash(&mut self, squash: Squash) -> crate::Result<()> {
        if !squash.verify(&self.key, &self.tag) {
            return Err(Error::Input);
        }
        match &mut self.receipt {
            None => {
                self.receipt = Some(Receipt::new(squash));
            }
            Some(r) => {
                r.update_squash(squash);
            }
        }
        Ok(())
    }

    /// Append a consumer-signed locked cheque.
    /// Verifies the signature and checks available funds before accepting.
    pub fn apply_locked(&mut self, locked: Locked) -> crate::Result<()> {
        if !locked.verify(&self.key, &self.tag) {
            return Err(Error::Input);
        }
        let available = self.spendable()?;
        if locked.amount() > available {
            return Err(Error::Funds);
        }
        self.receipt
            .as_mut()
            .ok_or(Error::Receipt)?
            .append_locked(locked)
            .map_err(|_| Error::Input)
    }

    /// Resolve a locked cheque with its payment preimage.
    pub fn apply_unlock(&mut self, secret: Secret) -> crate::Result<()> {
        self.receipt
            .as_mut()
            .ok_or(Error::Receipt)?
            .unlock(secret)
            .map_err(|_| Error::Input)
    }

    // --- Queries ------------------------------------------------------------

    /// The next squash the server expects the client to sign.
    pub fn squash_proposal(&self) -> crate::Result<SquashProposal> {
        self.receipt
            .as_ref()
            .ok_or(Error::Receipt)?
            .squash_proposal()
            .map_err(|_| Error::Input)
    }

    /// Amount available for new payments, given current on-chain backing and receipt state.
    pub fn spendable(&self) -> crate::Result<u64> {
        let backing = self.backing.best_live().ok_or(Error::Backing)?;
        let receipt = self.receipt.as_ref().ok_or(Error::Receipt)?;
        if receipt.capacity() == 0 {
            return Err(Error::Capacity);
        }
        let owed = receipt.potentially_subable(backing.useds());
        let rel = owed.saturating_sub(backing.subbed());
        Ok(backing.amount().saturating_sub(rel))
    }

    /// The index to assign to the next cheque.
    pub fn next_index(&self) -> crate::Result<u64> {
        let backing = self.backing.best_live().ok_or(Error::Backing)?;
        let receipt = self.receipt.as_ref().ok_or(Error::Receipt)?;
        let chain_max = backing.useds().last().map(|u| u.index);
        let index = match chain_max {
            None => receipt.max_index(),
            Some(i) => receipt.max_index().max(i),
        };
        Ok(index + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nota::Limit;
    use crate::{
        Error,
        backing::{
            BackingUtxo, Chain,
            cardano::{BlockHeight, OutputReference},
        },
    };
    use cardano_sdk::{SigningKey, VerificationKey};
    use konduit_data::{ChequeBody, Duration, Indexes, Lock, Secret, SquashBody};

    // -------------------------------------------------------------------------
    // Test helpers
    // -------------------------------------------------------------------------

    fn test_sk() -> SigningKey {
        SigningKey::from([7u8; 32])
    }

    fn test_tag() -> Tag {
        Tag::from([3u8; 20].to_vec())
    }

    fn test_keytag() -> Keytag {
        let sk = test_sk();
        let vk = VerificationKey::from(&sk);
        Keytag::new(vk, test_tag())
    }

    /// A Nota with unlimited capacity — suitable for tests that don't exercise limits.
    fn unlimited_nota() -> Nota {
        Nota::new(Limit::new(u64::MAX, 0, 0), vec![])
    }

    /// A backing UTxO with `amount` lovelace, nothing subbed, no useds.
    fn backing_utxo(amount: u64) -> BackingUtxo {
        BackingUtxo::new(
            BlockHeight(100),
            OutputReference::new([0u8; 32], 0),
            amount,
            0,
            vec![],
        )
    }

    /// A null squash (index=0, amount=0, no excludes) signed by the test key.
    fn null_squash() -> Squash {
        let body = SquashBody::new(0, 0, Indexes::empty()).unwrap();
        Squash::make(&test_sk(), &test_tag(), body)
    }

    /// A locked cheque for `amount` at `index` with the given lock.
    fn make_locked(index: u64, amount: u64, lock: Lock) -> Locked {
        let body = ChequeBody::new(index, amount, Duration::from_secs(99_999_999), lock);
        Locked::make(&test_sk(), &test_tag(), body)
    }

    /// A channel with live backing of `amount` lovelace and a null squash applied.
    fn funded_channel(amount: u64) -> Channel {
        let mut ch = Channel::new(test_keytag(), unlimited_nota());
        ch.backing_mut().push(Chain::new(backing_utxo(amount)));
        ch.apply_squash(null_squash()).unwrap();
        ch
    }

    // -------------------------------------------------------------------------
    // new
    // -------------------------------------------------------------------------

    #[test]
    fn new_has_empty_backing_and_no_receipt() {
        let ch = Channel::new(test_keytag(), unlimited_nota());
        assert!(ch.backing().best_live().is_none());
        assert!(ch.receipt().is_none());
    }

    // -------------------------------------------------------------------------
    // apply_squash
    // -------------------------------------------------------------------------

    #[test]
    fn apply_squash_creates_receipt_on_first_squash() {
        let mut ch = Channel::new(test_keytag(), unlimited_nota());
        assert!(ch.receipt().is_none());
        ch.apply_squash(null_squash()).unwrap();
        assert!(ch.receipt().is_some());
    }

    #[test]
    fn apply_squash_advances_existing_receipt() {
        let mut ch = Channel::new(test_keytag(), unlimited_nota());
        ch.apply_squash(null_squash()).unwrap();
        let first_index = ch.receipt().unwrap().squash.index();

        // A higher squash
        let body2 = SquashBody::new(0, 1, Indexes::empty()).unwrap();
        let squash2 = Squash::make(&test_sk(), &test_tag(), body2);
        ch.apply_squash(squash2).unwrap();
        assert_eq!(ch.receipt().unwrap().squash.index(), 1);
        let _ = first_index; // was 0
    }

    #[test]
    fn apply_squash_rejects_bad_signature() {
        let mut ch = Channel::new(test_keytag(), unlimited_nota());
        // Sign with a different key
        let other_sk = SigningKey::from([99u8; 32]);
        let body = SquashBody::new(0, 0, Indexes::empty()).unwrap();
        let bad_squash = Squash::make(&other_sk, &test_tag(), body);
        assert!(matches!(ch.apply_squash(bad_squash), Err(Error::Input)));
    }

    // -------------------------------------------------------------------------
    // spendable / next_index — error cases
    // -------------------------------------------------------------------------

    #[test]
    fn spendable_no_backing_returns_error() {
        let mut ch = Channel::new(test_keytag(), unlimited_nota());
        ch.apply_squash(null_squash()).unwrap(); // has receipt, no backing
        assert!(matches!(ch.spendable(), Err(Error::Backing)));
    }

    #[test]
    fn spendable_no_receipt_returns_error() {
        let mut ch = Channel::new(test_keytag(), unlimited_nota());
        ch.backing_mut().push(Chain::new(backing_utxo(10_000_000)));
        assert!(matches!(ch.spendable(), Err(Error::Receipt)));
    }

    #[test]
    fn next_index_no_backing_returns_error() {
        let mut ch = Channel::new(test_keytag(), unlimited_nota());
        ch.apply_squash(null_squash()).unwrap();
        assert!(matches!(ch.next_index(), Err(Error::Backing)));
    }

    #[test]
    fn next_index_no_receipt_returns_error() {
        let mut ch = Channel::new(test_keytag(), unlimited_nota());
        ch.backing_mut().push(Chain::new(backing_utxo(10_000_000)));
        assert!(matches!(ch.next_index(), Err(Error::Receipt)));
    }

    // -------------------------------------------------------------------------
    // spendable — happy paths
    // -------------------------------------------------------------------------

    #[test]
    fn spendable_equals_backing_amount_when_nothing_committed() {
        // No cheques, null squash → full UTxO amount is spendable.
        let amount = 50_000_000u64;
        let ch = funded_channel(amount);
        assert_eq!(ch.spendable().unwrap(), amount);
    }

    #[test]
    fn spendable_decreases_after_locked_cheque() {
        let amount = 50_000_000u64;
        let cheque_amount = 10_000_000u64;
        let mut ch = funded_channel(amount);

        let secret = Secret([1u8; 32]);
        let lock = Lock::from(&secret);
        let locked = make_locked(ch.next_index().unwrap(), cheque_amount, lock);
        ch.apply_locked(locked).unwrap();

        assert_eq!(ch.spendable().unwrap(), amount - cheque_amount);
    }

    // -------------------------------------------------------------------------
    // next_index
    // -------------------------------------------------------------------------

    #[test]
    fn next_index_is_one_after_null_squash() {
        let ch = funded_channel(50_000_000);
        // Null squash has index=0, no useds on-chain → next = 0 + 1 = 1
        assert_eq!(ch.next_index().unwrap(), 1);
    }

    #[test]
    fn next_index_advances_after_applying_cheque() {
        let amount = 50_000_000u64;
        let mut ch = funded_channel(amount);

        let idx = ch.next_index().unwrap();
        let locked = make_locked(idx, 1_000_000, Lock([42u8; 32]));
        ch.apply_locked(locked).unwrap();

        assert_eq!(ch.next_index().unwrap(), idx + 1);
    }

    // -------------------------------------------------------------------------
    // apply_locked
    // -------------------------------------------------------------------------

    #[test]
    fn apply_locked_no_backing_returns_error() {
        let mut ch = Channel::new(test_keytag(), unlimited_nota());
        ch.apply_squash(null_squash()).unwrap();
        let locked = make_locked(1, 1_000_000, Lock([0u8; 32]));
        assert!(matches!(ch.apply_locked(locked), Err(Error::Backing)));
    }

    #[test]
    fn apply_locked_no_receipt_returns_error() {
        let mut ch = Channel::new(test_keytag(), unlimited_nota());
        ch.backing_mut().push(Chain::new(backing_utxo(50_000_000)));
        let locked = make_locked(1, 1_000_000, Lock([0u8; 32]));
        // spendable() returns Receipt before we reach receipt.append_locked
        assert!(matches!(ch.apply_locked(locked), Err(Error::Receipt)));
    }

    #[test]
    fn apply_locked_rejects_bad_signature() {
        let mut ch = funded_channel(50_000_000);
        let other_sk = SigningKey::from([99u8; 32]);
        let body = ChequeBody::new(
            1,
            1_000_000,
            Duration::from_secs(99_999_999),
            Lock([0u8; 32]),
        );
        let bad_locked = Locked::make(&other_sk, &test_tag(), body);
        assert!(matches!(ch.apply_locked(bad_locked), Err(Error::Input)));
    }

    #[test]
    fn apply_locked_rejects_overspend() {
        let amount = 10_000_000u64;
        let mut ch = funded_channel(amount);
        let locked = make_locked(1, amount + 1, Lock([0u8; 32]));
        assert!(matches!(ch.apply_locked(locked), Err(Error::Funds)));
    }

    #[test]
    fn apply_locked_appends_cheque_to_receipt() {
        let mut ch = funded_channel(50_000_000);
        assert_eq!(ch.receipt().unwrap().cheques.len(), 0);
        let locked = make_locked(1, 1_000_000, Lock([0u8; 32]));
        ch.apply_locked(locked).unwrap();
        assert_eq!(ch.receipt().unwrap().cheques.len(), 1);
    }

    // -------------------------------------------------------------------------
    // apply_unlock
    // -------------------------------------------------------------------------

    #[test]
    fn apply_unlock_no_receipt_returns_error() {
        let mut ch = Channel::new(test_keytag(), unlimited_nota());
        assert!(matches!(
            ch.apply_unlock(Secret([0u8; 32])),
            Err(Error::Receipt)
        ));
    }

    #[test]
    fn apply_unlock_converts_locked_cheque_to_unlocked() {
        let mut ch = funded_channel(50_000_000);
        let secret = Secret([55u8; 32]);
        let lock = Lock::from(&secret);
        let locked = make_locked(1, 1_000_000, lock);
        ch.apply_locked(locked).unwrap();

        // Before unlock: cheque is Locked
        assert!(ch.receipt().unwrap().cheques[0].as_locked().is_some());
        ch.apply_unlock(secret).unwrap();
        // After unlock: cheque is Unlocked
        assert!(ch.receipt().unwrap().cheques[0].as_unlocked().is_some());
    }

    // -------------------------------------------------------------------------
    // squash_proposal
    // -------------------------------------------------------------------------

    #[test]
    fn squash_proposal_no_receipt_returns_error() {
        let ch = Channel::new(test_keytag(), unlimited_nota());
        assert!(matches!(ch.squash_proposal(), Err(Error::Receipt)));
    }

    #[test]
    fn squash_proposal_reflects_current_receipt() {
        let ch = funded_channel(50_000_000);
        let proposal = ch.squash_proposal().unwrap();
        // The proposal should match the null squash we applied
        assert_eq!(proposal.current.index(), 0);
        assert_eq!(proposal.current.amount(), 0);
    }

    #[test]
    fn squash_proposal_includes_unlocked_cheques() {
        let mut ch = funded_channel(50_000_000);
        let secret = Secret([77u8; 32]);
        let lock = Lock::from(&secret);
        let locked = make_locked(1, 5_000_000, lock);
        ch.apply_locked(locked).unwrap();
        ch.apply_unlock(secret).unwrap();

        let proposal = ch.squash_proposal().unwrap();
        // One unlocked cheque should appear in the proposal
        assert_eq!(proposal.unlockeds.len(), 1);
        assert_eq!(proposal.proposal.amount, 5_000_000); // squash.amount(0) + unlocked(5M)
    }
}

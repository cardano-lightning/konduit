use anyhow::anyhow;
use cardano_tx_builder::VerificationKey;
use cryptoxide::hashing::sha256;

use crate::{
    MAX_TAG_LENGTH, MAX_UNSQUASHED, cheque::Cheque, mixed_cheque::MixedCheque,
    mixed_receipt::MixedReceipt, receipt::Receipt, squash::Squash, unlocked::Unlocked,
};

/// This represents the adaptors state associated to a channel.
/// Each event updates state.
/// Each update is verified as valid of invalid.
pub struct Evidence {
    tag: Vec<u8>,
    verification_key: VerificationKey,
    squash: Squash,
    cheques: Vec<Cheque>,
    unlockeds: Vec<Unlocked>,
    underwritten: u64,
}

impl Evidence {
    pub fn new(
        tag: Vec<u8>,
        verification_key: VerificationKey,
        squash: Squash,
        underwritten: u64,
    ) -> anyhow::Result<Self> {
        if tag.len() > MAX_TAG_LENGTH {
            Err(anyhow!("Tag too long"))?;
        }
        Ok(Evidence {
            tag,
            verification_key,
            squash,
            cheques: vec![],
            unlockeds: vec![],
            underwritten,
        })
    }

    pub fn add_cheque(&mut self, min_timeout: u64, cheque: Cheque) -> anyhow::Result<()> {
        if min_timeout > cheque.cheque_body.timeout {
            Err(anyhow!("Timeout too soon"))?;
        }
        if !cheque.verify(&self.verification_key, self.tag.clone()) {
            Err(anyhow!("Cheque invalid"))?;
        }
        if self
            .squash
            .squash_body
            .index_squashed(cheque.cheque_body.index)
        {
            Err(anyhow!("Index already squashed"))?;
        }
        if self.cheques.len() + self.unlockeds.len() > MAX_UNSQUASHED {
            Err(anyhow!("Too many cheques"))?;
        }
        // FIXME:
        // - Cheque index already in self.cheques
        // - Cheque index already in self.unlockeds
        // - Cheque is underwritten
        self.cheques.push(cheque);
        Ok(())
    }

    pub fn available_amount(&self) -> u64 {
        // Total amount committed.
        // Amount available is then underwitten - commited
        self.underwritten - self.committed()
    }

    pub fn committed(&self) -> u64 {
        // Total amount committed.
        // Amount available is then underwitten - commited
        self.squash.amount()
            + self
                .cheques
                .iter()
                .map(|x| x.cheque_body.amount)
                .sum::<u64>()
            + self
                .unlockeds
                .iter()
                .map(|x| x.cheque_body.amount)
                .sum::<u64>()
    }

    pub fn timeout_cheque(&mut self, _timeout: u64) -> anyhow::Result<Vec<u64>> {
        // Drop cheques that have timed out.
        todo!()
    }

    pub fn add_secret(&mut self, secret: [u8; 32]) -> anyhow::Result<()> {
        let lock = sha256(&secret);
        let (new_unlockeds, cheques): (Vec<Cheque>, Vec<Cheque>) = self
            .cheques
            .clone()
            .into_iter()
            .partition(|x| x.cheque_body.lock == lock);
        if new_unlockeds.is_empty() {
            Err(anyhow!("Secret unlocked no cheques"))?;
        }
        self.cheques = cheques;
        let mut unlockeds = self.unlockeds.clone();
        let mut new = new_unlockeds
            .into_iter()
            .map(|x| Unlocked::new(x.cheque_body, x.signature, secret.clone()))
            .collect::<anyhow::Result<Vec<Unlocked>>>()?;
        unlockeds.append(&mut new);
        self.unlockeds = unlockeds;
        // FIXME:
        // - Cheque index already in self.cheques
        // - Cheque index already in self.unlockeds
        Ok(())
    }

    pub fn receipt(&self) -> anyhow::Result<Receipt> {
        Receipt::new(self.squash.clone(), self.unlockeds.clone())
    }

    pub fn mixed_receipt(&self) -> anyhow::Result<MixedReceipt> {
        MixedReceipt::new(
            self.squash.clone(),
            self.unlockeds
                .iter()
                .map(|x| MixedCheque::from(x.clone()))
                .chain(self.cheques.iter().map(|x| MixedCheque::from(x.clone())))
                .collect(),
        )
    }

    pub fn add_squash(&mut self, _squash: Squash) -> anyhow::Result<()> {
        // FIXME :: Cases to verify
        // - verify squash
        // - squash is strictly ahead of current squash
        // - the value of squashed index
        // - drop unlockeds and maybe cheques that have been squashed
        todo!()
    }
}

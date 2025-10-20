use anyhow::anyhow;
use cardano_tx_builder::VerificationKey;
use cryptoxide::hashing::sha256;

use crate::{
    MAX_TAG_LENGTH, MAX_UNLOCKEDS_LENGTH, cheque::Cheque, squash::Squash, unlocked::Unlocked,
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
}

impl Evidence {
    pub fn new(
        tag: Vec<u8>,
        verification_key: VerificationKey,
        squash: Squash,
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
        if self.cheques.len() + self.unlockeds.len() > MAX_UNLOCKEDS_LENGTH {
            Err(anyhow!("Too many cheques"))?;
        }
        // FIXME:
        // - Cheque index already in self.cheques
        // - Cheque index already in self.unlockeds
        self.cheques.push(cheque);
        Ok(())
    }

    pub fn timeout_cheque(&mut self) {
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

    pub fn squash_request(&self) -> (Squash, Vec<Unlocked>) {
        (self.squash.clone(), self.unlockeds.clone())
    }

    pub fn add_squash(&mut self, squash: Squash) -> anyhow::Result<()> {
        // FIXME :: Cases to verify
        // - verify squash
        // - squash is strictly ahead of current squash
        // - the value of squashed index
        // - drop unlockeds and maybe cheques that have been squashed
        todo!()
    }
}

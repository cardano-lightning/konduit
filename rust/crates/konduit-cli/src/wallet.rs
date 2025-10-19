use anyhow::anyhow;
use std::collections::HashMap;

use cardano_tx_builder::{Credential, Hash};

use rand::{TryRngCore, rngs::OsRng};

const PREFIX: &str = "wallet_";

/// Label for wallet key
const KEY: &str = "key";

pub struct Wallet {
    pub signing_key: [u8;32],
}

// FIXME :: move upstream 
fn blake2b_224(x : [u8; 32]) -> Hash<28> {
    todo!()
}

impl Wallet {
    pub fn verification_key(&self) -> [u8;32] { 
        todo!()
    }

    pub fn credential(&self) -> Credential {
        Credential::from_key(blake2b_224(self.verification_key()))
    }

    pub fn from_env(env: &HashMap<String, String>) -> anyhow::Result<Wallet> {
        let wallet_env: HashMap<String, String> = env
            .iter()
            .filter_map(|(k, v)| k.strip_prefix(PREFIX).map(|k| (k.to_string(), v.clone())))
            .collect();
        let raw = wallet_env.get(KEY).ok_or(anyhow!("wallet `{KEY}` not found"))?;
        let signing_key = parse_key(raw)?;
        Ok(Wallet { signing_key })
    }

    // pub fn sign(&self, tx: &mut Tx) {
    //     let mut msg = Vec::new();
    //     encode(&tx.transaction_body, &mut msg).unwrap();
    //     let tx_hash = blake2b_256(&msg);
    //     let sig = self.sign_hash(&tx_hash);
    //     tx.transaction_witness_set.vkeywitness = non_empty_set(vec![VKeyWitness {
    //         vkey: self.vkey().to_vec().into(),
    //         signature: sig.as_ref().to_vec().into(),
    //     }])
    // }
}

pub fn generate_key() -> [u8;32] {
    let mut key = [0u8; 32];
    OsRng.try_fill_bytes(&mut key).unwrap();
    key
}

fn parse_key(raw: &str) -> anyhow::Result<[u8; 32]> {
    // FIXME :: Not tested
    if raw.len() == 64 {
        // Assume hex
        <[u8;32]>::try_from(hex::decode(raw)?).map_err(|_| anyhow!("Bad length"))
    } else if raw.len() == 70 {
        // Assume Bech
        // <[u8;32]>::try_from(bech32::decode(raw).unwrap().1).expect("Bad length")
        panic!("Not supported")
    } else {
        panic!("Not supported")
    }
}

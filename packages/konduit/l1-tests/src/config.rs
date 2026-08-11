use cardano_connector_direct::Blockfrost;
use cardano_sdk::address::kind::Shelley;
use cardano_sdk::{Network, NetworkId};
use konduit_data::{Constants, Duration, SigningKey, Tag, VerifyingKey};
use serde::{Deserialize, Serialize};

use cryptoxide::blake2b::Blake2b;
use cryptoxide::digest::Digest;

fn hash_me(input: &[u8]) -> [u8; 32] {
    let mut hasher = Blake2b::new(32); // 32 = output size in bytes
    hasher.input(input);
    let mut out = [0u8; 32];
    hasher.result(&mut out);
    out
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptorConfig {
    pub key: SigningKey,
    pub close_period: Duration,
}

impl AdaptorConfig {
    pub fn constants(&self) -> (VerifyingKey, Duration) {
        (self.key.verifying_key(), self.close_period)
    }

    pub fn cardano_signing_key(&self) -> cardano_sdk::SigningKey {
        cardano_sdk::SigningKey::from(<[u8; 32]>::from(self.key.clone()))
    }
}

impl Default for AdaptorConfig {
    fn default() -> Self {
        Self {
            key: hash_me("adaptor".as_bytes()).into(),
            close_period: Duration::from_secs(3600),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountConfig {
    pub key: SigningKey,
    pub tag: Tag,
}

impl AccountConfig {
    pub fn verifying_key(&self) -> VerifyingKey {
        self.key.verifying_key()
    }

    pub fn cardano_signing_key(&self) -> cardano_sdk::SigningKey {
        cardano_sdk::SigningKey::from(<[u8; 32]>::from(self.key.clone()))
    }

    pub fn constants(&self, adaptor_key: VerifyingKey, close_period: Duration) -> Constants {
        Constants {
            tag: self.tag.clone(),
            add_vkey: self.key.verifying_key(),
            sub_vkey: adaptor_key,
            close_period,
        }
    }
}

impl Default for AccountConfig {
    fn default() -> Self {
        Self {
            key: hash_me("account".as_bytes()).into(),
            tag: Tag::from(vec![]),
        }
    }
}

impl AccountConfig {
    pub fn new(seed: u8) -> Self {
        let key = hash_me(&format!("account {}", seed).into_bytes()).into();
        Self {
            key,
            tag: Tag::generate((seed % 32) as usize),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CardanoConfig {
    Blockfrost { key: String, network: Network },
}

impl CardanoConfig {
    pub fn build(&self) -> Blockfrost {
        match self {
            CardanoConfig::Blockfrost { key, .. } => Blockfrost::new(key.clone()),
        }
    }
}

impl Default for CardanoConfig {
    fn default() -> Self {
        Self::Blockfrost {
            key: "mainnetxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx".to_string(),
            network: Network::Mainnet,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub wallet: WalletConfig,
    pub cardano: CardanoConfig,
    pub adaptor: AdaptorConfig,
    pub accounts: Vec<AccountConfig>,
    pub txs: Vec<TxConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TxConfig {
    Consumer {
        open: Option<u8>,
        add: Option<u8>,
        close: Option<u8>,
        any: Option<u8>,
    },
    Adaptor {
        claim: Option<u8>,
        any: Option<u8>,
    },
}

impl Default for Config {
    fn default() -> Self {
        let accounts = (0..3).map(|i| AccountConfig::new(i)).collect();
        let txs = vec![
            TxConfig::Consumer {
                open: Some(1),
                add: None,
                close: None,
                any: None,
            },
            TxConfig::Adaptor {
                claim: Some(1),
                any: None,
            },
            TxConfig::Consumer {
                open: None,
                add: None,
                close: Some(1),
                any: None,
            },
            TxConfig::Adaptor {
                claim: Some(1),
                any: None,
            },
            TxConfig::Consumer {
                open: None,
                add: None,
                close: None,
                any: Some(1),
            },
        ];
        Self {
            wallet: Default::default(),
            cardano: Default::default(),
            adaptor: Default::default(),
            accounts,
            txs,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    key: SigningKey,
}

impl Default for WalletConfig {
    fn default() -> Self {
        Self {
            key: hash_me("wallet".as_bytes()).into(),
        }
    }
}

impl WalletConfig {
    pub fn cardano_signing_key(&self) -> cardano_sdk::SigningKey {
        cardano_sdk::SigningKey::from(<[u8; 32]>::from(self.key.clone()))
    }

    pub fn verification_key(&self) -> cardano_sdk::VerificationKey {
        self.cardano_signing_key().to_verification_key()
    }

    pub fn credential(&self) -> cardano_sdk::Credential {
        cardano_sdk::Credential::from_key(cardano_sdk::Hash::<28>::new(self.verification_key()))
    }

    pub fn address(&self, network_id: NetworkId) -> cardano_sdk::Address<Shelley> {
        cardano_sdk::Address::new(network_id, self.credential())
    }
}

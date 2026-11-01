use konduit_data::VerifyingKey;
use serde::{Deserialize, Serialize};

use crate::{channels, commits, signer};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub signer: signer::Config,
    /// Who this rail can reach, and the channels held with them.
    pub outbound: channels::Config,
    /// Where the rail keeps what it has committed, so a restart between
    /// a commit and its reveal does not lose the secret.
    pub known_commits: commits::Config,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("expect peer: {0:?}")]
    ExpectPeer(VerifyingKey),
}

impl Config {
    /// A channel is only usable if its key is one the rail knows how to
    /// reach, so a config naming a channel with an unknown key is
    /// refused here rather than silently dropped later.
    pub fn verify(&self) -> Result<(), Error> {
        match self.outbound.unknown_keys().into_iter().next() {
            Some(key) => Err(Error::ExpectPeer(key)),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel;
    use konduit_data::{SigningKey, Tag};

    fn key(seed: u8) -> VerifyingKey {
        SigningKey::from([seed; 32]).verifying_key()
    }

    fn known(seed: u8) -> channel::KnownKey {
        channel::KnownKey {
            key: key(seed),
            base_url: "http://localhost:1".to_string(),
            media_type: Default::default(),
        }
    }

    fn entry(seed: u8) -> channel::Entry {
        channel::Entry {
            key: key(seed),
            tag: Tag::from(b"tag".to_vec()),
        }
    }

    #[test]
    fn a_channel_with_a_known_key_verifies() {
        let config = Config {
            outbound: channels::Config {
                known_keys: vec![known(1)],
                channels: vec![entry(1)],
            },
            ..Default::default()
        };
        assert!(config.verify().is_ok());
    }

    #[test]
    fn a_channel_with_an_unknown_key_is_refused() {
        let config = Config {
            outbound: channels::Config {
                known_keys: vec![known(1)],
                channels: vec![entry(2)],
            },
            ..Default::default()
        };
        match config.verify() {
            Err(Error::ExpectPeer(unknown)) => assert_eq!(unknown, key(2)),
            other => panic!("expected ExpectPeer, got {other:?}"),
        }
    }

    /// A key the rail can reach but holds no channel with is not an
    /// error: it is a peer it could pay once a channel exists.
    #[test]
    fn a_known_key_without_a_channel_verifies() {
        let config = Config {
            outbound: channels::Config {
                known_keys: vec![known(1), known(2)],
                channels: vec![entry(1)],
            },
            ..Default::default()
        };
        assert!(config.verify().is_ok());
    }
}

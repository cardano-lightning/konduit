//! Local to this crate (not `konduit_data`): prior squash/cheque
//! settlement records, looked up by `Keytag` (vkey + tag bytes) rather
//! than `Input`, so a receipt survives a channel being stepped/recreated.
//!
//! ASSUMPTION: `Squash`/`Cheque` already implement `serde` + `minicbor`
//! (matches the rest of `konduit_data`) — worth confirming.
//! ASSUMPTION: receipts file is JSON (needs the `serde_json` dep) —
//! swap `load_receipts`'s body if the real format differs.

use std::{collections::BTreeMap, path::Path};

use anyhow::{Context, Result};
use cardano_sdk::Hash;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use konduit_data::{Cheque, Indexes, SigningKey, Squash, SquashBody, Tag, VerifyingKey};

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Receipt {
    #[n(0)]
    pub squash: Squash,
    #[n(1)]
    pub cheques: Vec<Cheque>,
}

/// Verifying key (32 bytes) followed by tag bytes, verbatim.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Keytag(Vec<u8>);

impl Keytag {
    pub fn new(vkey: &VerifyingKey, tag: &[u8]) -> Self {
        let mut bytes = vkey.as_ref().to_vec();
        bytes.extend_from_slice(tag);
        Self(bytes)
    }

    pub fn vkey(&self) -> Option<VerifyingKey> {
        <[u8; 32]>::try_from(self.0.get(..32)?)
            .ok()
            .map(VerifyingKey::from)
    }

    pub fn tag(&self) -> &[u8] {
        self.0.get(32..).unwrap_or(&[])
    }
}

/// Hex string in human formats — permissive: tolerates a `0x` prefix and
/// surrounding whitespace.
impl Serialize for Keytag {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&hex::encode(&self.0))
    }
}

impl<'de> Deserialize<'de> for Keytag {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        let bytes =
            hex::decode(raw.trim().trim_start_matches("0x")).map_err(serde::de::Error::custom)?;
        Ok(Self(bytes))
    }
}

impl<C> Encode<C> for Keytag {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> std::result::Result<(), minicbor::encode::Error<W::Error>> {
        e.bytes(&self.0)?;
        Ok(())
    }
}

impl<'b, C> Decode<'b, C> for Keytag {
    fn decode(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> std::result::Result<Self, minicbor::decode::Error> {
        Ok(Self(d.bytes()?.to_vec()))
    }
}

pub type Receipts = BTreeMap<Keytag, Receipt>;

/// Permissive: a missing file is an empty `Receipts`, not an error;
/// `Keytag`'s `Deserialize` tolerates `0x`-prefixed/whitespace-padded hex.
pub fn load_receipts(path: &Path) -> Result<Receipts> {
    if !path.exists() {
        return Ok(receipts_example());
    }
    let raw =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

/// Panic on fail
pub fn decode_u8_32(s: &str) -> [u8; 32] {
    <[u8; 32]>::try_from(hex::decode(s).unwrap()).unwrap()
}

/// FIXME! :: There is no ergonomic way to generate receipts.
/// The best i think we can hope for is integration with server
/// or synthetic generation.
pub fn receipts_example() -> Receipts {
    let alice_seed = "alice";
    let alice_key = SigningKey::from(<[u8; 32]>::from(Hash::<32>::new(alice_seed)));
    let alice_tag = Tag::from(hex::decode("deadbeef0a").unwrap());
    let alice_tag1 = Tag::from(hex::decode("0123456789").unwrap());

    let charlie_seed = "charlie";
    let charlie_key = SigningKey::from(<[u8; 32]>::from(Hash::<32>::new(charlie_seed)));
    let charlie_tag0 = Tag::from(hex::decode("00").unwrap());
    let charlie_tag1 = Tag::from(hex::decode("01").unwrap());

    let alice_squash = Squash::make(
        &alice_key,
        &alice_tag,
        SquashBody::new(1234569, 2, Indexes::default()).unwrap(),
    );
    let alice_receipt = Receipt {
        squash: alice_squash.into_unverified(),
        cheques: Vec::new(),
    };

    let alice_squash1 = Squash::make(
        &alice_key,
        &alice_tag1,
        SquashBody::new(1234567, 1, Indexes::default()).unwrap(),
    );
    let alice_receipt1 = Receipt {
        squash: alice_squash1.into_unverified(),
        cheques: Vec::new(),
    };

    let charlie_squash0 = Squash::make(
        &charlie_key,
        &charlie_tag0,
        SquashBody::new(2345678, 1, Indexes::default()).unwrap(),
    );
    let charlie_receipt0 = Receipt {
        squash: charlie_squash0.into_unverified(),
        cheques: Vec::new(),
    };

    let charlie_squash1 = Squash::make(
        &charlie_key,
        &charlie_tag1,
        SquashBody::new(3456789, 2, Indexes::default()).unwrap(),
    );
    let charlie_receipt1 = Receipt {
        squash: charlie_squash1.into_unverified(),
        cheques: Vec::new(),
    };

    let mut receipts = BTreeMap::new();
    receipts.insert(
        Keytag::new(&alice_key.verifying_key(), &alice_tag),
        alice_receipt,
    );
    receipts.insert(
        Keytag::new(&alice_key.verifying_key(), &alice_tag1),
        alice_receipt1,
    );
    receipts.insert(
        Keytag::new(&charlie_key.verifying_key(), &charlie_tag0),
        charlie_receipt0,
    );
    receipts.insert(
        Keytag::new(&charlie_key.verifying_key(), &charlie_tag1),
        charlie_receipt1,
    );
    receipts
}
